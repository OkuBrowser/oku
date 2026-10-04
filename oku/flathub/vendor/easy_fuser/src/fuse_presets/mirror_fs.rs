/*!
# MirrorFs

A FUSE (Filesystem in Userspace) handler that mirrors the content of another folder in either read-only or read-write mode.

## Overview

The `MirrorFs` struct implements the `FuseHandler` trait, providing a way to create a mirror of an existing filesystem. It comes in two variants:

1. `MirrorFsReadOnly`: A read-only version that only allows read operations on the mirrored content.
2. `MirrorFs`: A read-write version that allows both read and write operations on the mirrored content.

## Implementation Details

- Both variants use a `std::path:: PathBuf` to represent the repository path they're mirroring.
- They wrap another `FuseHandler<std::path:: PathBuf>` implementation, allowing for composition of filesystem behaviors.
- Most FUSE operations are implemented by translating paths and delegating to the `unix_fs` module.
- The implementation uses macros to define common methods for both read-only and read-write variants.

## Usage

To use these handlers:

1. Create a new `MirrorFsReadOnly` or `MirrorFs` instance by providing a repository path and an inner `FuseHandler<std::path:: PathBuf>` implementation:

   ```text
   let read_only_fs = MirrorFsReadOnly::new(repo_path, inner_handler);
   // or
   let read_write_fs = MirrorFs::new(repo_path, inner_handler);
   ```

2. Use the resulting MirrorFsReadOnly or MirrorFs as your FUSE handler.

3. Alternatively, you can use MirrorFs or MirrorFsReadOnly as delegators in your own FUSE implementation (see FuseHandler documentation for more details).

## Unimplemented Functions
The following FUSE operations are not implemented in either variant:

- link
- setlk
- getlk
- bmap
- ioctl


## Important Note
This implementation does not include safeguards against recursive mounting scenarios. Users should be cautious when choosing mount points to avoid potential system hangs.

For example, if the MirrorFs is set up like this:
```text
let fs = MirrorFs::new("/my_repo");
mount(fs, "/my_repo/mountpoint")
```

Operations like ls /my_repo/mountpoint could cause the system to hang indefinitely. This occurs because the filesystem would repeatedly try to access its own mountpoint, creating an endless loop.

Specifically, operations such as lstat (used in lookup, getattr, and ls commands) can trigger this recursive behavior when a child directory in the mirrored filesystem is also a parent in the actual filesystem hierarchy.

To avoid this issue, ensure that the mountpoint is not located within the mirrored repository.

## Read-Only vs Read-Write
- MirrorFsReadOnly: This variant only implements methods for reading and accessing file metadata. It does not allow any modifications to the mirrored filesystem.
- MirrorFs: This variant implements all methods from MirrorFsReadOnly plus additional methods for modifying the filesystem, such as creating, deleting, and modifying files and directories.


## Note
For more specific implementations or to extend functionality, you can modify these handlers or use them as a reference for implementing your own FuseHandler.

If you intend to enforce read-only at the fuse level,
prefer the usage of option `MountOption::RO` instead of `MirrorFsReadOnly`.
*/


use std::path::Path;

use super::fd_handler_helper::*;
use crate::types::*;
use crate::unix_fs;

macro_rules! mirror_fs_readonly_methods {
    () => {
        pub fn access(&self, _req: &RequestInfo, file_id: std::path:: PathBuf, mask: AccessFlags) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::access(&file_path, mask)
        }

        pub fn getattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            _file_handle: Option<BorrowedFileHandle>,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            unix_fs::lookup(&file_path)
        }

        pub fn getxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::getxattr(&file_path, name, size)
        }

        pub fn listxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            size: u32,
        ) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::listxattr(&file_path, size)
        }

        pub fn lookup(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::lookup(&file_path)
        }

        pub fn open(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
            let file_path = self.source_path.join(file_id);
            let fd = unix_fs::open(file_path.as_ref(), flags)?;
            // Open by definition returns positive Fd or error
            let file_handle = OwnedFileHandle::from_owned_fd(fd).unwrap();
            Ok((file_handle, FopenFlags::empty()))
        }

        pub fn readdir(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            _file_handle: BorrowedFileHandle,
        ) -> FuseResult<Vec<(std::ffi::OsString, FileKind)>> {
            let folder_path = self.source_path.join(file_id);
            let children = unix_fs::readdir(folder_path.as_ref())?;
            let mut result = Vec::new();
            result.push((std::ffi::OsString::from("."), FileKind::Directory));
            result.push((std::ffi::OsString::from(".."), FileKind::Directory));
            for (child_name, child_kind) in children {
                result.push((child_name, child_kind));
            }
            Ok(result)
        }

        pub fn readlink(&self, _req: &RequestInfo, file_id: std::path:: PathBuf) -> FuseResult<Vec<u8>> {
            let file_path = self.source_path.join(file_id);
            unix_fs::readlink(&file_path)
        }

        pub fn statfs(&self, _req: &RequestInfo, file_id: std::path:: PathBuf) -> FuseResult<StatFs> {
            let file_path = self.source_path.join(file_id);
            unix_fs::statfs(&file_path)
        }
    };
}

macro_rules! mirror_fs_readwrite_methods {
    () => {
        pub fn create(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
            flags: OpenFlags,
        ) -> FuseResult<(OwnedFileHandle, FileAttribute, FopenFlags)> {
            let file_path = self.source_path.join(parent_id).join(name);
            let (fd, file_attr) = unix_fs::create(&file_path, mode, umask, flags)?;
            // Open by definition returns positive Fd or error
            let file_handle = OwnedFileHandle::from_owned_fd(fd).unwrap();
            Ok((file_handle, file_attr, FopenFlags::empty()))
        }

        pub fn mkdir(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mkdir(&file_path, mode, umask)
        }

        pub fn mknod(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            mode: u32,
            umask: u32,
            rdev: DeviceType,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::mknod(&file_path, mode, umask, rdev)
        }

        pub fn removexattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::removexattr(&file_path, name)
        }

        pub fn rename(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            newparent: std::path:: PathBuf,
            newname: &std::ffi::OsStr,
            flags: RenameFlags,
        ) -> FuseResult<()> {
            let oldpath = self.source_path.join(parent_id).join(name);
            let newpath = self.source_path.join(newparent).join(newname);
            unix_fs::rename(&oldpath, &newpath, flags)
        }

        pub fn rmdir(&self, _req: &RequestInfo, parent_id: std::path:: PathBuf, name: &std::ffi::OsStr) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::rmdir(&file_path)
        }

        pub fn setattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            attrs: SetAttrRequest,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setattr(&file_path, attrs)
        }

        pub fn setxattr(
            &self,
            _req: &RequestInfo,
            file_id: std::path:: PathBuf,
            name: &std::ffi::OsStr,
            value: Vec<u8>,
            flags: SetXAttrFlags,
            position: u32,
        ) -> FuseResult<()> {
            let file_path = self.source_path.join(file_id);
            unix_fs::setxattr(&file_path, name, &value, flags, position)
        }

        pub fn symlink(
            &self,
            _req: &RequestInfo,
            parent_id: std::path:: PathBuf,
            link_name: &std::ffi::OsStr,
            target: &std::path::Path,
        ) -> FuseResult<FileAttribute> {
            let file_path = self.source_path.join(parent_id).join(link_name);
            unix_fs::symlink(&file_path, target)
        }

        pub fn unlink(&self, _req: &RequestInfo, parent_id: std::path:: PathBuf, name: &std::ffi::OsStr) -> FuseResult<()> {
            let file_path = self.source_path.join(parent_id).join(name);
            unix_fs::unlink(&file_path)
        }
    };
}

pub trait MirrorFsTrait {
    fn new(source_path: std::path:: PathBuf) -> Self;

    fn source_dir(&self) -> &Path;
}

/// Specific documentation is located in parent module documentation.
pub struct MirrorFs {
    source_path: std::path:: PathBuf,
}

impl MirrorFsTrait for MirrorFs {
    fn new(source_path: std::path:: PathBuf) -> Self {
        Self { source_path }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl MirrorFs {
    mirror_fs_readonly_methods!();
    mirror_fs_readwrite_methods!();
    fd_handler_readonly_methods!(std::path::PathBuf);
    fd_handler_readwrite_methods!(std::path::PathBuf);
}

/// Specific documentation is located in parent module documentation.
pub struct MirrorFsReadOnly {
    source_path: std::path:: PathBuf,
}

impl MirrorFsTrait for MirrorFsReadOnly {
    fn new(source_path: std::path:: PathBuf) -> Self {
        Self { source_path }
    }

    fn source_dir(&self) -> &Path {
        self.source_path.as_path()
    }
}

impl MirrorFsReadOnly {
    mirror_fs_readonly_methods!();
    fd_handler_readonly_methods!(std::path::PathBuf);
}
