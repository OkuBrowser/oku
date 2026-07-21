use super::*;
use crate::{config::OkuFsConfig, error::OkuFsError, fs::util::path_to_entry_key};
use bytes::Bytes;
use iroh::protocol::ProtocolHandler;
#[cfg(feature = "persistent")]
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::{api::Store, store::mem::MemStore, BlobsProtocol};
use iroh_docs::{Author, NamespaceId};
use iroh_gossip::Gossip;
use log::{error, info, trace};
use miette::IntoDiagnostic;
use moka::{
    future::{Cache, FutureExt},
    notification::{ListenerFuture, RemovalCause},
    policy::EvictionPolicy,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashSet, io::SeekFrom, path::PathBuf, time::Duration};
use tempfile::NamedTempFile;
#[cfg(feature = "fuse")]
use tokio::runtime::Handle;
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    sync::watch::{self},
};

impl OkuFs {
    /// Obtain the private key of the node's authorship credentials.
    ///
    /// # Return
    ///
    /// The private key of the node's authorship credentials.
    pub async fn get_author(&self) -> anyhow::Result<Author> {
        let default_author_id = self.default_author().await;

        self.docs
            .author_export(default_author_id)
            .await
            .ok()
            .flatten()
            .ok_or(anyhow::anyhow!(
                "Missing private key for default author ({}).",
                crate::fs::util::fmt_short(default_author_id)
            ))
    }

    /// Starts an instance of an Oku file system.
    /// In the background, an Iroh node is started if none is running, or is connected to if one is already running.
    ///
    /// # Arguments
    ///
    /// * `handle` - If compiling with the `fuse` feature, a Tokio runtime handle is required.
    ///
    /// * `persistent` - If compiling with the `persistent` feature, a boolean indicating whether to persist replicas to the filesystem or to keep them in memory.
    ///
    /// # Returns
    ///
    /// A running instance of an Oku file system.
    pub async fn start(
        #[cfg(feature = "fuse")] handle: Option<&Handle>,
        #[cfg(feature = "persistent")] persistent: bool,
    ) -> anyhow::Result<Self> {
        let mdns = iroh_mdns_address_lookup::MdnsAddressLookup::builder();
        let dht_discovery = iroh_mainline_address_lookup::DhtAddressLookup::builder();

        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .address_lookup(mdns)
            .address_lookup(dht_discovery)
            .bind()
            .await?;

        cfg_if::cfg_if!(
            if #[cfg(any(feature = "persistent"))] {
                let store: Store = match persistent {
                    true => Store::from(FsStore::load(NODE_PATH.clone()).await?),
                    false => MemStore::new().into()
                };
            } else {
                let store: Store = MemStore::new().into();
            }
        );

        let blobs = BlobsProtocol::new(&store, None);

        let gossip = Gossip::builder().spawn(endpoint.clone());
        cfg_if::cfg_if!(
            if #[cfg(any(feature = "persistent"))] {
                let docs = match persistent {
                    true => iroh_docs::protocol::Docs::persistent(NODE_PATH.clone())
                        .spawn(endpoint.clone(), store, gossip.clone())
                        .await?,
                    false => iroh_docs::protocol::Docs::memory()
                        .spawn(endpoint.clone(), store, gossip.clone())
                        .await?
                };
            } else {
                let docs = iroh_docs::protocol::Docs::memory()
                    .spawn(endpoint.clone(), store, gossip.clone())
                    .await?;
            }
        );

        let router = iroh::protocol::Router::builder(endpoint.clone())
            .accept(iroh_blobs::ALPN, blobs.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .accept(iroh_docs::ALPN, docs.clone())
            .spawn();
        let default_author = docs.author_default().await.unwrap_or_default();
        info!(
            "Default author ID is {} … ",
            crate::fs::util::fmt_short(default_author)
        );

        let (replica_sender, _replica_receiver) = watch::channel(());
        let (okunet_fetch_sender, _okunet_fetch_receiver) = watch::channel(false);

        let docs_client = docs.clone();
        let blobs_client = blobs.clone();
        // let sync_eviction_listener = |key, _value, cause| {
        //     println!("Evicted key {key:?}. Cause: {cause:?}");
        // };
        let eviction_listener = move |k: Arc<(NamespaceId, PathBuf)>,
                                      v: Arc<Mutex<NamedTempFile>>,
                                      cause: RemovalCause|
              -> ListenerFuture {
            // The cached file is past its TTL, so we should commit it to the replica.
            let (namespace_id, path) = (k.0, k.1.clone());
            let namespace_id_str = crate::fs::util::fmt(namespace_id);
            trace!("Starting to commit cached file (replica: {namespace_id_str}, path: {path:?}), with cause: {cause:?}");
            let docs = docs_client.clone();
            let blobs = blobs_client.clone();
            let v = v.clone();
            async move {
                let inner = async |(namespace_id, path): (NamespaceId, PathBuf), v: Arc<Mutex<NamedTempFile>>, cause: RemovalCause| -> miette::Result<iroh_blobs::Hash> {
                    let namespace_id_str = crate::fs::util::fmt(namespace_id);
                    trace!("Comitting cached file (replica: {namespace_id_str}, path: {path:?}), with cause: {cause:?}");

                    let file_key = path_to_entry_key(&path);
                    let v = Arc::clone(&v);
                    let tempfile = v.try_lock().map_err(|e| miette::miette!("{e}"))?;
                    let document = docs
                        .open(namespace_id)
                        .await
                        .map_err(|e| {
                            error!("{}", e);
                            OkuFsError::CannotOpenReplica
                        })?
                        .ok_or(OkuFsError::FsEntryNotFound)?;
                    let query = iroh_docs::store::Query::single_latest_per_key()
                        .key_exact(&file_key)
                        .build();
                    let entry = document
                        .get_one(query)
                        .await
                        .map_err(|e| {
                            error!("{}", e);
                            OkuFsError::CannotReadFile
                        })?
                        .ok_or(OkuFsError::FsEntryNotFound)?;
                    let _entries_deleted = document
                        .del(entry.author(), entry.key().to_vec())
                        .await
                        .map_err(|e| {
                            error!("{}", e);
                            OkuFsError::CannotDeleteFile
                        })?;
            let import_file_outcome = document.import_file(blobs.store(), default_author, file_key, tempfile.path(), iroh_blobs::api::blobs::ImportMode::TryReference).await.map_err(|e| miette::miette!("{e}"))?.await.map_err(|e| miette::miette!("{e}"))?;

            let bytes_written = import_file_outcome.size;
            let data_len = tempfile.as_file().metadata().into_diagnostic()?.len();
            if bytes_written != data_len {
                error!("[File cache commit closure] likely data loss when writing data to file (bytes written: {bytes_written}, bytes intended: {data_len})")
            }

            info!("Committed cached file (replica: {namespace_id_str}, path: {path:?}): {import_file_outcome:?}");

            Ok(import_file_outcome.hash)
                };
                if let Err(e) = inner((namespace_id, path.clone()), v, cause).await {
                    let namespace_id_str = crate::fs::util::fmt(namespace_id);
                    error!("Unable to commit cached file (replica: {namespace_id_str}, path: {path:?}): {e}");
                }
            }
            .boxed()
        };

        let file_cache: Cache<(NamespaceId, PathBuf), Arc<Mutex<NamedTempFile>>> = Cache::builder()
            .time_to_live(Duration::from_secs(5))
            .eviction_policy(EvictionPolicy::lru())
            .async_eviction_listener(eviction_listener)
            // .eviction_listener(sync_eviction_listener)
            .build();

        let oku_core = Self {
            endpoint,
            blobs,
            docs,
            router,
            replica_sender,
            okunet_fetch_sender,
            #[cfg(feature = "fuse")]
            fuse_handler: DebugIgnore::from(Arc::new(DefaultFuseHandler::new())),
            #[cfg(feature = "fuse")]
            handle: handle.cloned(),
            dht: mainline::Dht::server()?.as_async(),
            file_cache,
        };
        let oku_core_clone = oku_core.clone();
        cfg_if::cfg_if!(
            if #[cfg(any(feature = "persistent"))] {
                let config = match persistent {
                    true => OkuFsConfig::load_or_create_config().unwrap_or_default(),
                    false => OkuFsConfig::default()
                };
            } else {
                let config = OkuFsConfig::default();
            }
        );
        let republish_delay = config.get_republish_delay();
        let initial_publish_delay = config.get_initial_publish_delay();

        tokio::spawn(async move {
            tokio::time::sleep(initial_publish_delay).await;
            loop {
                match oku_core_clone.announce_replicas().await {
                    Ok(_) => info!("Announced all replicas … "),
                    Err(e) => error!("{}", e),
                }
                match oku_core_clone.refresh_users().await {
                    Ok(_) => info!("Refreshed OkuNet database … "),
                    Err(e) => error!("{}", e),
                }
                tokio::time::sleep(republish_delay).await;
            }
        });
        Ok(oku_core.clone())
    }

    /// Shuts down the Oku file system.
    pub async fn shutdown(self) {
        info!("Node shutting down … ");
        self.endpoint.close().await;
        if let Err(e) = self.router.shutdown().await {
            error!("{e}");
        }
        self.docs.shutdown().await;
        self.blobs.shutdown().await;
        if let Err(e) = self.blobs.store().shutdown().await {
            error!("{e}");
        }
    }

    /// Retrieve the content of a document entry.
    ///
    /// # Arguments
    ///
    /// * `entry` - An entry in an Iroh document.
    ///
    /// * `seek` - Optional direction of where in the file to read from.
    ///
    /// * `len` - Optional number of bytes to read.
    ///
    /// # Returns
    ///
    /// The content of the entry, as raw bytes.
    pub async fn content_bytes(
        &self,
        entry: &iroh_docs::Entry,
        seek: &Option<SeekFrom>,
        len: &Option<u64>,
    ) -> anyhow::Result<Bytes> {
        self.content_bytes_by_hash(&entry.content_hash(), seek, len)
            .await
    }

    /// Retrieve the content of a document entry by its hash.
    ///
    /// # Arguments
    ///
    /// * `hash` - The content hash of an Iroh document.
    ///
    /// * `seek` - Optional direction of where in the content to read from.
    ///
    /// * `len` - Optional number of bytes to read.
    ///
    /// # Returns
    ///
    /// The content of the entry, as raw bytes.
    pub async fn content_bytes_by_hash(
        &self,
        hash: &iroh_blobs::Hash,
        seek: &Option<SeekFrom>,
        len: &Option<u64>,
    ) -> anyhow::Result<Bytes> {
        match (seek, len) {
            (None, None) => self
                .blobs
                .blobs()
                .get_bytes(*hash)
                .await
                .map_err(|e| anyhow::anyhow!(e)),
            _ => {
                let mut reader = self.blobs.blobs().reader(*hash);
                if let Some(seek) = seek {
                    reader.seek(*seek).await?;
                }

                match len {
                    Some(len) => {
                        // This was really annoying.
                        // `AsyncReadExt` doesn't guarantee that `read` (or equivalents) will actually fill up the buffer.
                        // This includes `read_buf`, for some reason!
                        // So, we need to make a new reader (`.take(u64)`) that prematurely hits EOF,
                        // and then use `read_to_end`, which repeatedly `read`s until EOF, to actually get
                        // the number of bytes we requested, and not less.
                        let mut buffer = Vec::new();
                        let mut handle = reader.take(*len);
                        let read_bytes = handle.read_to_end(&mut buffer).await?;
                        buffer.truncate(read_bytes); // Trying to be safe, so we don't somehow return too many bytes.

                        // Check for data loss
                        let buffer_size = buffer.len();
                        let mut length_set = HashSet::new();
                        length_set.insert(*len);
                        length_set.insert(read_bytes as u64);
                        length_set.insert(buffer_size as u64);
                        if length_set.len() != 1 {
                            error!("[content_bytes_by_hash] likely data loss when reading file (requested length: {len}, bytes read {read_bytes}, buffer size {buffer_size})");
                        }

                        Ok(buffer.into())
                    }
                    None => {
                        let mut buffer = Vec::new();
                        reader.read_to_end(&mut buffer).await?;
                        Ok(buffer.into())
                    }
                }
            }
        }
    }

    /// Determines the oldest timestamp of a file entry in any replica stored locally.
    ///
    /// # Returns
    ///
    /// The oldest timestamp in any local replica, in microseconds from the Unix epoch.
    pub async fn get_oldest_timestamp(&self) -> miette::Result<u64> {
        let replicas = self.list_replicas().await?;
        let mut timestamps: Vec<u64> = Vec::new();
        for (replica, _capability_kind, _is_home_replica) in replicas {
            timestamps.push(
                self.get_oldest_timestamp_in_folder(&replica, &PathBuf::from("/"))
                    .await?,
            );
        }
        Ok(*timestamps.par_iter().min().unwrap_or(&u64::MIN))
    }

    /// Determines the latest timestamp of a file entry in any replica stored locally.
    ///
    /// # Returns
    ///
    /// The latest timestamp in any local replica, in microseconds from the Unix epoch.
    pub async fn get_newest_timestamp(&self) -> miette::Result<u64> {
        let replicas = self.list_replicas().await?;
        let mut timestamps: Vec<u64> = Vec::new();
        for (replica, _capability_kind, _is_home_replica) in replicas {
            timestamps.push(
                self.get_newest_timestamp_in_folder(&replica, &PathBuf::from("/"))
                    .await?,
            );
        }
        Ok(*timestamps.par_iter().max().unwrap_or(&u64::MIN))
    }

    /// Determines the size of the file system.
    ///
    /// # Returns
    ///
    /// The total size, in bytes, of the files in every replica stored locally.
    pub async fn get_size(&self) -> miette::Result<u64> {
        let replicas = self.list_replicas().await?;
        let mut size = 0;
        for (replica, _capability_kind, _is_home_replica) in replicas {
            size += self.get_folder_size(&replica, &PathBuf::from("/")).await?;
        }
        Ok(size)
    }

    #[cfg(feature = "fuse")]
    /// Mount the file system.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file system mount point.
    ///
    /// # Returns
    ///
    /// A handle referencing the mounted file system; joining or dropping the handle will unmount the file system and shutdown the node.
    pub fn mount(
        &self,
        path: PathBuf,
    ) -> miette::Result<easy_fuser::session::FuseSession<PathBuf>> {
        let handle = self
            .handle
            .clone()
            .ok_or(miette::miette!("Tokio handle for FUSE is missing."))?;
        let self_clone = self.clone();
        futures::executor::block_on(async {
            handle
                .spawn_blocking(|| {
                    easy_fuser::fuse_async::mounting::spawn_mount(
                        self_clone,
                        path,
                        &[
                            easy_fuser::fuse_async::prelude::MountOption::FSName("Oku".into()),
                            easy_fuser::fuse_async::prelude::MountOption::RW,
                            easy_fuser::fuse_async::prelude::MountOption::Exec,
                            easy_fuser::fuse_async::prelude::MountOption::Async,
                        ],
                        None,
                    )
                    .into_diagnostic()
                })
                .await
                .expect("Task spawned in Tokio executor panicked")
        })
    }
}
