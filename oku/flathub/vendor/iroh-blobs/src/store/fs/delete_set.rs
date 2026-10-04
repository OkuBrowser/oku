use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use tracing::warn;

use super::options::PathOptions;
use crate::Hash;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum BaoFilePart {
    Outboard,
    Data,
    Sizes,
    Bitfield,
}

/// Creates a pair of a protect handle and a delete handle.
///
/// The protect handle can be used to protect files from deletion.
/// The delete handle can be used to create transactions in which files can be marked for deletion.
pub(super) fn pair(options: Arc<PathOptions>) -> (ProtectHandle, DeleteHandle) {
    let ds = Arc::new(Mutex::new(DeleteSet::default()));
    (ProtectHandle(ds.clone()), DeleteHandle::new(ds, options))
}

/// Helper to keep track of files to delete after a transaction is committed.
#[derive(Debug, Default)]
struct DeleteSet(BTreeSet<(Hash, BaoFilePart)>);

impl DeleteSet {
    /// Mark a file as to be deleted after the transaction is committed.
    fn delete(&mut self, hash: Hash, parts: impl IntoIterator<Item = BaoFilePart>) {
        for part in parts {
            self.0.insert((hash, part));
        }
    }

    /// Mark a file as to be kept after the transaction is committed.
    ///
    /// This will cancel any previous delete for the same file in the same transaction.
    fn protect(&mut self, hash: Hash, parts: impl IntoIterator<Item = BaoFilePart>) {
        for part in parts {
            self.0.remove(&(hash, part));
        }
    }

    /// Apply the delete set and clear it.
    ///
    /// This will delete all files marked for deletion and then clear the set.
    /// Errors will just be logged.
    fn commit(&mut self, options: &PathOptions) {
        for (hash, to_delete) in &self.0 {
            tracing::debug!("deleting {:?} for {hash}", to_delete);
            let path = match to_delete {
                BaoFilePart::Data => options.data_path(hash),
                BaoFilePart::Outboard => options.outboard_path(hash),
                BaoFilePart::Sizes => options.sizes_path(hash),
                BaoFilePart::Bitfield => options.bitfield_path(hash),
            };
            if let Err(cause) = std::fs::remove_file(&path) {
                // Ignore NotFound errors, if the file is already gone that's fine.
                if cause.kind() != std::io::ErrorKind::NotFound {
                    warn!(
                        "failed to delete {:?} {}: {}",
                        to_delete,
                        path.display(),
                        cause
                    );
                }
            }
        }
        self.0.clear();
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone)]
pub(super) struct ProtectHandle(Arc<Mutex<DeleteSet>>);

/// Protect handle, to be used concurrently with transactions to mark files for keeping.
impl ProtectHandle {
    /// Inside or outside a transaction, mark files as to be kept
    ///
    /// If we are not inside a transaction, this will do nothing.
    pub(super) fn protect(&self, hash: Hash, parts: impl IntoIterator<Item = BaoFilePart>) {
        let mut guard = self.0.lock().unwrap();
        guard.protect(hash, parts);
    }
}

/// A delete handle. The only thing you can do with this is to open transactions that keep track of files to delete.
#[derive(Debug)]
pub(super) struct DeleteHandle {
    ds: Arc<Mutex<DeleteSet>>,
    options: Arc<PathOptions>,
}

impl DeleteHandle {
    fn new(ds: Arc<Mutex<DeleteSet>>, options: Arc<PathOptions>) -> Self {
        Self { ds, options }
    }

    /// Open a file transaction. You can open only one transaction at a time.
    pub(super) fn begin_write(&mut self) -> FileTransaction<'_> {
        FileTransaction::new(self)
    }
}

/// A file transaction. Inside a transaction, you can mark files for deletion.
///
/// Dropping a transaction will clear the delete set. Committing a transaction will apply the delete set by actually deleting the files.
#[derive(Debug)]
pub(super) struct FileTransaction<'a>(&'a DeleteHandle);

impl<'a> FileTransaction<'a> {
    fn new(inner: &'a DeleteHandle) -> Self {
        let guard = inner.ds.lock().unwrap();
        debug_assert!(guard.is_empty());
        drop(guard);
        Self(inner)
    }

    /// Mark files as to be deleted
    pub fn delete(&self, hash: Hash, parts: impl IntoIterator<Item = BaoFilePart>) {
        let mut guard = self.0.ds.lock().unwrap();
        guard.delete(hash, parts);
    }

    /// Apply the delete set and clear it.
    pub fn commit(self) {
        let mut guard = self.0.ds.lock().unwrap();
        guard.commit(&self.0.options);
    }
}

impl Drop for FileTransaction<'_> {
    fn drop(&mut self) {
        self.0.ds.lock().unwrap().clear();
    }
}
