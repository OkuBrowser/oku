#[cfg(feature = "fuse")]
use debug_ignore::DebugIgnore;
#[cfg(feature = "fuse")]
use easy_fuser::fuse_presets::DefaultFuseHandler;
use iroh_blobs::BlobsProtocol;
use iroh_docs::{protocol::Docs, NamespaceId};
use moka::future::Cache;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(feature = "persistent")]
use std::sync::LazyLock;
use tempfile::NamedTempFile;
#[cfg(feature = "fuse")]
use tokio::runtime::Handle;
use tokio::sync::{watch::Sender, Mutex};

/// Core functionality of an Oku file system.
pub mod core;
/// Directory-related functionality of an Oku file system.
pub mod directory;
/// File-related functionality of an Oku file system.
pub mod file;
/// Implementation of OkuNet.
pub mod net;
/// Replica-related functionality of an Oku file system.
pub mod replica;
/// Useful functions for implementing the Oku file system.
pub mod util;

mod file_test;

#[cfg(feature = "persistent")]
/// The path on disk where the file system is stored.
pub const FS_PATH: &str = ".oku";
#[cfg(feature = "persistent")]
pub(crate) static NODE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(FS_PATH).join("node"));

/// An instance of an Oku file system.
///
/// The `OkuFs` struct is the primary interface for interacting with an Oku file system.
#[derive(Clone, Debug)]
pub struct OkuFs {
    pub(crate) endpoint: iroh::Endpoint,
    pub(crate) blobs: BlobsProtocol,
    pub(crate) docs: Docs,
    pub(crate) router: iroh::protocol::Router,
    /// A watcher for when replicas are created, deleted, or imported.
    pub replica_sender: Sender<()>,
    /// A watcher for when OkuNet posts are created, deleted, or modified.
    pub okunet_post_sender: Sender<()>,
    /// A watcher for when OkuNet user records are created, deleted, or modified.
    pub okunet_user_sender: Sender<()>,
    /// A watcher for whether or not content is being fetched from the OkuNet.
    pub okunet_fetch_sender: Sender<bool>,
    #[cfg(feature = "fuse")]
    pub(crate) fuse_handler: DebugIgnore<Arc<DefaultFuseHandler<std::path::PathBuf>>>,
    #[cfg(feature = "fuse")]
    /// A Tokio runtime handle to perform asynchronous operations with.
    pub(crate) handle: Option<Handle>,
    pub(crate) dht: mainline::async_dht::AsyncDht,
    pub(crate) file_cache: Cache<(NamespaceId, PathBuf), Arc<Mutex<NamedTempFile>>>,
}
