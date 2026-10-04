use libc::{self, c_char, c_int, c_uint};
use std::os::fd::*;

use crate::types::{ErrorKind, PosixError};

pub(crate) fn get_errno() -> i32 {
    unsafe { *libc::__error() }
}

pub(super) fn set_errno(errno: i32) {
    unsafe { *libc::__error() = errno };
}

// Flags are ignored
pub(super) unsafe fn renameat2(
    olddirfd: c_int,
    oldpath: *const c_char,
    newdirfd: c_int,
    newpath: *const c_char,
    _flags: c_uint,
) -> c_int {
    unsafe { libc::renameat(olddirfd, oldpath, newdirfd, newpath) }
}

pub(super) unsafe fn fdatasync(fd: c_int) -> c_int {
    unsafe { libc::fsync(fd) }
}

pub(crate) unsafe fn ftruncate(fd: c_int, length: i64) -> c_int {
    unsafe { libc::ftruncate(fd, length as libc::off_t) }
}

pub(crate) unsafe fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64 {
    unsafe { libc::lseek(fd, offset as libc::off_t, whence) as i64 }
}

pub(crate) unsafe fn pread(fd: c_int, buf: *mut libc::c_void, count: libc::size_t, offset: i64) -> libc::ssize_t {
    unsafe { libc::pread(fd, buf, count, offset as libc::off_t) }
}

pub(crate) unsafe fn pwrite(fd: c_int, buf: *const libc::c_void, count: libc::size_t, offset: i64) -> libc::ssize_t {
    unsafe { libc::pwrite(fd, buf, count, offset as libc::off_t) }
}

/// Copies a range of data from one file to another.
///
/// This function is equivalent to the FUSE `copy_file_range` operation.
///
/// It copies `len` bytes from the file descriptor `fd_in` starting at offset `offset_in`
/// to the file descriptor `fd_out` starting at offset `offset_out`. The function returns
/// the number of bytes actually copied, which may be less than requested.
///
/// Note: This function is not available on all platforms, like BSD, in that case, it will return not implemented.
pub fn copy_file_range(
    _fd_in: BorrowedFd,
    _offset_in: i64,
    _fd_out: BorrowedFd,
    _offset_out: i64,
    _len: u64,
) -> Result<u32, PosixError> {
    Err(PosixError::new(
        ErrorKind::FunctionNotImplemented,
        "copy_file_range is not implemented on this platform" as &str,
    )
    .into())
}
