use super::*;
use crate::config::OkuFsConfig;
use bytes::Bytes;
use iroh::protocol::ProtocolHandler;
#[cfg(feature = "persistent")]
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::{api::Store, store::mem::MemStore, BlobsProtocol};
use iroh_docs::Author;
use iroh_gossip::Gossip;
use log::{error, info};
#[cfg(feature = "fuse")]
use miette::IntoDiagnostic;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;
#[cfg(feature = "fuse")]
use tokio::runtime::Handle;
use tokio::sync::watch::{self};

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
        let mdns = iroh::address_lookup::mdns::MdnsAddressLookup::builder();
        let dht_discovery = iroh::address_lookup::DhtAddressLookup::builder();

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
        info!(
            "Default author ID is {} … ",
            crate::fs::util::fmt_short(docs.author_default().await.unwrap_or_default())
        );

        let (replica_sender, _replica_receiver) = watch::channel(());
        let (okunet_fetch_sender, _okunet_fetch_receiver) = watch::channel(false);

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
    /// # Returns
    ///
    /// The content of the entry, as raw bytes.
    pub async fn content_bytes(&self, entry: &iroh_docs::Entry) -> anyhow::Result<Bytes> {
        self.content_bytes_by_hash(&entry.content_hash()).await
    }

    /// Retrieve the content of a document entry by its hash.
    ///
    /// # Arguments
    ///
    /// * `hash` - The content hash of an Iroh document.
    ///
    /// # Returns
    ///
    /// The content of the entry, as raw bytes.
    pub async fn content_bytes_by_hash(&self, hash: &iroh_blobs::Hash) -> anyhow::Result<Bytes> {
        self.blobs
            .blobs()
            .get_bytes(*hash)
            .await
            .map_err(|e| anyhow::anyhow!(e))
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
    pub fn mount(&self, path: PathBuf) -> miette::Result<fuser::BackgroundSession> {
        easy_fuser::spawn_mount(self.clone(), path, &[], 4).into_diagnostic()
    }
}
