//! in FUSE filesystem operations. It includes definitions for file attributes,
//! device types, filesystem statistics, and request information.
//!
//! # Key Types
//!
//! - [`DeviceType`]: Represents POSIX device types based on the `rdev` value.
//! - [`StatFs`]: Represents file system statistics, similar to the POSIX `statvfs` structure.
//! - [`RequestInfo`]: Encapsulates essential information about a FUSE request.
//! - [`FileAttribute`]: Represents file attributes for FUSE operations with optional caching parameters.
//! - [`SetAttrRequest`]: Represents a request to set file attributes in a FUSE file system.
//!
//! # Functions
//!
//! - [`seek_from_raw`]: Converts raw seek parameters to a `SeekFrom` enum.
//!
//! This module also re-exports `SeekFrom` from the standard library for convenience.

use std::time::{Duration, SystemTime};

use fuser::FileAttr as FuseFileAttr;
pub use fuser::RequestId;
use fuser::{FileType, Request, TimeOrNow, INodeNo, BsdFileFlags};
use libc::mode_t;

use super::BorrowedFileHandle;
use super::LockType;

pub use std::io::SeekFrom;

pub fn seek_from_raw(whence: Option<i32>, offset: i64) -> SeekFrom {
    match whence {
        Some(libc::SEEK_SET) => SeekFrom::Start(
            offset
                .try_into()
                .expect("Invalid negative seek offset for file start"),
        ),
        Some(libc::SEEK_CUR) => SeekFrom::Current(offset),
        Some(libc::SEEK_END) => SeekFrom::End(offset),
        Some(_) => panic!("Invalid seek code"),
        None => SeekFrom::Start(
            offset
                .try_into()
                .expect("Invalid negative seek offset for file start"),
        ),
    }
}

/// Represents POSIX device types based on the `rdev` value.
///
/// This enum encapsulates various file system object types, including:
/// - Regular files and directories
/// - Character and block devices (with major/minor numbers)
/// - Named pipes, sockets, and symbolic links
#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    RegularFile,
    Directory,
    CharacterDevice { major: u32, minor: u32 },
    BlockDevice { major: u32, minor: u32 },
    NamedPipe,
    Socket,
    Symlink,
    Unknown,
}

impl DeviceType {
    pub fn from_rdev(rdev: mode_t) -> Self {
        use libc::*;
        // Extract major and minor device numbers (assuming the device number format).
        let major: u32 = (rdev >> 8) as u32; // Major is the upper part of the 32-bit value (16 bit on macos)
        let minor: u32 = (rdev & 0xFF) as u32; // Minor is the lower 8 bits
        match rdev {
            x if x & S_IFREG != 0 => DeviceType::RegularFile,
            x if x & S_IFDIR != 0 => DeviceType::Directory,
            x if x & S_IFCHR != 0 => DeviceType::CharacterDevice { major, minor },
            x if x & S_IFBLK != 0 => DeviceType::BlockDevice { major, minor },
            x if x & S_IFIFO != 0 => DeviceType::NamedPipe,
            x if x & S_IFSOCK != 0 => DeviceType::Socket,
            x if x & S_IFLNK != 0 => DeviceType::Symlink,
            _ => DeviceType::Unknown,
        }
    }

    pub fn to_rdev(&self) -> mode_t {
        use libc::*;

        match self {
            DeviceType::RegularFile => S_IFREG,
            DeviceType::Directory => S_IFDIR,
            DeviceType::CharacterDevice { major, minor } => {
                let device = ((major & 0xFF) << 8) | (minor & 0xFF);
                (device as mode_t) | S_IFCHR
            }
            DeviceType::BlockDevice { major, minor } => {
                let device = ((major & 0xFF) << 8) | (minor & 0xFF);
                (device as mode_t) | S_IFBLK
            }
            DeviceType::NamedPipe => S_IFIFO,
            DeviceType::Socket => S_IFSOCK,
            DeviceType::Symlink => S_IFLNK,
            DeviceType::Unknown => 0, // Represents an unknown device
        }
    }
}

/// Represents file system statistics, similar to the POSIX `statvfs` structure.
///
/// Used to report file system status in FUSE operations.
#[derive(Debug, Clone)]
pub struct StatFs {
    /// Total number of blocks
    pub total_blocks: u64,
    /// Number of free blocks
    pub free_blocks: u64,
    /// Number of blocks available to non-root users
    pub available_blocks: u64,
    /// Total number of files
    pub total_files: u64,
    /// Number of free file nodes
    pub free_files: u64,
    /// Size of a block in bytes
    pub block_size: u32,
    /// Maximum length of a filename
    pub max_filename_length: u32,
    /// Fragment size in bytes
    pub fragment_size: u32,
}

impl StatFs {
    /// Creates a default `StatFs` instance with maximum capacity values.
    ///
    /// This method returns a `StatFs` struct with:
    /// - Maximum values for block and file counts
    /// - Standard block size (4096 bytes)
    /// - Typical maximum filename length (255 characters)
    /// - Fragment size matching block size
    ///
    /// Note: These values are placeholders and may not reflect actual file system limits.
    /// For accurate statistics, use the `helper::statfs` function instead.
    pub fn default() -> Self {
        StatFs {
            total_blocks: u64::MAX,
            free_blocks: u64::MAX,
            available_blocks: u64::MAX,
            total_files: u64::MAX / 2,
            free_files: u64::MAX / 2,
            block_size: 4096,
            max_filename_length: 255,
            fragment_size: 4096,
        }
    }
}

/// Encapsulates essential information about a FUSE request.
///
/// Fields:
/// - `id`: Unique identifier for the request
/// - `uid`: User ID of the process that initiated the request
/// - `gid`: Group ID of the process that initiated the request
/// - `pid`: Process ID of the process that initiated the request
#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub id: RequestId,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
}
impl From<&Request> for RequestInfo {
    fn from(req: &Request) -> Self {
        Self {
            id: req.unique(),
            uid: req.uid(),
            gid: req.gid(),
            pid: req.pid(),
        }
    }
}

/// Represents file attributes for FUSE operations with optional caching parameters.
#[derive(Debug, PartialEq, Clone)]
pub struct FileAttribute {
    /// File size in bytes
    pub size: u64,
    /// Number of 512-byte blocks allocated
    pub blocks: u64,
    /// Last access time
    pub atime: SystemTime,
    /// Last modification time
    pub mtime: SystemTime,
    /// Last status change time
    pub ctime: SystemTime,
    /// Creation time
    pub crtime: SystemTime,
    /// File type (regular file, directory, etc.)
    pub kind: FileType,
    /// File permissions
    pub perm: u16,
    /// Number of hard links
    pub nlink: u32,
    /// User ID of the file owner
    pub uid: u32,
    /// Group ID of the file owner
    pub gid: u32,
    /// Device ID (if special file)
    pub rdev: u32,
    /// Preferred block size for file system I/O
    pub blksize: u32,
    /// File flags
    pub flags: u32,
    /// Time-to-live for caching this attribute (None for default)
    pub ttl: Option<Duration>,
    // File generation number (None for random)
    /// If set, it must follow these constraints:
    /// - Must be non-zero (FUSE treats zero as an error)
    /// - Should be unique over the file system's lifetime if exported over NFS
    /// - Should be a new, previously unused number if an inode is reused after deletion
    pub generation: Option<u64>,
}

/// `FuseFileAttr`, `Option<ttl>`, `Option<generation>`
impl FileAttribute {
    pub(crate) fn to_fuse(self, ino: INodeNo) -> (FuseFileAttr, Option<Duration>, Option<u64>) {
        (
            FuseFileAttr {
                ino,
                size: self.size,
                blocks: self.blocks,
                atime: self.atime,
                mtime: self.mtime,
                ctime: self.ctime,
                crtime: self.crtime,
                kind: self.kind,
                perm: self.perm,
                nlink: self.nlink,
                uid: self.uid,
                gid: self.gid,
                rdev: self.rdev,
                blksize: self.blksize,
                flags: self.flags,
            },
            self.ttl,
            self.generation,
        )
    }
}

/// Represents a request to set file attributes in a FUSE file system.
///
/// This struct uses the builder pattern to construct a request with optional fields.
/// Each field corresponds to a file attribute that can be modified.
#[derive(Debug)]
pub struct SetAttrRequest<'a> {
    /// File mode (permissions)
    pub mode: Option<u32>,
    /// User ID of the file owner
    pub uid: Option<u32>,
    /// Group ID of the file owner
    pub gid: Option<u32>,
    /// File size in bytes
    pub size: Option<u64>,
    /// Last access time
    pub atime: Option<TimeOrNow>,
    /// Last modification time
    pub mtime: Option<TimeOrNow>,
    /// Last status change time
    pub ctime: Option<SystemTime>,
    /// Creation time
    pub crtime: Option<SystemTime>,
    /// Change time (for BSD systems)
    pub chgtime: Option<SystemTime>,
    /// Backup time (for macOS)
    pub bkuptime: Option<SystemTime>,
    /// File flags (unused in FUSE)
    pub flags: Option<BsdFileFlags>,
    /// File handle for the file being modified
    pub file_handle: Option<BorrowedFileHandle<'a>>,
}

impl<'a> Default for SetAttrRequest<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> SetAttrRequest<'a> {
    pub fn new() -> Self {
        Self {
            mode: None,
            uid: None,
            gid: None,
            size: None,
            atime: None,
            mtime: None,
            ctime: None,
            crtime: None,
            chgtime: None,
            bkuptime: None,
            flags: None,
            file_handle: None,
        }
    }

    pub fn mode(mut self, mode: u32) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn uid(mut self, uid: u32) -> Self {
        self.uid = Some(uid);
        self
    }

    pub fn gid(mut self, gid: u32) -> Self {
        self.gid = Some(gid);
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn atime(mut self, atime: TimeOrNow) -> Self {
        self.atime = Some(atime);
        self
    }

    pub fn mtime(mut self, mtime: TimeOrNow) -> Self {
        self.mtime = Some(mtime);
        self
    }

    pub fn ctime(mut self, ctime: SystemTime) -> Self {
        self.ctime = Some(ctime);
        self
    }

    pub fn crtime(mut self, crtime: SystemTime) -> Self {
        self.crtime = Some(crtime);
        self
    }

    pub fn chgtime(mut self, chgtime: SystemTime) -> Self {
        self.chgtime = Some(chgtime);
        self
    }

    pub fn bkuptime(mut self, bkuptime: SystemTime) -> Self {
        self.bkuptime = Some(bkuptime);
        self
    }

    /// Unused by FUSE
    pub fn flags(mut self, flags: BsdFileFlags) -> Self {
        self.flags = Some(flags);
        self
    }

    pub fn file_handle(mut self, file_handle: BorrowedFileHandle<'a>) -> Self {
        self.file_handle = Some(file_handle);
        self
    }
}

/// Represents file locking information for FUSE operations.
#[derive(Debug)]
pub struct LockInfo {
    /// Starting offset of the lock range in bytes
    pub start: u64,
    /// Ending offset of the lock range in bytes (exclusive)
    pub end: u64,
    /// Type of lock (e.g., Read, Write, or Unlock)
    pub lock_type: LockType,
    /// Process ID of the lock owner
    pub pid: u32,
}
