//! # POSIX Filesystem Operations Module compatible with FUSE signatures.
//!
//! This module provides a set of functions that wrap POSIX filesystem operations,
//! making them more convenient to use within a Rust-based FUSE (Filesystem in Userspace) implementation.
//! It offers a layer of abstraction over low-level system calls, handling conversions between
//! Rust types and C types, as well as error handling.
//!
//! ## Key Features:
//!
//! - File type and attribute conversions between standard Rust types and FUSE-specific types.
//! - Bridge layer between POSIX-compliant system calls and libfuse filesystem operations.
//! - Error handling using custom `PosixError` type.
//! - Utilities for working with file descriptors, paths, and system time.
//!
//! ## Usage:
//!
//! This module is designed to be used as part of a larger FUSE implementation.
//! It provides the necessary tools to interact with the underlying filesystem
//! in a POSIX-compliant manner, while presenting the data in a format suitable
//! for FUSE operations.
//!
//! Note: Some operations, especially those involving symlinks, may require
//! special handling or additional considerations.

/*
Platform specific notes:
- mode_t is a u16 on bsd and u32 on linux
*/

use std::path::Path;
use std::time::{Duration, SystemTime};

use std::ffi::{CStr, CString, OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::*;

use crate::types::*;
use libc::{c_char, c_void, timespec};

// Modify to #[cfg_attr(windows, path = "windows/mod.rs")]
#[cfg(target_os = "linux")]
pub(crate) mod linux_fs;
#[cfg(target_os = "linux")]
use linux_fs as unix_impl;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub(crate) mod bsd_like_fs;

#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub(crate) mod bsd_fs;
#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
use bsd_fs as unix_impl;

#[cfg(target_os = "macos")]
pub(crate) mod macos_fs;
#[cfg(target_os = "macos")]
use macos_fs as unix_impl;

pub(crate) use unix_impl::get_errno;
pub(crate) use unix_impl::lseek as lseek_raw;
pub use unix_impl::{copy_file_range, statfs};

/// Converts a `std::fs::FileType` to the corresponding `FileKind` expected by fuse_api.
///
/// This function is made public for specific use cases.
///
/// This function maps the file types provided by the standard library to the file kinds
/// used in the FUSE API. It handles all common file types, including regular files,
/// directories, symlinks, block devices, character devices, named pipes, and sockets.
pub fn convert_filetype(f: fs::FileType) -> FileKind {
    match f {
        f if f.is_file() => FileKind::RegularFile,
        f if f.is_dir() => FileKind::Directory,
        f if f.is_symlink() => FileKind::Symlink,
        f if f.is_block_device() => FileKind::BlockDevice,
        f if f.is_char_device() => FileKind::CharDevice,
        f if f.is_fifo() => FileKind::NamedPipe,
        f if f.is_socket() => FileKind::Socket,
        _ => panic!("Unknown FileKind"), // not possible in theory
    }
}

/// Converts `std::fs::Metadata` to `FileAttribute` expected by fuser.
///
/// This function is made public for specific use cases.
///
/// It maps standard filesystem metadata to the `FileAttribute` struct used in the FUSE API.
/// Handles file size, timestamps, permissions, ownership, and other attributes.
///
/// # Important
/// For symlinks, use `fs::symlink_metadata` instead of regular `fs::metadata`.
pub fn convert_fileattribute(metadata: fs::Metadata) -> FileAttribute {
    FileAttribute {
        size: metadata.size(),
        blocks: metadata.blocks(),
        atime: SystemTime::UNIX_EPOCH + Duration::new(metadata.atime() as u64, 0),
        mtime: SystemTime::UNIX_EPOCH + Duration::new(metadata.mtime() as u64, 0),
        ctime: SystemTime::UNIX_EPOCH + Duration::new(metadata.ctime() as u64, 0),
        crtime: SystemTime::UNIX_EPOCH + Duration::new(metadata.mtime() as u64, 0),
        kind: convert_filetype(metadata.file_type()),
        perm: (metadata.mode() & 0o777) as u16,
        nlink: metadata.nlink() as u32,
        uid: metadata.uid(),
        gid: metadata.gid(),
        rdev: metadata.rdev() as u32,
        blksize: metadata.blksize() as u32,
        flags: 0, // macOS only; placeholder here
        ttl: None,
        generation: None,
    }
}

fn convert_stat_struct(statbuf: libc::stat) -> Option<FileAttribute> {
    // Convert timestamp values to SystemTime
    let atime = SystemTime::UNIX_EPOCH + Duration::new(statbuf.st_atime as u64, 0);
    let mtime = SystemTime::UNIX_EPOCH + Duration::new(statbuf.st_mtime as u64, 0);
    let ctime = SystemTime::UNIX_EPOCH + Duration::new(statbuf.st_ctime as u64, 0);
    // Extract permissions (lower 9 bits of st_mode)
    let perm = (statbuf.st_mode & (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO)) as u16;
    // Flags are not directly supported in `stat`, use a placeholder for now
    let flags = 0; // Update this if your system provides a way to get file flags

    Some(FileAttribute {
        size: statbuf.st_size as u64,
        blocks: statbuf.st_blocks as u64,
        atime,
        mtime,
        ctime,
        crtime: mtime,
        kind: stat_to_kind(statbuf)?,
        perm,
        nlink: statbuf.st_nlink as u32,
        uid: statbuf.st_uid,
        gid: statbuf.st_gid,
        rdev: statbuf.st_rdev as u32,
        blksize: statbuf.st_blksize as u32,
        flags,
        ttl: None,
        generation: None,
    })
}

fn stat_to_kind(statbuf: libc::stat) -> Option<FileKind> {
    use libc::*;
    Some(match statbuf.st_mode & S_IFMT {
        S_IFREG => FileKind::RegularFile,
        S_IFDIR => FileKind::Directory,
        S_IFCHR => FileKind::CharDevice,
        S_IFBLK => FileKind::BlockDevice,
        S_IFIFO => FileKind::NamedPipe,
        S_IFLNK => FileKind::Symlink,
        S_IFSOCK => FileKind::Socket,
        _ => return None, // Unsupported or unknown file type
    })
}

fn system_time_to_timespec(time: SystemTime) -> Result<timespec, PosixError> {
    let duration = time.duration_since(std::time::UNIX_EPOCH).map_err(|_| {
        PosixError::new(
            ErrorKind::InvalidArgument,
            "System time could not be converted to TimeSpec",
        )
    })?;
    Ok(timespec {
        tv_sec: duration.as_secs() as _,
        tv_nsec: duration.subsec_nanos() as _,
    })
}

fn cstring_from_path(path: &Path) -> Result<CString, PosixError> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        PosixError::new(
            ErrorKind::InvalidArgument,
            format!("{}: Cstring conversion failed", path.display()),
        )
    })
}

/// Retrieves file attributes for a given path.
///
/// This function is equivalent to the FUSE `lookup` operation.
pub fn lookup(path: &Path) -> Result<FileAttribute, PosixError> {
    let c_path = cstring_from_path(path)?;
    let mut statbuf: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::lstat(c_path.as_ptr(), &mut statbuf) };
    if result == -1 {
        return Err(PosixError::last_error(format!(
            "{}: lstat failed in lookup",
            path.display()
        )));
    }
    convert_stat_struct(statbuf).ok_or(PosixError::new(
        ErrorKind::InvalidArgument,
        format!(
            "{}: statbuf conversion failed {:?}",
            path.display(),
            statbuf
        ),
    ))
}

/// Retrieves file attributes for a given file descriptor.
///
/// This function is equivalent to the FUSE `getattr` operation.
///
/// # Note
/// This function uses the system's file descriptor. If tracking custom inodes,
/// additional handling may be required.
pub fn getattr(fd: BorrowedFd) -> Result<FileAttribute, PosixError> {
    let mut statbuf: libc::stat = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::fstat(fd.as_raw_fd(), &mut statbuf) };
    if result == -1 {
        return Err(PosixError::last_error(format!(
            "{:?}: fstat failed in getattr",
            fd
        )));
    }
    convert_stat_struct(statbuf).ok_or(PosixError::new(
        ErrorKind::InvalidArgument,
        format!("{:?}: statbuf conversion failed {:?}", fd, statbuf),
    ))
}

/// Modifies file attributes for a given path.
///
/// This function is equivalent to the FUSE `setattr` operation. It handles changes
/// to file permissions, ownership, size, and timestamps using system calls.
pub fn setattr(path: &Path, attrs: SetAttrRequest) -> Result<FileAttribute, PosixError> {
    let c_path = cstring_from_path(path)?;

    // update permissions
    if let Some(mode) = attrs.mode {
        let result = unsafe { libc::chmod(c_path.as_ptr(), mode.try_into().unwrap()) };
        if result == -1 {
            return Err(PosixError::last_error(format!(
                "{}: chmod failed in setattr",
                path.display()
            )));
        }
    }

    // Change file owner (UID and GID)
    if attrs.uid.is_some() || attrs.gid.is_some() {
        let uid = attrs.uid.unwrap_or(0_u32.wrapping_sub(1));
        let gid = attrs.gid.unwrap_or(0_u32.wrapping_sub(1));

        let result = unsafe { libc::chown(c_path.as_ptr(), uid, gid) };
        if result == -1 {
            return Err(PosixError::last_error(format!(
                "{}: chown failed in setattr",
                path.display()
            )));
        }
    }

    // Change file size (if `size` is provided)
    if let Some(size) = attrs.size {
        let result = {
            // If we have no file handle, use `open` to get one, then `ftruncate`
            let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_WRONLY) };
            if fd == -1 {
                return Err(PosixError::last_error(format!(
                    "{}: open failed in setattr",
                    path.display()
                )));
            }
            let res = unsafe {
                unix_impl::ftruncate(
                    fd,
                    i64::try_from(size).map_err(|_| {
                        PosixError::new(
                            ErrorKind::InvalidArgument,
                            format!(
                                "{}: ftruncate size ({}) out of bound in setattr",
                                path.display(),
                                size
                            ),
                        )
                    })?,
                )
            };
            unsafe { libc::close(fd) };
            res
        };

        if result == -1 {
            return Err(PosixError::last_error(format!(
                "{}: ftruncate failed on setattr",
                path.display()
            )));
        }
    }

    // Set access and modification times (atime and mtime)
    if let (Some(atime), Some(mtime)) = (attrs.atime, attrs.mtime) {
        let times = match (atime, mtime) {
            (TimeOrNow::Now, TimeOrNow::Now) => {
                let now_spec = system_time_to_timespec(SystemTime::now())?;
                [now_spec, now_spec]
            }
            (TimeOrNow::SpecificTime(at), TimeOrNow::SpecificTime(mt)) => {
                let at_spec = system_time_to_timespec(at)?;
                let mt_spec = system_time_to_timespec(mt)?;
                [at_spec, mt_spec]
            }
            _ => {
                return Err(PosixError::new(
                    ErrorKind::InvalidArgument,
                    "Could not convert timespec to TimeOrNow in setattr",
                ));
            }
        };
        let result = unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                c_path.as_ptr(),
                &times[0],
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result == -1 {
            return Err(PosixError::last_error(format!(
                "{}: utimensat failed in setattr",
                path.display()
            )));
        }
    }

    lookup(path)
}

/// Reads the target of a symbolic link.
///
/// This function is equivalent to the FUSE `readlink` operation.
pub fn readlink(path: &Path) -> Result<Vec<u8>, PosixError> {
    let c_path = cstring_from_path(path)?;
    let mut buf = vec![0u8; 1024]; // Initial buffer size
    let ret =
        unsafe { libc::readlink(c_path.as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: readlink",
            path.display()
        )));
    }
    buf.truncate(ret as usize);
    Ok(buf)
}

/// Creates a new file node (device special file or named pipe) at the specified path.
///
/// This function is equivalent to the FUSE `mknod` operation and uses the system's mknod call.
pub fn mknod(
    path: &Path,
    mode: u32,
    umask: u32,
    rdev: DeviceType,
) -> Result<FileAttribute, PosixError> {
    let c_path = cstring_from_path(path)?;
    let final_mode = mode & !umask;
    let ret = unsafe {
        libc::mknod(
            c_path.as_ptr(),
            final_mode.try_into().unwrap(),
            rdev.to_rdev() as libc::dev_t,
        )
    };
    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: mknod failed",
            path.display()
        )));
    }
    lookup(path)
}

/// Creates a new directory at the specified path with the given mode and umask.
///
/// This function is equivalent to the FUSE `mkdir` operation and uses the system's mkdir call.
pub fn mkdir(path: &Path, mode: u32, umask: u32) -> Result<FileAttribute, PosixError> {
    let c_path = cstring_from_path(path)?;
    let final_mode = mode & !umask;
    let ret = unsafe { libc::mkdir(c_path.as_ptr(), final_mode.try_into().unwrap()) };
    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: mkdir failed in setattr",
            path.display()
        )));
    }
    lookup(path)
}

/// Removes a file at the specified path.
///
/// This function is equivalent to the FUSE `unlink` operation and uses the system's unlink call.
pub fn unlink(path: &Path) -> Result<(), PosixError> {
    let c_path = cstring_from_path(path)?;
    let result = unsafe { libc::unlink(c_path.as_ptr()) };
    if result == -1 {
        return Err(PosixError::last_error(format!(
            "{}: unlink failed",
            path.display()
        )));
    }
    Ok(())
}

/// Removes an empty directory at the specified path.
///
/// This function is equivalent to the FUSE `rmdir` operation and uses the system's rmdir call.
pub fn rmdir(path: &Path) -> Result<(), PosixError> {
    let c_path = cstring_from_path(path)?;
    let result = unsafe { libc::rmdir(c_path.as_ptr()) };
    if result == -1 {
        return Err(PosixError::last_error(format!(
            "{}: rmdir failed",
            path.display()
        )));
    }
    Ok(())
}

/// Creates a symbolic link at the specified path, pointing to the given target.
///
/// This function is equivalent to the FUSE `symlink` operation and uses the system's symlink call.
pub fn symlink(path: &Path, target: &Path) -> Result<FileAttribute, PosixError> {
    let c_path = cstring_from_path(path)?;
    let c_target = cstring_from_path(target)?;

    let result = unsafe { libc::symlink(c_target.as_ptr(), c_path.as_ptr()) };
    if result == -1 {
        return Err(PosixError::last_error(format!(
            "{}: symlink failed (target: {})",
            path.display(),
            target.display()
        )));
    }

    lookup(path)
}

/// Renames a file or directory from the old path to the new path.
///
/// This function is equivalent to the FUSE `rename` operation.
/// It allows for specifying additional flags to control the rename operation.
/// However, those flags will be ignored on other platforms than linux.
pub fn rename(oldpath: &Path, newpath: &Path, flags: RenameFlags) -> Result<(), PosixError> {
    let old_cstr = cstring_from_path(oldpath)?;
    let new_cstr = cstring_from_path(newpath)?;
    let result = unsafe {
        unix_impl::renameat2(
            libc::AT_FDCWD, // Current working directory for the old path
            old_cstr.as_ptr(),
            libc::AT_FDCWD, // Current working directory for the new path
            new_cstr.as_ptr(),
            flags.bits(),
        )
    };
    if result == 0 {
        return Ok(());
    }
    Err(PosixError::last_error(format!(
        "{}: rename failed into {}",
        oldpath.display(),
        newpath.display()
    )))
}

/// Opens a file at the specified path with given flags.
///
/// This function is equivalent to the FUSE `open` operation. It returns a file descriptor
/// which may not necessarily be equivalent to the FUSE file handle.
///
/// Although this function returns a Fd, it is guaranted to be positive and valid.
pub fn open(path: &Path, flags: OpenFlags) -> Result<OwnedFd, PosixError> {
    let c_path = cstring_from_path(path)?;
    let fd = unsafe { libc::open(c_path.as_ptr(), flags.0) };
    if fd == -1 {
        return Err(PosixError::last_error(format!(
            "{}: open failed",
            path.display()
        )));
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Reads data from a file descriptor at a specified offset.
///
/// This function is equivalent to the FUSE `read` operation.
///
/// Note: This function reads from the specified offset, which is determined by the `seek` parameter.
/// If `seek` is `SeekFrom::Start`, it reads from that absolute position.
/// For `SeekFrom::Current` or `SeekFrom::End`, it first updates the file's current position,
/// then reads from there. In all cases, the file's position after the read operation
/// remains where it was before the read, regardless of how much data was read.
pub fn read(fd: BorrowedFd, seek: SeekFrom, size: usize) -> Result<Vec<u8>, PosixError> {
    let mut buffer = vec![0; size as usize];
    let offset: i64 = match seek {
        SeekFrom::Start(offset) => i64::try_from(offset).map_err(|_| {
            PosixError::new(
                ErrorKind::InvalidArgument,
                "Offset too large for i64".to_string(),
            )
        })?,
        SeekFrom::Current(offset) => {
            let current = lseek(fd, SeekFrom::Current(0))?;
            current.checked_add(offset).ok_or_else(|| {
                PosixError::new(
                    ErrorKind::InvalidArgument,
                    "Resulting offset too large for off_t".to_string(),
                )
            })?
        }
        SeekFrom::End(offset) => {
            let end = lseek(fd, SeekFrom::End(0))?;
            end.checked_add(offset).ok_or_else(|| {
                PosixError::new(
                    ErrorKind::InvalidArgument,
                    "Resulting offset too large for off_t".to_string(),
                )
            })?
        }
    };
    let bytes_read = unsafe {
        unix_impl::pread(
            fd.as_raw_fd(),
            buffer.as_mut_ptr() as *mut libc::c_void,
            size,
            offset,
        )
    };
    if bytes_read == -1 {
        return Err(PosixError::last_error(format!("{:?}: read failed", fd)));
    }
    buffer.truncate(bytes_read as usize);
    Ok(buffer)
}

/// Writes data to a file descriptor at a specified offset.
///
/// This function is equivalent to the FUSE `write` operation.
///
/// Note: This function reads from the specified offset, which is determined by the `seek` parameter.
/// If `seek` is `SeekFrom::Start`, it reads from that absolute position.
/// For `SeekFrom::Current` or `SeekFrom::End`, it first updates the file's current position,
/// then reads from there. In all cases, the file's position after the read operation
/// remains where it was before the read, regardless of how much data was read.
pub fn write(fd: BorrowedFd, seek: SeekFrom, data: &[u8]) -> Result<usize, PosixError> {
    let bytes_to_write = data.len() as usize;
    let offset: i64 = match seek {
        SeekFrom::Start(offset) => i64::try_from(offset).map_err(|_| {
            PosixError::new(
                ErrorKind::InvalidArgument,
                "Offset too large for i64".to_string(),
            )
        })?,
        SeekFrom::Current(offset) => {
            let current = lseek(fd, SeekFrom::Current(0))?;
            current.checked_add(offset).ok_or_else(|| {
                PosixError::new(
                    ErrorKind::InvalidArgument,
                    "Resulting offset too large for off_t".to_string(),
                )
            })?
        }
        SeekFrom::End(offset) => {
            let end = lseek(fd, SeekFrom::End(0))?;
            end.checked_add(offset).ok_or_else(|| {
                PosixError::new(
                    ErrorKind::InvalidArgument,
                    "Resulting offset too large for off_t".to_string(),
                )
            })?
        }
    };
    let bytes_written = unsafe {
        unix_impl::pwrite(
            fd.as_raw_fd(),
            data.as_ptr() as *const libc::c_void,
            bytes_to_write,
            offset,
        )
    };
    if bytes_written == -1 {
        return Err(PosixError::last_error(format!("{:?}: write failed", fd)));
    }

    Ok(bytes_written as usize)
}

/// Flushes any buffered data to the file system for the given file descriptor.
///
/// This function is equivalent to the FUSE `flush` operation and uses the system's fdatasync call.
pub fn flush(fd: BorrowedFd) -> Result<(), PosixError> {
    let result = unsafe { unix_impl::fdatasync(fd.as_raw_fd()) };
    if result == -1 {
        return Err(PosixError::last_error(format!("{:?}: flush failed", fd)));
    }

    Ok(())
}

/// Synchronizes a file's in-core state with storage device.
///
/// This function is equivalent to the FUSE `fsync` operation. It uses either `fdatasync` or `fsync`
/// system call depending on the `datasync` parameter.
///
/// - If `datasync` is true, it calls `fdatasync`, which synchronizes only the file's data.
/// - If `datasync` is false, it calls `fsync`, which synchronizes both the file's data and metadata.
pub fn fsync(fd: BorrowedFd, datasync: bool) -> Result<(), PosixError> {
    let fd = fd.as_raw_fd();
    let result = unsafe {
        if datasync {
            unix_impl::fdatasync(fd)
        } else {
            libc::fsync(fd)
        }
    };

    if result == -1 {
        return Err(PosixError::last_error(format!("{:?}: fsync failed", fd)));
    }

    Ok(())
}

/// Reads the contents of a directory.
///
/// This function is equivalent to the FUSE `readdir` operation. It returns a vector of tuples,
/// each containing the filename as an OsString and the file type as a FileKind.
pub fn readdir(path: &Path) -> Result<Vec<(OsString, FileKind)>, PosixError> {
    let c_path = cstring_from_path(path)?;
    let dir = unsafe { libc::opendir(c_path.as_ptr()) };
    if dir.is_null() {
        return Err(PosixError::last_error(format!(
            "{}: opendir failed",
            path.display()
        )));
    }

    let mut result = Vec::new();
    loop {
        unix_impl::set_errno(0);
        let entry = unsafe { libc::readdir(dir) };
        if entry.is_null() {
            if unix_impl::get_errno() != 0 {
                unsafe { libc::closedir(dir) };
                return Err(PosixError::last_error(format!(
                    "{}: readdir failed",
                    path.display()
                )));
            }
            break;
        }

        let entry = unsafe { &*entry };
        let name = unsafe { CStr::from_ptr(entry.d_name.as_ptr()) };
        let name = OsStr::from_bytes(name.to_bytes()).to_owned();

        if name == OsStr::new(".") || name == OsStr::new("..") {
            continue;
        }

        let mut statbuf: libc::stat = unsafe { std::mem::zeroed() };
        let full_path = path.join(&name);
        let c_full_path = cstring_from_path(&full_path)?;
        let stat_result = unsafe { libc::lstat(c_full_path.as_ptr(), &mut statbuf) };
        if stat_result == -1 {
            unsafe { libc::closedir(dir) };
            return Err(PosixError::last_error(format!(
                "{}: lstat failed",
                full_path.display()
            )));
        }

        if let Some(attr) = convert_stat_struct(statbuf) {
            result.push((name, attr.kind));
        }
    }

    unsafe { libc::closedir(dir) };
    Ok(result)
}

/// Releases a file descriptor, closing the associated file.
///
/// This function is equivalent to the FUSE `release` operation. It closes the file descriptor
/// using the system's close call.
///
/// Note:
/// - This function is integrated automatically into the FUSE API, so the file handle (as fd)
///   transmitted to it doesn't need to be released manually.
/// - It's unnecessary to call this function for `FileDescriptorGuard` instances, as they
///   automatically handle release on drop.
pub fn release(fd: OwnedFd) -> Result<(), PosixError> {
    // Attempt to close the file descriptor.
    let raw_fd = fd.into_raw_fd();
    let result = unsafe { libc::close(raw_fd) };

    if result == -1 {
        // Handle errors from the close system call.
        return Err(PosixError::last_error(format!(
            "{:?}: release failed",
            raw_fd
        )));
    }
    Ok(())
}

/// Sets an extended attribute for a file or directory.
///
/// This function is equivalent to the FUSE `setxattr` operation. It allows setting
/// extended attributes (key-value metadata) on files or directories.
///
/// # Arguments
/// * `path` - A reference to the `Path` of the file or directory.
/// * `name` - The name of the extended attribute as an `OsStr`.
/// * `value` - The value of the extended attribute as a byte slice.
/// * `flags` - Additional flags for the setxattr operation.
/// * `position` - The position at which to set the value (usually 0 for the entire attribute), ignored in linux
///
/// # Notes
/// - Extended attributes are additional metadata that can be associated with files or directories.
/// - The behavior may vary depending on the underlying filesystem support for extended attributes.
/// - Some filesystems may have limitations on attribute names or value sizes.
pub fn setxattr(
    path: &Path,
    name: &OsStr,
    value: &[u8],
    flags: SetXAttrFlags,
    position: u32,
) -> Result<(), PosixError> {
    let c_path = cstring_from_path(path)?;
    let c_name = CString::new(name.as_bytes()).map_err(|_| {
        PosixError::new(
            ErrorKind::InvalidArgument,
            format!(
                "{}: Cstring conversion failed in setxattr",
                Path::display(name.as_ref())
            ),
        )
    })?;
    let ret = unsafe {
        unix_impl::setxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            value.as_ptr() as *const c_void,
            value.len(),
            position,
            flags.bits(),
        )
    };

    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: setxattr failed. Name: {}, value: {:?}, position: {}",
            path.display(),
            Path::display(name.as_ref()),
            value,
            position
        )));
    }
    Ok(())
}

/// Equivalent to the fuse function of the same name/// Retrieves an extended attribute for a file or directory.
///
/// This function is equivalent to the FUSE `getxattr` operation. It allows retrieving
/// extended attributes (key-value metadata) from files or directories.
///
/// # Arguments
/// * `path` - A reference to the `Path` of the file or directory.
/// * `name` - The name of the extended attribute as an `OsStr`.
/// * `size` - The size of the buffer to store the attribute value.
///
/// # Returns
/// * `Result<Vec<u8>>` containing the value of the extended attribute if successful.
///
/// # Notes
/// - Extended attributes are additional metadata associated with files or directories.
/// - The behavior may vary depending on the underlying filesystem support for extended attributes.
/// - If the provided buffer size is too small, the function may return an error.
pub fn getxattr(path: &Path, name: &OsStr, size: u32) -> Result<Vec<u8>, PosixError> {
    let c_path = cstring_from_path(path)?;
    let c_name = CString::new(name.as_bytes()).map_err(|_| {
        PosixError::new(
            ErrorKind::InvalidArgument,
            format!(
                "{}: Cstring conversion failed in getxattr",
                Path::display(name.as_ref())
            ),
        )
    })?;

    let mut buf = vec![0u8; size as usize];
    let ret = unsafe {
        unix_impl::getxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
        )
    };

    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: getxattr failed. Name: {}, Size: {}",
            path.display(),
            Path::display(name.as_ref()),
            size
        )));
    }

    buf.truncate(ret as usize);
    Ok(buf)
}

/// Lists extended attributes for a file or directory.
///
/// This function is equivalent to the FUSE `listxattr` operation. It retrieves a list of
/// extended attribute names associated with the specified file or directory.
///
/// # Arguments
/// * `path` - A reference to the `Path` of the file or directory.
/// * `size` - The size of the buffer to store the attribute names.
///
/// # Returns
/// * `Result<Vec<u8>>` containing the list of extended attribute names if successful.
///
/// # Notes
/// - The returned vector contains null-terminated strings concatenated together.
/// - If the provided buffer size is too small, the function may return an error.
/// - Some filesystems may not support extended attributes, in which case this function
///   may return an empty list or an error.
pub fn listxattr(path: &Path, size: u32) -> Result<Vec<u8>, PosixError> {
    let c_path = cstring_from_path(path)?;
    let mut buf = vec![0u8; size as usize];
    let ret =
        unsafe { unix_impl::listxattr(c_path.as_ptr(), buf.as_mut_ptr() as *mut _, buf.len()) };

    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: listxattr failed",
            path.display()
        )));
    }

    buf.truncate(ret as usize);
    Ok(buf)
}

/// Removes an extended attribute from a file or directory.
///
/// This function is equivalent to the FUSE `removexattr` operation. It removes
/// the specified extended attribute from the file or directory at the given path.
///
/// # Arguments
/// * `path` - A reference to the `Path` of the file or directory.
/// * `name` - The name of the extended attribute to remove as an `OsStr`.
///
/// # Notes
/// - If the specified attribute does not exist, this function may return an error.
/// - Some filesystems may not support extended attributes, in which case this function
///   may return an error.
/// - Removing system-critical extended attributes may affect file system behavior.
pub fn removexattr(path: &Path, name: &OsStr) -> Result<(), PosixError> {
    let c_path = cstring_from_path(path)?;
    let c_name = CString::new(name.as_bytes()).map_err(|_| {
        PosixError::new(
            ErrorKind::InvalidArgument,
            format!(
                "{}: CString conversion failed in removexattr",
                Path::display(name.as_ref())
            ),
        )
    })?;

    let ret = unsafe { unix_impl::removexattr(c_path.as_ptr(), c_name.as_ptr()) };

    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: removexattr failed. Name: {}",
            path.display(),
            Path::display(name.as_ref())
        )));
    }

    Ok(())
}

/// Checks file accessibility based on the process's real user and group IDs.
///
/// This function is equivalent to the FUSE `access` operation.
///
/// It verifies whether the calling process can access the file specified by the path
/// according to the given access mask.
pub fn access(path: &Path, mask: AccessFlags) -> Result<(), PosixError> {
    let c_path = cstring_from_path(path)?;
    let ret = unsafe { libc::access(c_path.as_ptr(), mask.bits()) };
    if ret == -1 {
        return Err(PosixError::last_error(format!(
            "{}: access failed. Mask {:?}",
            path.display(),
            mask
        )));
    }
    Ok(())
}

/// Creates and opens a new file with specified permissions and flags.
///
/// This function is equivalent to the FUSE `create` operation.
///
/// It creates a new file if it doesn't exist, opens it with write access. It returns a file descriptor
/// which may not necessarily be equivalent to the FUSE file handle, along with its attributes.
/// An error is returned if the file already exists and the `O_EXCL` flag is set.
///
/// Although this function returns a Fd, it is guaranted to be positive and valid.
pub fn create(
    path: &Path,
    mode: u32,
    umask: u32,
    flags: OpenFlags,
) -> Result<(OwnedFd, FileAttribute), PosixError> {
    let c_path = cstring_from_path(path)?;
    let open_flags = flags.0;
    let final_mode = mode & !umask;

    // Ensure the file is opened with write access if not specified
    let open_flags = if open_flags & libc::O_ACCMODE == 0 {
        open_flags | libc::O_WRONLY
    } else {
        open_flags
    };

    // Open the file with O_CREAT (create if it does not exist)
    let fd = unsafe { libc::open(c_path.as_ptr(), open_flags | libc::O_CREAT, final_mode) };

    if fd == -1 {
        return Err(PosixError::last_error(format!(
            "{}: create failed",
            path.display()
        )));
    }

    Ok((unsafe { OwnedFd::from_raw_fd(fd) }, lookup(path)?))
}

/// Manipulates the allocated disk space for a file.
///
/// This function is equivalent to the FUSE `fallocate` operation.
///
/// It allows pre-allocation or deallocation of disk space for the file
/// referenced by the given file descriptor, starting at the specified offset
/// and extending for the given length.
pub fn fallocate(
    fd: BorrowedFd,
    offset: i64,
    length: i64,
    mode: FallocateFlags,
) -> Result<(), PosixError> {
    let result = unsafe { unix_impl::fallocate(fd.as_raw_fd(), mode.bits(), offset, length) };
    if result == -1 {
        return Err(PosixError::last_error(format!(
            "{:?}: fallocate failed",
            fd
        )));
    }
    Ok(())
}

/// Repositions the file offset of the open file descriptor.
///
/// This function is equivalent to the FUSE `lseek` operation.
///
/// It changes the file offset for the given file descriptor, based on the provided
/// offset and whence values. The new position is returned as a 64-bit integer.
pub fn lseek(fd: BorrowedFd, seek: SeekFrom) -> Result<i64, PosixError> {
    let (whence, offset) = match seek {
        SeekFrom::Start(offset) => (
            libc::SEEK_SET,
            i64::try_from(offset).map_err(|_| {
                PosixError::new(
                    ErrorKind::InvalidArgument,
                    "Offset too large for i64".to_string(),
                )
            })?,
        ),
        SeekFrom::Current(offset) => (libc::SEEK_CUR, offset),
        SeekFrom::End(offset) => (libc::SEEK_END, offset),
    };
    let result = unsafe { lseek_raw(fd.as_raw_fd(), offset, whence) };
    if result == -1 {
        return Err(PosixError::last_error(format!(
            "{:?}: lseek failed. Offset: {:?}, whence: {:?}",
            fd, offset, whence
        )));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    /*
    Some suggestions for additional tests:

    1. Test accessibility with different file permissions and user/group IDs.
    2. Test copy_file_range by copying data between two files.
    3. Test create by creating a new file with specific permissions and flags.
    4. Test fallocate by allocating space for a file and verifying the allocation.
    5. Test flush and fsync by writing data and ensuring it's persisted to disk.
    6. Test getxattr, listxattr, and setxattr by working with extended attributes.
    7. Test mknod by creating special files (e.g., named pipes).
    8. Test release by opening and then releasing file descriptors.
    9. Test setattr by modifying file attributes and verifying the changes.


    Additionally, we could consider:

    1. Testing edge cases and error conditions for existing tests.
    2. Adding more comprehensive tests for functions that are only indirectly tested (e.g., lseek).
    3. Testing functions with various input parameters to ensure they handle different scenarios correctly.
    */

    use tempfile::{NamedTempFile, TempDir};

    use super::*;
    use std::fs::{self, File};
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    #[test]
    fn test_convert_filetype() {
        let tmpfile = NamedTempFile::new().unwrap();
        let filetype = convert_filetype(fs::metadata(&tmpfile.path()).unwrap().file_type());
        assert_eq!(filetype, FileKind::RegularFile);
        drop(tmpfile);
    }

    #[test]
    fn test_convert_fileattribute() {
        let tmpfile = NamedTempFile::new().unwrap();
        fs::write(&tmpfile.path(), "blah").unwrap();
        let metadata = fs::metadata(&tmpfile.path()).unwrap();
        let attr = convert_fileattribute(metadata);

        assert!(attr.size > 0);
        drop(tmpfile);
    }

    #[test]
    fn test_system_time_to_timespec() {
        let system_time = SystemTime::now();
        let timespec = system_time_to_timespec(system_time).unwrap();

        assert!(timespec.tv_sec > 0);
        assert!(timespec.tv_nsec >= 0);
    }

    #[test]
    fn test_cstring_from_path() {
        let path = PathBuf::from("test_cstring");
        let c_string = cstring_from_path(&path).unwrap();

        assert_eq!(c_string.to_str().unwrap(), path.to_str().unwrap());
    }

    #[test]
    fn test_get_attr() {
        let tmpfile = NamedTempFile::new().unwrap();
        fs::write(&tmpfile.path(), "blah").unwrap();
        let attr1 = lookup(&tmpfile.path()).unwrap();
        let fd = open(&tmpfile.path(), OpenFlags(libc::O_RDONLY)).unwrap();
        let attr2 = getattr(fd.as_fd()).unwrap();
        assert!(attr1.size > 0);
        assert_eq!(attr1, attr2);
        drop(tmpfile);
    }

    #[test]
    fn test_readlink() {
        let tmpdir = TempDir::new().unwrap();

        let target_path = tmpdir.path().join("link_target");
        let _ = File::create_new(&target_path).unwrap();

        let symlink_path = tmpdir.path().join("symlink");
        symlink(&symlink_path, &target_path).unwrap();

        let link_target = readlink(&symlink_path).unwrap();
        fs::remove_file(&target_path).unwrap();
        fs::remove_file(&symlink_path).unwrap();
        drop(tmpdir);

        assert_eq!(Path::new(OsStr::from_bytes(&link_target)), target_path);
    }

    #[test]
    fn test_mkdir_and_rmdir() {
        let tmpdir = TempDir::new().unwrap();
        let dir_path = tmpdir.path().join("dir");
        mkdir(&dir_path, 0o755, 0).unwrap();
        assert!(dir_path.exists());

        rmdir(&dir_path).unwrap();
        assert!(!dir_path.exists());
        drop(tmpdir);
    }

    #[test]
    fn test_symlink() {
        let tmpdir = TempDir::new().unwrap();

        let target_path = tmpdir.path().join("link_target");
        let _ = File::create_new(&target_path).unwrap();

        let symlink_path = tmpdir.path().join("symlink");
        let attr = symlink(&symlink_path, &target_path).unwrap();

        fs::remove_file(&target_path).unwrap();
        fs::remove_file(&symlink_path).unwrap();
        drop(tmpdir);

        assert_eq!(attr.kind, FileKind::Symlink);
    }

    #[test]
    fn test_unlink() {
        let tmpdir = TempDir::new().unwrap();
        let file_path = tmpdir.path().join("file");
        File::create(&file_path).unwrap();

        assert!(&file_path.exists());
        unlink(&file_path).unwrap();
        assert!(!file_path.exists());
        drop(tmpdir);
    }

    #[test]
    fn test_rename() {
        let tmpdir = TempDir::new().unwrap();
        let src_path = tmpdir.path().join("src");
        File::create(&src_path).unwrap();
        let dest_path = tmpdir.path().join("dest");

        rename(&src_path, &dest_path, RenameFlags::empty()).unwrap();
        assert!(!src_path.exists());
        assert!(dest_path.exists());

        fs::remove_file(&dest_path).unwrap();
    }

    #[test]
    fn test_open() {
        let tmpfile = NamedTempFile::new().unwrap();

        let fd = open(&tmpfile.path(), OpenFlags(0)).unwrap();
        assert!(fd.as_raw_fd() > 0);
        drop(tmpfile);
    }

    #[test]
    fn test_read() {
        let tmpfile = NamedTempFile::new().unwrap();
        fs::write(&tmpfile.path(), b"Hello, world!").unwrap();

        let fd = open(&tmpfile.path(), OpenFlags(libc::O_RDONLY)).unwrap();
        let result = read(fd.as_fd(), SeekFrom::Current(0), 5).unwrap();
        assert_eq!(result, b"Hello");

        let result = read(fd.as_fd(), SeekFrom::Current(0), 5).unwrap();
        assert_eq!(result, b"Hello");

        // Attempt to read past the end of the file
        let result = read(fd.as_fd(), SeekFrom::Current(50), 10);
        assert!(!result.is_err());
        assert_eq!(result.unwrap().len(), 0);
        drop(tmpfile);
    }

    #[test]
    fn test_write() {
        let tmpfile = NamedTempFile::new().unwrap();
        let fd = open(&tmpfile.path(), OpenFlags(libc::O_RDWR)).unwrap();

        // Write data to the file
        let bytes_written = write(fd.as_fd(), SeekFrom::Current(0), b"Hello, world!").unwrap();
        assert_eq!(bytes_written, 13);

        // Verify written content
        let content = read(fd.as_fd(), SeekFrom::Start(0), 100).unwrap();
        assert_eq!(&String::from_utf8(content).unwrap(), "Hello, world!");

        // Overwrite part of the file
        let bytes_written = write(fd.as_fd(), SeekFrom::Current(7), b"Rustaceans!").unwrap();
        assert_eq!(bytes_written, 11);

        let content = read(fd.as_fd(), SeekFrom::Start(0), 100).unwrap();
        assert_eq!(&String::from_utf8(content).unwrap(), "Hello, Rustaceans!");
        drop(tmpfile);
    }

    #[test]
    fn test_readdir() {
        let tmpdir = TempDir::new().unwrap();
        let file1 = tmpdir.path().join("file1");
        File::create(&file1).unwrap();

        let entries = readdir(&tmpdir.path()).unwrap();
        assert!(entries.iter().any(|(name, _)| name == Path::new("file1")));

        fs::remove_file(&file1).unwrap();
        drop(tmpdir);
    }

    #[test]
    fn test_statfs() {
        let dir_path = Path::new("/tmp");
        let stat = statfs(dir_path).unwrap();

        assert!(stat.total_blocks > 0);
        assert!(stat.block_size > 0);
    }

    #[test]
    fn test_lseek() {
        use std::io::Write;

        let tmpfile = NamedTempFile::new().unwrap();
        let path = tmpfile.path().to_path_buf();

        // Write some data to the file
        {
            let mut file = File::create(&path).unwrap();
            file.write_all(b"Hello, World!").unwrap();
        }

        let fd = open(&path, OpenFlags(libc::O_RDWR)).unwrap();
        let borrowed_fd = fd.as_fd();

        // Test SeekFrom::Start
        let new_pos = lseek(borrowed_fd, SeekFrom::Start(7)).unwrap();
        assert_eq!(new_pos, 7);

        // Read to verify position
        let buffer = read(borrowed_fd, SeekFrom::Current(0), 6).unwrap();
        assert_eq!(buffer, b"World!");

        // Read again, because read does not update position
        let buffer = read(borrowed_fd, SeekFrom::Current(0), 6).unwrap();
        assert_eq!(buffer, b"World!");

        // Test SeekFrom::Current
        let new_pos = lseek(borrowed_fd, SeekFrom::Current(-6)).unwrap();
        assert_eq!(new_pos, 1);

        // Read to verify position
        let buffer = read(borrowed_fd, SeekFrom::Current(-1), 5).unwrap();
        assert_eq!(buffer, b"Hello");

        // Test SeekFrom::End
        let new_pos = lseek(borrowed_fd, SeekFrom::End(-5)).unwrap();
        assert_eq!(new_pos, 8);

        // Read to verify position
        let buffer = read(borrowed_fd, SeekFrom::Current(0), 5).unwrap();
        assert_eq!(buffer, b"orld!");

        // Test seeking beyond file size
        let new_pos = lseek(borrowed_fd, SeekFrom::Start(20)).unwrap();
        assert_eq!(new_pos, 20);

        // Attempt to read from beyond file size
        let result = read(borrowed_fd, SeekFrom::Current(0), 5).unwrap();
        assert_eq!(result.len(), 0);

        drop(tmpfile);
    }
}
