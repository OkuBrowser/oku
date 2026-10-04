//! Flags used in FUSE (Filesystem in Userspace) operations.

use bitflags::bitflags;

pub use fuser::{
    AccessFlags,
    FopenFlags,
    OpenFlags,
    RenameFlags,
    IoctlFlags,
    WriteFlags,
    CopyFileRangeFlags,
};

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    /// Flags used in fallocate calls.
    pub struct FallocateFlags: i32 {
        /// Retain file size; don't extend even if offset + len is greater
        #[cfg(target_os = "linux")]
        const KEEP_SIZE = libc::FALLOC_FL_KEEP_SIZE;
        /// Deallocate space (must be ORed with KEEP_SIZE)
        #[cfg(target_os = "linux")]
        const PUNCH_HOLE = libc::FALLOC_FL_PUNCH_HOLE;
        /// Remove a range from the file without leaving a hole
        #[cfg(target_os = "linux")]
        const COLLAPSE_RANGE = libc::FALLOC_FL_COLLAPSE_RANGE;
        /// Zero and ensure allocation of a range
        #[cfg(target_os = "linux")]
        const ZERO_RANGE = libc::FALLOC_FL_ZERO_RANGE;
        /// Insert a hole at the specified range, shifting existing data
        #[cfg(target_os = "linux")]
        const INSERT_RANGE = libc::FALLOC_FL_INSERT_RANGE;
        /// Make shared file data extents private to the file
        #[cfg(target_os = "linux")]
        const UNSHARE_RANGE = libc::FALLOC_FL_UNSHARE_RANGE;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct AttrFlags: u32 {
        const SUBMOUNT = 1 << 0;
        const DAX = 1 << 1;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct GetAttrFlags: i32 {
        const GETATTR_FH = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ReadFlags: i32 {
        const LOCKOWNER = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ReleaseFlags: i32 {
        const FLUSH = 1 << 0;
        const FLOCK_UNLOCK = 1 << 1;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct FsyncFlags: u32 {
        const FDATASYNC = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct SetXAttrFlags: i32 {
        const ACL_KILL_SGID = 1 << 0;
        const _ = !0;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    // c_short in BSD, c_int in linux
    /// Flags representing different types of file locks.
    pub struct LockType: i32 {
        /// No lock held.
        const UNLOCKED = libc::F_UNLCK as i32;
        /// Shared or read lock.
        const READ_LOCK = libc::F_RDLCK as i32;
        /// Exclusive or write lock.
        const WRITE_LOCK = libc::F_WRLCK as i32;
        const _ = !0;
    }
}
