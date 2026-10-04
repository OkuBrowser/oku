use crate::core::FileIdResolver;
use crate::types::FileIdType;
use fuser::BackgroundSession;
use std::collections::HashSet;
use std::sync::Arc;

/// A session for a mounted FUSE filesystem running in the background.
///
/// This struct wraps the `fuser::BackgroundSession` and provides access to the
/// underlying inode resolver, allowing for manual maintenance tasks like pruning.
pub struct FuseSession<T: FileIdType> {
    pub(crate) session: BackgroundSession,
    pub(crate) resolver: Arc<T::Resolver>,
}

impl<T: FileIdType> FuseSession<T> {
    pub(crate) fn new(session: BackgroundSession, resolver: Arc<T::Resolver>) -> Self {
        Self { session, resolver }
    }

    /// Creates a lightweight pruner handle that can be sent to other threads.
    pub fn pruner(&self) -> FusePruner<T> {
        FusePruner {
            resolver: self.resolver.clone(),
        }
    }

    /// Prune the resolver by removing unreferenced inodes.
    ///
    /// The `keep` set contains file IDs that should be preserved even if their lookup count is zero.
    /// This is useful for cleaning up inodes that are no longer referenced by the kernel but
    /// were not automatically evicted (e.g., to prevent race conditions).
    pub fn prune(&self, keep: &HashSet<T>) {
        self.resolver.prune(keep);
    }

    /// Join the background session, waiting for the filesystem to unmount.
    ///
    /// This method blocks until the filesystem is unmounted.
    pub fn join(self) -> std::io::Result<()> {
        self.session.umount_and_join()
    }
}

/// A lightweight handle for pruning inodes.
///
/// This struct can be cloned and sent to other threads to perform background pruning
/// without holding the main `FuseSession`.
#[derive(Clone)]
pub struct FusePruner<T: FileIdType> {
    resolver: Arc<T::Resolver>,
}

impl<T: FileIdType> FusePruner<T> {
    /// Prune the resolver by removing unreferenced inodes.
    pub fn prune(&self, keep: &HashSet<T>) {
        self.resolver.prune(keep);
    }
}
