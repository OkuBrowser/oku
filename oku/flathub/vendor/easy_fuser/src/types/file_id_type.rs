//! File identification types and traits for FUSE filesystems.
//!
//! This module defines the `FileIdType` trait and its implementations, which provide
//! flexible ways to identify files in a FUSE filesystem. It supports three main
//! identification methods: inode-based, path-based, and component-based. Each method
//! offers different trade-offs in terms of performance, ease of use, and memory usage.
//! The module also includes associated types for full and minimal metadata, which
//! are different possible return values in FUSE operations.

use std::{
    ffi::OsString,
    fmt::{Debug, Display},
    hash::Hasher,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, atomic::AtomicU64},
};

use super::arguments::FileAttribute;
use super::inode::*;
use crate::{core::InodeResolvable, inode_multi_mapper::InodeMultiMapper};
use fuser::FileType as FileKind;

/// Represents the type used to identify files in the file system.
///
/// This trait allows different approaches to file identification:
///
/// 1. `PathBuf`: Uses file paths for identification.
///    - Pros: Automatic inode-to-path mapping and caching.
///    - Cons: May have performance overhead for large file systems.
///    - Root: Represented by an empty string. Paths are relative and never begin with a forward slash.
///
/// 2. `Vec<OsString>`: Uses a vector of path components for identification.
///    - Pros: Slightly lower overhead than PathBuf, allows path to be divided into parts.
///    - Cons: Path components are stored in reverse order, which may require additional handling.
///    - Root: Represented by an empty vector.
///
/// 3. `Inode`: The user provides their own unique inode numbers.
///    - Pros: Direct control over inode assignment.
///    - Cons: Requires manual management of inode uniqueness.
///    - Root: Represented by the constant ROOT_INODE with a value of 1.
///    - Usage:
///      - The user should provide an inode value for each operation requiring as a return value `<Inode as TId>::Metadata` or `<Inode as TId>::MinimalMetadata` (eg: lookup, create, link, etc.)
///      - Then for subsequent operations concerning the same file, Fuse system will return the provided inode as argument `Inode as TId` (eg: access, getattr, lookup _to reference parent_, etc.)
///
/// 4. `HybridId<BackingId>`: Uses inode for identification; however, file paths are also provided for use.
///     - Pros:
///         - Supports automatic inode-to-path mapping, similar to PathBuf.
///         - User can supply an optional backing ID to accurately reuse an existing inode and model a hard link
///         if the underlying file system uses hard links, and allows for retrieving multiple paths to the same inode.
///         - Hard link relationships and inode values persist after unmounting and remounting the file system.
///     - Cons:
///         - May have more overhead compared to PathBuf.
///         - May lead to performance degradation or service denial if the user tries to exhaustively search all paths
///         to an inode, and hard links were extensively used.
///         - When using first_path method, the pre-supplied PathBuf can change over multiple requests to the same inode, so it should not be used as a
///         comparison method.
///     - Root: Represented by the constant ROOT_INODE with a value of 1 and an empty string.
///     - Usage: (see <https://github.com/Alogani/easy_fuser/pull/77#issuecomment-3830951142>)
///       - If two paths represents hardlinks, the user will return the same inode to the fuse filesystem
///       - The user can use the hardlinks of the current filesystem by using `libc::fstat(...).f_fsid` (Persistent) or libc::fstatfs(...).f_dev` (Ephemeral)
///       - When a Fuse operation provides an inode, the user can use `BackingId::all_paths()` to retrieve all the paths associated to that inode
pub trait FileIdType:
    'static + Debug + Clone + PartialEq + Eq
    + Send + std::hash::Hash + InodeResolvable
{
    /// Full metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileAttribute)
    /// - User must provide both Inode and FileAttribute.
    ///
    /// For PathBuf-based: FileAttribute
    /// - User only needs to provide FileAttribute; Inode is managed internally.
    type Metadata: Send;

    /// Minimal metadata type for the file system.
    ///
    /// For Inode-based: (Inode, FileKind)
    /// - User must provide both Inode and FileKind.
    ///
    /// For PathBuf-based: FileKind
    /// - User only needs to provide FileKind; Inode is managed internally.
    type MinimalMetadata: Send;
    #[doc(hidden)]
    type _Id;

    /// Returns a displayable representation of the file identifier.
    ///
    /// This method provides a human-readable string representation of the file identifier,
    /// which can be useful for debugging, logging, or user-facing output.
    fn display(&self) -> impl Display;

    /// Checks if this file identifier represents the root of the filesystem.
    ///
    /// This method determines whether the current file identifier corresponds to the
    /// topmost directory in the filesystem hierarchy.
    fn is_filesystem_root(&self) -> bool;

    #[doc(hidden)]
    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute);
    #[doc(hidden)]
    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind);
}

impl FileIdType for Inode {
    type _Id = Inode;
    type Metadata = (Inode, FileAttribute);
    type MinimalMetadata = (Inode, FileKind);

    fn display(&self) -> impl Display {
        format!("{:?}", self)
    }

    fn is_filesystem_root(&self) -> bool {
        *self == ROOT_INODE
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        metadata
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        minimal_metadata
    }
}

impl FileIdType for PathBuf {
    type _Id = ();
    type Metadata = FileAttribute;
    type MinimalMetadata = FileKind;

    fn display(&self) -> impl Display {
        Path::display(self)
    }

    fn is_filesystem_root(&self) -> bool {
        self.as_os_str().is_empty()
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        ((), metadata)
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        ((), minimal_metadata)
    }
}

impl FileIdType for Vec<OsString> {
    type _Id = ();
    type Metadata = FileAttribute;
    type MinimalMetadata = FileKind;

    fn display(&self) -> impl Display {
        // Join all paths with a separator for display
        self.iter()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn is_filesystem_root(&self) -> bool {
        self.is_empty()
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        ((), metadata)
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        ((), minimal_metadata)
    }
}

#[derive(Clone)]
pub struct HybridId<BackingId>
where
    BackingId: Clone + Eq + std::hash::Hash + Debug,
{
    inode: Inode,
    mapper: Arc<RwLock<InodeMultiMapper<AtomicU64, BackingId>>>,
}

impl<BackingId> HybridId<BackingId>
where
    BackingId: Clone + Eq + std::hash::Hash + Debug,
{
    /// Creates a new hybrid ID.
    pub fn new(inode: Inode, mapper: Arc<RwLock<InodeMultiMapper<AtomicU64, BackingId>>>) -> Self {
        Self { inode, mapper }
    }

    /// Retrieves the first path to the inode.
    ///
    /// # Notes
    /// - Due to the nature of an inode being able to have multiple links, there can be multiple combinations of path components
    /// that resolve to the same inode. This method only returns the first combination of path components that
    /// resolves to the inode.
    pub fn first_path(&self) -> Option<PathBuf> {
        let mapper = self
            .mapper
            .read()
            .expect("failed to acquire read lock on mapper");
        
        mapper.resolve(&self.inode).map(|components| {
            components
                .iter()
                .map(|component| component.name.as_ref())
                .rev()
                .collect::<PathBuf>()
        })
    }

    /// Retrieves the inode of the hybrid ID.
    pub fn inode(&self) -> &Inode {
        &self.inode
    }

    /// Retrieves all paths to the inode, up to a given limit.
    pub fn all_paths(&self, limit: Option<usize>) -> Vec<PathBuf> {
        let mapper = self
            .mapper
            .read()
            .expect("failed to acquire read lock on mapper");
        let resolved = mapper.resolve_all(&self.inode, limit);
        resolved
            .iter()
            .map(|components| {
                components
                    .iter()
                    .rev()
                    .map(|component| component.name.as_ref())
                    .collect::<PathBuf>()
            })
            .collect()
    }

    /// Retrieves the backing ID of the inode.
    ///
    /// This is useful for comparing to the backing ID of the actual underlying
    /// file that a filesystem handler opened, which mitigates the risk of a race
    /// condition, in which case another backing path could be tried, or an error
    /// could be returned.
    pub fn backing_id(&self) -> Option<BackingId> {
        let mapper = self
            .mapper
            .read()
            .expect("failed to acquire read lock on mapper");
        mapper.get_backing_id(&self.inode).cloned()
    }
}

impl<BackingId> PartialEq for HybridId<BackingId>
where
    BackingId: Clone + Eq + std::hash::Hash + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.inode == other.inode && Arc::ptr_eq(&self.mapper, &other.mapper)
    }
}

impl<BackingId> Eq for HybridId<BackingId> where BackingId: Clone + Eq + std::hash::Hash + Debug {}

impl<BackingId> std::hash::Hash for HybridId<BackingId>
where
    BackingId: Clone + Eq + std::hash::Hash + Debug,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inode.hash(state);
        Arc::as_ptr(&self.mapper).hash(state);
    }
}

impl<BackingId> Debug for HybridId<BackingId>
where
    BackingId: Clone + Eq + std::hash::Hash + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HybridId({:?}, {})",
            self.inode,
            match &self.first_path() {
                Some(path) => path.display().to_string(),
                None => "<orphaned inode>".to_string(),
            }
        )
    }
}

impl<BackingId> FileIdType for HybridId<BackingId>
where
    BackingId: Clone + Eq + std::hash::Hash + Send + Sync + Debug + 'static,
{
    type _Id = Option<BackingId>;
    type Metadata = (Option<BackingId>, FileAttribute);
    type MinimalMetadata = (Option<BackingId>, FileKind);

    fn display(&self) -> impl Display {
        format!(
            "HybridId({:?}, {})",
            self.inode,
            match &self.first_path() {
                Some(path) => path.display().to_string(),
                None => "<orphaned inode>".to_string(),
            }
        )
    }

    fn is_filesystem_root(&self) -> bool {
        self.inode == ROOT_INODE
    }

    fn extract_metadata(metadata: Self::Metadata) -> (Self::_Id, FileAttribute) {
        metadata
    }

    fn extract_minimal_metadata(minimal_metadata: Self::MinimalMetadata) -> (Self::_Id, FileKind) {
        minimal_metadata
    }
}
