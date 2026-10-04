pub use super::bsd_like_fs::*;
use std::ffi::c_void;

use super::{StatFs, cstring_from_path};
use crate::types::PosixError;
use libc::{self, c_char, c_int, size_t, ssize_t};
use libc::{ENOTSUP, F_ALLOCATECONTIG, F_PREALLOCATE, fcntl, fstore_t, off_t};
use std::path::Path;

pub(crate) use super::bsd_like_fs::{ftruncate, lseek, pread, pwrite};

// Define FALLOC_FL_KEEP_SIZE constant
const FALLOC_FL_KEEP_SIZE: c_int = 1;

pub(super) unsafe fn fallocate(fd: c_int, mode: c_int, offset: i64, len: i64) -> c_int {
    let offset = offset as off_t;
    let len = len as off_t;
    // Check for unsupported modes
    if mode != 0 && mode != FALLOC_FL_KEEP_SIZE {
        // Set errno to "Operation not supported"
        *libc::__error() = ENOTSUP;
        return -1;
    }

    // First, try to allocate contiguous space
    let mut fst: fstore_t = std::mem::zeroed();
    fst.fst_flags = F_ALLOCATECONTIG;
    fst.fst_posmode = libc::F_PEOFPOSMODE;
    fst.fst_offset = offset;
    fst.fst_length = len;

    let mut result = fcntl(fd, F_PREALLOCATE, &fst as *const fstore_t);

    if result != 0 {
        // If contiguous allocation failed, try non-contiguous allocation
        fst.fst_flags = 0;
        result = fcntl(fd, F_PREALLOCATE, &fst as *const fstore_t);
    }
    if result != 0 {
        return -1;
    }

    // If FALLOC_FL_KEEP_SIZE is not set, adjust the file size
    if mode != FALLOC_FL_KEEP_SIZE {
        result = ftruncate(fd, offset + len);
        if result != 0 {
            return -1;
        }
    }

    0 // Success
}

pub(super) unsafe fn setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const c_void,
    size: size_t,
    position: u32,
    flags: c_int,
) -> c_int {
    libc::setxattr(path, name, value, size, position, flags)
}

pub(super) unsafe fn getxattr(
    path: *const c_char,
    name: *const c_char,
    value: *mut c_void,
    size: size_t,
) -> ssize_t {
    libc::getxattr(path, name, value, size, 0, 0)
}

pub(super) unsafe fn listxattr(path: *const c_char, list: *mut c_char, size: size_t) -> ssize_t {
    libc::listxattr(path, list, size, 0)
}

pub(super) unsafe fn removexattr(path: *const c_char, name: *const c_char) -> c_int {
    libc::removexattr(path, name, 0)
}

/// Retrieves file system statistics for the specified path.
///
/// This function is equivalent to the FUSE `statfs` operation.
pub fn statfs(path: &Path) -> Result<StatFs, PosixError> {
    let c_path = cstring_from_path(path)?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };

    // Use statfs to get file system stats
    let result = unsafe { libc::statfs(c_path.as_ptr(), &mut stat) };
    if result != 0 {
        return Err(PosixError::last_error(format!(
            "{}: statfs failed",
            path.display()
        )));
    }

    Ok(StatFs {
        total_blocks: stat.f_blocks as u64,
        free_blocks: stat.f_bfree as u64,
        available_blocks: stat.f_bavail as u64,
        total_files: stat.f_files as u64,
        free_files: stat.f_ffree as u64,
        block_size: stat.f_bsize as u32,
        max_filename_length: 255,
        fragment_size: stat.f_bsize as u32, // BSD doesn't have f_frsize, so we use f_bsize
    })
}
