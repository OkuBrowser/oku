//! Handlers and actors to for live syncing replicas.
//!
//! [`crate::Replica`] is also called documents here.

use std::sync::{Arc, RwLock};

use anyhow::{bail, Result};
use iroh::{Endpoint, EndpointAddr, PublicKey};
use iroh_blobs::{
    api::{blobs::BlobStatus, downloader::Downloader, Store},
    store::{ProtectCb, ProtectOutcome},
    Hash,
};
use iroh_gossip::net::Gossip;
use n0_future::{task::AbortOnDropHandle, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, error_span, Instrument};

use self::live::{LiveActor, ToLiveActor};
pub use self::{
    live::SyncEvent,
    state::{Origin, SyncReason},
};
use crate::{
    actor::SyncHandle, metrics::Metrics, Author, AuthorId, ContentStatus, ContentStatusCallback,
    Entry, NamespaceId,
};

mod gossip;
mod live;
mod state;

/// Capacity of the channel for the [`ToLiveActor`] messages.
const ACTOR_CHANNEL_CAP: usize = 64;
/// Capacity for the channels for [`Engine::subscribe`].
const SUBSCRIBE_CHANNEL_CAP: usize = 256;

/// The sync engine coordinates actors that manage open documents, set-reconciliation syncs with
/// peers and a gossip swarm for each syncing document.
#[derive(derive_more::Debug)]
pub struct Engine {
    /// [`Endpoint`] used by the engine.
    pub endpoint: Endpoint,
    /// Handle to the actor thread.
    pub sync: SyncHandle,
    /// The persistent default author for this engine.
    pub default_author: DefaultAuthor,
    to_live_actor: mpsc::Sender<ToLiveActor>,
    #[allow(dead_code)]
    actor_handle: AbortOnDropHandle<()>,
    #[debug("ContentStatusCallback")]
    content_status_cb: ContentStatusCallback,
    blob_store: iroh_blobs::api::Store,
    _gc_protect_task: AbortOnDropHandle<()>,
}

impl Engine {
    /// Start the sync engine.
    ///
    /// This will spawn two tokio tasks for the live sync coordination and gossip actors, and a
    /// thread for the actor interacting with doc storage.
    pub async fn spawn(
        endpoint: Endpoint,
        gossip: Gossip,
        replica_store: crate::store::Store,
        bao_store: iroh_blobs::api::Store,
        downloader: Downloader,
        default_author_storage: DefaultAuthorStorage,
        protect_cb: Option<ProtectCallbackHandler>,
    ) -> anyhow::Result<Self> {
        let (live_actor_tx, to_live_actor_recv) = mpsc::channel(ACTOR_CHANNEL_CAP);
        let me = endpoint.id().fmt_short().to_string();

        let content_status_cb: ContentStatusCallback = {
            let blobs = bao_store.blobs().clone();
            Arc::new(move |hash: iroh_blobs::Hash| {
                let blobs = blobs.clone();
                Box::pin(async move {
                    let blob_status = blobs.status(hash).await;
                    entry_to_content_status(blob_status)
                })
            })
        };
        let sync = SyncHandle::spawn(replica_store, Some(content_status_cb.clone()), me.clone());

        let sync2 = sync.clone();
        let gc_protect_task = AbortOnDropHandle::new(n0_future::task::spawn(async move {
            let Some(mut protect_handler) = protect_cb else {
                return;
            };
            while let Some(reply_tx) = protect_handler.0.recv().await {
                let (tx, rx) = mpsc::channel(64);
                if let Err(_err) = reply_tx.send(rx) {
                    continue;
                }
                let hashes = match sync2.content_hashes().await {
                    Ok(hashes) => hashes,
                    Err(err) => {
                        debug!("protect task: getting content hashes failed with {err:#}");
                        if let Err(_err) = tx.send(Err(err)).await {
                            debug!("protect task: failed to forward error");
                        }
                        continue;
                    }
                };
                for hash in hashes {
                    if let Err(_err) = tx.send(hash).await {
                        debug!("protect task: failed to forward hash");
                        break;
                    }
                }
            }
        }));

        let actor = LiveActor::new(
            sync.clone(),
            endpoint.clone(),
            gossip.clone(),
            bao_store.clone(),
            downloader,
            to_live_actor_recv,
            live_actor_tx.clone(),
            sync.metrics().clone(),
        )?;
        let actor_handle = n0_future::task::spawn(
            async move {
                if let Err(err) = actor.run().await {
                    error!("sync actor failed: {err:?}");
                }
            }
            .instrument(error_span!("sync", %me)),
        );

        let default_author = match DefaultAuthor::load(default_author_storage, &sync).await {
            Ok(author) => author,
            Err(err) => {
                // If loading the default author failed, make sure to shutdown the sync actor before
                // returning.
                let _store = sync.shutdown().await.ok();
                return Err(err);
            }
        };

        Ok(Self {
            endpoint,
            sync,
            to_live_actor: live_actor_tx,
            actor_handle: AbortOnDropHandle::new(actor_handle),
            content_status_cb,
            default_author,
            blob_store: bao_store,
            _gc_protect_task: gc_protect_task,
        })
    }

    /// Get the blob store.
    pub fn blob_store(&self) -> &Store {
        &self.blob_store
    }

    /// Returns the metrics tracked for this engine.
    pub fn metrics(&self) -> &Arc<Metrics> {
        self.sync.metrics()
    }

    /// Start to sync a document.
    ///
    /// If `peers` is non-empty, it will both do an initial set-reconciliation sync with each peer,
    /// and join an iroh-gossip swarm with these peers to receive and broadcast document updates.
    pub async fn start_sync(&self, namespace: NamespaceId, peers: Vec<EndpointAddr>) -> Result<()> {
        let (reply, reply_rx) = oneshot::channel();
        self.to_live_actor
            .send(ToLiveActor::StartSync {
                namespace,
                peers,
                reply,
            })
            .await?;
        reply_rx.await??;
        Ok(())
    }

    /// Stop the live sync for a document and leave the gossip swarm.
    ///
    /// If `kill_subscribers` is true, all existing event subscribers will be dropped. This means
    /// they will receive `None` and no further events in case of rejoining the document.
    pub async fn leave(&self, namespace: NamespaceId, kill_subscribers: bool) -> Result<()> {
        let (reply, reply_rx) = oneshot::channel();
        self.to_live_actor
            .send(ToLiveActor::Leave {
                namespace,
                kill_subscribers,
                reply,
            })
            .await?;
        reply_rx.await??;
        Ok(())
    }

    /// Subscribe to replica and sync progress events.
    pub async fn subscribe(
        &self,
        namespace: NamespaceId,
    ) -> Result<impl Stream<Item = Result<LiveEvent>> + Unpin + 'static> {
        // Create a future that sends channel senders to the respective actors.
        // We clone `self` so that the future does not capture any lifetimes.
        let content_status_cb = self.content_status_cb.clone();

        // Subscribe to insert events from the replica.
        let a = {
            let (s, r) = async_channel::bounded(SUBSCRIBE_CHANNEL_CAP);
            self.sync.subscribe(namespace, s).await?;
            Box::pin(r).then(move |ev| {
                let content_status_cb = content_status_cb.clone();
                Box::pin(async move { LiveEvent::from_replica_event(ev, &content_status_cb).await })
            })
        };

        // Subscribe to events from the [`live::Actor`].
        let b = {
            let (s, r) = async_channel::bounded(SUBSCRIBE_CHANNEL_CAP);
            let r = Box::pin(r);
            let (reply, reply_rx) = oneshot::channel();
            self.to_live_actor
                .send(ToLiveActor::Subscribe {
                    namespace,
                    sender: s,
                    reply,
                })
                .await?;
            reply_rx.await??;
            r.map(|event| Ok(LiveEvent::from(event)))
        };

        Ok(a.or(b))
    }

    /// Handle an incoming iroh-docs connection.
    pub async fn handle_connection(&self, conn: iroh::endpoint::Connection) -> anyhow::Result<()> {
        self.to_live_actor
            .send(ToLiveActor::HandleConnection { conn })
            .await?;
        Ok(())
    }

    /// Shutdown the engine.
    pub async fn shutdown(&self) -> Result<()> {
        let (reply, reply_rx) = oneshot::channel();
        self.to_live_actor
            .send(ToLiveActor::Shutdown { reply })
            .await?;
        reply_rx.await?;
        Ok(())
    }
}

/// Converts an [`BlobStatus`] into a ['ContentStatus'].
fn entry_to_content_status(entry: irpc::Result<BlobStatus>) -> ContentStatus {
    match entry {
        Ok(BlobStatus::Complete { .. }) => ContentStatus::Complete,
        Ok(BlobStatus::Partial { .. }) => ContentStatus::Incomplete,
        Ok(BlobStatus::NotFound) => ContentStatus::Missing,
        Err(cause) => {
            tracing::warn!("Error while checking entry status: {cause:?}");
            ContentStatus::Missing
        }
    }
}

/// Events informing about actions of the live sync progress.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, strum::Display)]
pub enum LiveEvent {
    /// A local insertion.
    InsertLocal {
        /// The inserted entry.
        entry: Entry,
    },
    /// Received a remote insert.
    InsertRemote {
        /// The peer that sent us the entry.
        from: PublicKey,
        /// The inserted entry.
        entry: Entry,
        /// If the content is available at the local node
        content_status: ContentStatus,
    },
    /// The content of an entry was downloaded and is now available at the local node
    ContentReady {
        /// The content hash of the newly available entry content
        hash: Hash,
    },
    /// All pending content is now ready.
    ///
    /// This event signals that all queued content downloads from the last sync run have either
    /// completed or failed.
    ///
    /// It will only be emitted after a [`Self::SyncFinished`] event, never before.
    ///
    /// Receiving this event does not guarantee that all content in the document is available. If
    /// blobs failed to download, this event will still be emitted after all operations completed.
    PendingContentReady,
    /// We have a new neighbor in the swarm.
    NeighborUp(PublicKey),
    /// We lost a neighbor in the swarm.
    NeighborDown(PublicKey),
    /// A set-reconciliation sync finished.
    SyncFinished(SyncEvent),
}

impl From<live::Event> for LiveEvent {
    fn from(ev: live::Event) -> Self {
        match ev {
            live::Event::ContentReady { hash } => Self::ContentReady { hash },
            live::Event::NeighborUp(peer) => Self::NeighborUp(peer),
            live::Event::NeighborDown(peer) => Self::NeighborDown(peer),
            live::Event::SyncFinished(ev) => Self::SyncFinished(ev),
            live::Event::PendingContentReady => Self::PendingContentReady,
        }
    }
}

impl LiveEvent {
    async fn from_replica_event(
        ev: crate::Event,
        content_status_cb: &ContentStatusCallback,
    ) -> Result<Self> {
        Ok(match ev {
            crate::Event::LocalInsert { entry, .. } => Self::InsertLocal {
                entry: entry.into(),
            },
            crate::Event::RemoteInsert { entry, from, .. } => Self::InsertRemote {
                content_status: content_status_cb(entry.content_hash()).await,
                entry: entry.into(),
                from: PublicKey::from_bytes(&from)?,
            },
        })
    }
}

/// Where to persist the default author.
///
/// If set to `Mem`, a new author will be created in the docs store before spawning the sync
/// engine. Changing the default author will not be persisted.
///
/// If set to `Persistent`, the default author will be loaded from and persisted to the specified
/// path (as hex encoded string of the author's public key).
#[derive(Debug)]
pub enum DefaultAuthorStorage {
    /// Memory storage.
    Mem,
    /// File based persistent storage.
    #[cfg(feature = "fs-store")]
    Persistent(std::path::PathBuf),
}

impl DefaultAuthorStorage {
    /// Load the default author from the storage.
    ///
    /// Will create and save a new author if the storage is empty.
    ///
    /// Returns an error if the author can't be parsed or if the uathor does not exist in the docs
    /// store.
    pub async fn load(&self, docs_store: &SyncHandle) -> anyhow::Result<AuthorId> {
        match self {
            Self::Mem => {
                let author = Author::new(&mut rand::rng());
                let author_id = author.id();
                docs_store.import_author(author).await?;
                Ok(author_id)
            }
            #[cfg(feature = "fs-store")]
            Self::Persistent(ref path) => {
                use std::str::FromStr;

                use anyhow::Context;
                if path.exists() {
                    let data = tokio::fs::read_to_string(path).await.with_context(|| {
                        format!(
                            "Failed to read the default author file at `{}`",
                            path.to_string_lossy()
                        )
                    })?;
                    let author_id = AuthorId::from_str(&data).with_context(|| {
                        format!(
                            "Failed to parse the default author from `{}`",
                            path.to_string_lossy()
                        )
                    })?;
                    if docs_store.export_author(author_id).await?.is_none() {
                        bail!(
                            "The default author is missing from the docs store. To recover, delete the file `{}`. Then iroh will create a new default author.",
                            path.to_string_lossy()
                        )
                    }
                    Ok(author_id)
                } else {
                    let author = Author::new(&mut rand::rng());
                    let author_id = author.id();
                    docs_store.import_author(author).await?;
                    // Make sure to write the default author to the store
                    // *before* we write the default author ID file.
                    // Otherwise the default author ID file is effectively a dangling reference.
                    docs_store.flush_store().await?;
                    self.persist(author_id).await?;
                    Ok(author_id)
                }
            }
        }
    }

    /// Save a new default author.
    pub async fn persist(&self, #[allow(unused)] author_id: AuthorId) -> anyhow::Result<()> {
        match self {
            Self::Mem => {
                // persistence is not possible for the mem storage so this is a noop.
            }
            #[cfg(feature = "fs-store")]
            Self::Persistent(ref path) => {
                use anyhow::Context;
                tokio::fs::write(path, author_id.to_string())
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to write the default author to `{}`",
                            path.to_string_lossy()
                        )
                    })?;
            }
        }
        Ok(())
    }
}

/// Persistent default author for a docs engine.
#[derive(Debug)]
pub struct DefaultAuthor {
    value: RwLock<AuthorId>,
    storage: DefaultAuthorStorage,
}

impl DefaultAuthor {
    /// Load the default author from storage.
    ///
    /// If the storage is empty creates a new author and persists it.
    pub async fn load(storage: DefaultAuthorStorage, docs_store: &SyncHandle) -> Result<Self> {
        let value = storage.load(docs_store).await?;
        Ok(Self {
            value: RwLock::new(value),
            storage,
        })
    }

    /// Get the current default author.
    pub fn get(&self) -> AuthorId {
        *self.value.read().unwrap()
    }

    /// Set the default author.
    pub async fn set(&self, author_id: AuthorId, docs_store: &SyncHandle) -> Result<()> {
        if docs_store.export_author(author_id).await?.is_none() {
            bail!("The author does not exist");
        }
        self.storage.persist(author_id).await?;
        *self.value.write().unwrap() = author_id;
        Ok(())
    }
}

#[derive(Debug)]
struct ProtectCallbackSender(mpsc::Sender<oneshot::Sender<mpsc::Receiver<Result<Hash>>>>);

/// The handler for a blobs protection callback.
///
/// See [`ProtectCallbackHandler::new`].
#[derive(Debug)]
pub struct ProtectCallbackHandler(
    pub(crate) mpsc::Receiver<oneshot::Sender<mpsc::Receiver<Result<Hash>>>>,
);

impl ProtectCallbackHandler {
    /// Creates a callback and handler to manage blob protection.
    ///
    /// The returned [`ProtectCb`] must be passed set in the [`GcConfig`] of the [`iroh_blobs`] store where
    /// the blobs for hashes in documents are persisted. The [`ProtectCallbackHandler`] must be passed to
    /// [`Builder::protect_handler`] (or [`Engine::spawn`]). This will then ensure that hashes referenced
    /// in docs will not be deleted from the blobs store, and will be garbage collected if they no longer appear
    /// in any doc.
    ///
    /// [`Builder::protect_handler`]: crate::protocol::Builder::protect_handler
    /// [`GcConfig`]: iroh_blobs::store::GcConfig
    pub fn new() -> (Self, ProtectCb) {
        let (tx, rx) = mpsc::channel(4);
        let cb = ProtectCallbackSender(tx).into_cb();
        let handler = ProtectCallbackHandler(rx);
        (handler, cb)
    }
}

impl ProtectCallbackSender {
    fn into_cb(self) -> ProtectCb {
        let start_tx = self.0.clone();
        Arc::new(move |live| {
            let start_tx = start_tx.clone();
            Box::pin(async move {
                let (tx, rx) = oneshot::channel();
                if let Err(_err) = start_tx.send(tx).await {
                    tracing::warn!(
                        "Failed to get protected hashes from docs: ProtectCallback receiver dropped"
                    );
                    return ProtectOutcome::Abort;
                }
                let mut rx = match rx.await {
                    Ok(rx) => rx,
                    Err(_err) => {
                        tracing::warn!(
                            "Failed to get protected hashes from docs: ProtectCallback sender dropped"
                        );
                        return ProtectOutcome::Abort;
                    }
                };
                while let Some(res) = rx.recv().await {
                    match res {
                        Err(err) => {
                            tracing::warn!("Getting protected hashes produces error: {err:#}");
                            return ProtectOutcome::Abort;
                        }
                        Ok(hash) => {
                            live.insert(hash);
                        }
                    }
                }
                ProtectOutcome::Continue
            })
        })
    }
}
