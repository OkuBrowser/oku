pub use super::bsd_like_fs::*;
use std::path::Path;
use std::ffi::{CStr, CString};
use crate::PosixError;

use libc::{self, c_char, c_int, size_t, ssize_t, off_t, c_void};

pub(crate) use super::bsd_like_fs::{ftruncate, lseek, pread, pwrite};

use super::{StatFs, cstring_from_path};

pub(super) unsafe fn fallocate(fd: c_int, _mode: c_int, offset: i64, len: i64) -> c_int {
    unsafe { libc::posix_fallocate(fd, offset as off_t, len as off_t) }
}

/// Warning: untested function
pub(super) unsafe fn setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const c_void,
    size: libc::size_t,
    position: u32,
    _flags: c_int,           // ignored on FreeBSD
) -> c_int {
    unsafe {
        if position != 0 {
            return -libc::EOPNOTSUPP;
        }

        // Convert C strings → Rust &str (safe failure → EINVAL)
        let name_str = match CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return -libc::EINVAL,
        };

        // Map common prefixed names → FreeBSD namespace
        let (ns, attr_name): (c_int, &str) = if name_str.starts_with("user.") {
            (libc::EXTATTR_NAMESPACE_USER, &name_str[5..])
        } else if name_str.starts_with("system.") {
            (libc::EXTATTR_NAMESPACE_SYSTEM, &name_str[7..])
        } else if name_str.starts_with("trusted.") {
            (libc::EXTATTR_NAMESPACE_SYSTEM, &name_str[8..])
        } else if name_str.starts_with("security.") {
            (libc::EXTATTR_NAMESPACE_SYSTEM, &name_str[9..])
        } else {
            (libc::EXTATTR_NAMESPACE_USER, name_str)
        };

        let attr_name_c: CString = match CString::new(attr_name) {
            Ok(c) => c,
            Err(_) => return -libc::EINVAL,
        };

        let ret = libc::extattr_set_file(
            path,
            ns,
            attr_name_c.as_ptr(),
            value as *mut c_void, // FreeBSD wants mutable pointer
            size,
        );

        if ret >= 0 {
            0
        } else {
            -(*libc::__error()) // correct FreeBSD way to read errno
        }
    }
}

pub(super) unsafe fn getxattr(
    path: *const c_char,
    name: *const c_char,
    value: *mut c_void,
    size: size_t,
) -> ssize_t {
    unsafe { libc::extattr_get_file(path, libc::EXTATTR_NAMESPACE_USER, name, value, size) }
}

pub(super) unsafe fn listxattr(path: *const c_char, list: *mut c_char, size: size_t) -> ssize_t {
    unsafe { libc::extattr_list_file(
        path,
        libc::EXTATTR_NAMESPACE_USER,
        list as *mut c_void,
        size,
    ) }
}

pub(super) unsafe fn removexattr(path: *const c_char, name: *const c_char) -> c_int {
    unsafe { libc::extattr_delete_file(path, libc::EXTATTR_NAMESPACE_USER, name) }
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
        max_filename_length: stat.f_namemax as u32,
        fragment_size: stat.f_bsize as u32, // BSD doesn't have f_frsize, so we use f_bsize
    })
}
