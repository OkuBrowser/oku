//! Inode number in a FUSE (Filesystem in Userspace) filesystem.

pub use fuser::INodeNo;
pub type Inode = fuser::INodeNo;

/// Represents the mountpoint folder in a FuseFilesystem
/// Its value is 1.
pub const ROOT_INODE: Inode = fuser::INodeNo::ROOT;

pub trait InodeExt {
    fn add_one(&self) -> Self;
}

impl InodeExt for fuser::INodeNo {
    fn add_one(&self) -> Self {
        fuser::INodeNo(self.0 + 1)
    }
}
