use std::{
    ffi::{OsStr, OsString}, marker::PhantomData, path::Path
};

use crate::types::*;

/**
# DefaultFuseHandler

A default skeleton implementation for a FUSE (Filesystem in Userspace) handler. This struct provides a basic framework for implementing a custom filesystem.

## Overview

The `DefaultFuseHandler` implements the `FuseHandler` trait, providing default implementations for all FUSE operations. Most of these default implementations will return a "Not Implemented" error or panic, depending on the configuration.

## Default Implementations

The following functions are implemented with default responses, so they don't need to be explicitly implemented in derived handlers:

- `init`: Returns `Ok(())`.
- `opendir`: Returns a `OwnedFileHandle` with value 0 and empty `FUSEOpenResponseFlags`. Only safe because releasedir don't use the file handle
- `releasedir`: Returns `Ok(())`.
- `fsyncdir`: Returns `Ok(())`.
- `statfs`: Returns `StatFs::default()`.

## Usage

To use this handler, either:

1. Compose it with a more specific implementation, such as `MirrorFs`, which can use `DefaultFuseHandler` as its inner handler.
2. Use it as a reference for implementing your own `FuseHandler`.

## Configuration

The `DefaultFuseHandler` can be configured to either return errors or panic when unimplemented methods are called:

- `DefaultFuseHandler::new()`: Creates a handler that returns "Not Implemented" errors.
- `DefaultFuseHandler::new_with_panic()`: Creates a handler that panics on unimplemented methods.

## Note

This is a basic skeleton. For more complete implementations, refer to the templates provided in the library.
*/
pub struct DefaultFuseHandler<TId> {
    handling: HandlingMethod,
    phantom: PhantomData<TId>
}

enum HandlingMethod {
    Panic,
    Error(ErrorKind),
}

impl<TId: FileIdType> DefaultFuseHandler<TId> {
    /// Creates a new `DefaultFuseHandler` that returns "Not Implemented" errors for each unimplemented FUSE call.
    ///
    /// This is useful for gradually implementing FUSE operations, as it allows the filesystem to
    /// function (albeit with limited capabilities) even when not all operations are implemented.
    pub fn new() -> Self {
        DefaultFuseHandler {
            handling: HandlingMethod::Error(ErrorKind::FunctionNotImplemented),
            phantom: PhantomData,
        }
    }

    /// Creates a new `DefaultFuseHandler` that panics for each unimplemented FUSE call.
    ///
    /// This is useful for debugging purposes, as it immediately highlights which FUSE operations
    /// are being called but not yet implemented.
    pub fn new_with_panic() -> Self {
        DefaultFuseHandler {
            handling: HandlingMethod::Panic,
            phantom: PhantomData,
        }
    }

    /// Creates a new `DefaultFuseHandler` that returns a custom error for each unimplemented FUSE call.
    ///
    /// This is useful to give a different message for the user, like PermissionDenied.
    pub fn new_with_custom_error(error_kind: ErrorKind) -> Self {
        DefaultFuseHandler {
            handling: HandlingMethod::Error(error_kind),
            phantom: PhantomData,
        }
    }

    pub fn access(&self, _req: &RequestInfo, file_id: TId, mask: AccessFlags) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("access(file_id: {}, mask: {:?})", file_id.display(), mask)
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] access(file_id: {}, mask: {:?})",
                file_id.display(),
                mask
            ),
        }
    }

    pub fn bmap(&self, _req: &RequestInfo, file_id: TId, blocksize: u32, idx: u64) -> FuseResult<u64> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "bmap(file_id: {}, blocksize: {}, idx: {})",
                        file_id.display(),
                        blocksize,
                        idx
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] bmap(file_id: {}, blocksize: {}, idx: {})",
                file_id.display(),
                blocksize,
                idx
            ),
        }
    }

    pub fn copy_file_range(
        &self,
        _req: &RequestInfo,
        file_in: TId,
        file_handle_in: BorrowedFileHandle,
        offset_in: u64,
        file_out: TId,
        file_handle_out: BorrowedFileHandle,
        offset_out: u64,
        len: u64,
        flags: CopyFileRangeFlags,
    ) -> FuseResult<u32> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "copy_file_range(file_in: {}, file_handle_in: {:?}, offset_in: {}, file_out: {}, file_handle_out: {:?}, offset_out: {}, len: {}, flags: {:?})",
                        file_in.display(),
                        file_handle_in,
                        offset_in,
                        file_out.display(),
                        file_handle_out,
                        offset_out,
                        len,
                        flags
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] copy_file_range(file_in: {}, file_handle_in: {:?}, offset_in: {}, file_out: {}, file_handle_out: {:?}, offset_out: {}, len: {}, flags: {:?})",
                file_in.display(),
                file_handle_in,
                offset_in,
                file_out.display(),
                file_handle_out,
                offset_out,
                len,
                flags
            ),
        }
    }

    pub fn create(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, TId::Metadata, FopenFlags)> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "create(parent_id: {}, name: {:?}, mode: {}, umask: {}, flags: {:?})",
                        parent_id.display(),
                        name,
                        mode,
                        umask,
                        flags
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] create(parent_id: {}, name: {:?}, mode: {}, umask: {}, flags: {:?})",
                parent_id.display(),
                name,
                mode,
                umask,
                flags
            ),
        }
    }

    pub fn fallocate(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        offset: i64,
        length: i64,
        mode: FallocateFlags,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "fallocate(file_id: {}, file_handle: {:?}, offset: {}, length: {}, mode: {:?})",
                        file_id.display(),
                        file_handle,
                        offset,
                        length,
                        mode
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] fallocate(file_id: {}, file_handle: {:?}, offset: {}, length: {}, mode: {:?})",
                file_id.display(),
                file_handle,
                offset,
                length,
                mode
            ),
        }
    }

    pub fn flush(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        lock_owner: u64,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "flush(file_id: {}, file_handle: {:?}, lock_owner: {})",
                        file_id.display(),
                        file_handle,
                        lock_owner
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] flush(file_id: {}, file_handle: {:?}, lock_owner: {})",
                file_id.display(),
                file_handle,
                lock_owner
            ),
        }
    }

    pub fn forget(&self, _req: &RequestInfo, _file_id: TId, _nlookup: u64) {}

    pub fn fsync(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "fsync(file_id: {}, file_handle: {:?}, datasync: {})",
                        file_id.display(),
                        file_handle,
                        datasync
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] fsync(file_id: {}, file_handle: {:?}, datasync: {})",
                file_id.display(),
                file_handle,
                datasync
            ),
        }
    }

    pub fn fsyncdir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _file_handle: BorrowedFileHandle,
        _datasync: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    pub fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: Option<BorrowedFileHandle>,
    ) -> FuseResult<FileAttribute> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "getattr(file_id: {}, file_handle: {:?})",
                        file_id.display(),
                        file_handle
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] getattr(file_id: {}, file_handle: {:?})",
                file_id.display(),
                file_handle
            ),
        }
    }

    pub fn getlk(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "getlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?})",
                        file_id.display(),
                        file_handle,
                        lock_owner,
                        lock_info
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] getlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?})",
                file_id.display(),
                file_handle,
                lock_owner,
                lock_info
            ),
        }
    }

    pub fn getxattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "getxattr(file_id: {}, name: {:?}, size: {})",
                        file_id.display(),
                        name,
                        size
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] getxattr(file_id: {}, name: {:?}, size: {})",
                file_id.display(),
                name,
                size
            ),
        }
    }

    pub fn ioctl(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        flags: IoctlFlags,
        cmd: u32,
        in_data: Vec<u8>,
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "ioctl(file_id: {}, file_handle: {:?}, flags: {:?}, cmd: {}, in_data: {:?}, out_size: {})",
                        file_id.display(),
                        file_handle,
                        flags,
                        cmd,
                        in_data,
                        out_size
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] ioctl(file_id: {}, file_handle: {:?}, flags: {:?}, cmd: {}, in_data: {:?}, out_size: {})",
                file_id.display(),
                file_handle,
                flags,
                cmd,
                in_data,
                out_size
            ),
        }
    }

    pub fn link(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        newparent: TId,
        newname: &OsStr,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "link(file_id: {}, newparent: {}, newname: {:?})",
                        file_id.display(),
                        newparent.display(),
                        Path::new(newname)
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] link(file_id: {}, newparent: {}, newname: {:?})",
                file_id.display(),
                newparent.display(),
                Path::new(newname)
            ),
        }
    }

    pub fn listxattr(&self, _req: &RequestInfo, file_id: TId, size: u32) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("listxattr(file_id: {}, size: {})", file_id.display(), size)
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] listxattr(file_id: {}, size: {})",
                file_id.display(),
                size
            ),
        }
    }

    pub fn lookup(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "lookup(parent_file: {}, name {:?})",
                        parent_id.display(),
                        Path::display(name.as_ref())
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!("[Not Implemented] lookup"),
        }
    }

    pub fn lseek(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        seek: SeekFrom,
    ) -> FuseResult<i64> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "lseek(file_id: {}, file_handle: {:?}, seek: {:?})",
                        file_id.display(),
                        file_handle,
                        seek
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] lseek(file_id: {}, file_handle: {:?}, seek: {:?})",
                file_id.display(),
                file_handle,
                seek
            ),
        }
    }

    pub fn mkdir(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "mkdir(parent_id: {}, name: {:?}, mode: {}, umask: {})",
                        parent_id.display(),
                        Path::new(name),
                        mode,
                        umask
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] mkdir(parent_id: {}, name: {:?}, mode: {}, umask: {})",
                parent_id.display(),
                Path::new(name),
                mode,
                umask
            ),
        }
    }

    pub fn mknod(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "mknod(parent_id: {}, name: {:?}, mode: {}, umask: {}, rdev: {:?})",
                        parent_id.display(),
                        Path::new(name),
                        mode,
                        umask,
                        rdev
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] mknod(parent_id: {}, name: {:?}, mode: {}, umask: {}, rdev: {:?})",
                parent_id.display(),
                Path::new(name),
                mode,
                umask,
                rdev
            ),
        }
    }

    pub fn open(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("open(file_id: {}, flags: {:?})", file_id.display(), flags)
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] open(file_id: {}, flags: {:?})",
                file_id.display(),
                flags
            ),
        }
    }

    pub fn opendir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
        // Safe because in releasedir we don't use it
        Ok((
            unsafe { OwnedFileHandle::from_raw(0) },
            FopenFlags::empty(),
        ))
    }

    pub fn read(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        seek: SeekFrom,
        size: u32,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "read(file_id: {}, file_handle: {:?}, seek: {:?}, size: {}, flags: {:?}, lock_owner: {:?})",
                        file_id.display(),
                        file_handle,
                        seek,
                        size,
                        flags,
                        lock_owner
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] read(file_id: {}, file_handle: {:?}, seek: {:?}, size: {}, flags: {:?}, lock_owner: {:?})",
                file_id.display(),
                file_handle,
                seek,
                size,
                flags,
                lock_owner
            ),
        }
    }

    pub fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
    ) -> FuseResult<Vec<(OsString, TId::MinimalMetadata)>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "readdir(file_id: {}, file_handle: {:?})",
                        file_id.display(),
                        file_handle
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] readdir(file_id: {}, file_handle: {:?})",
                file_id.display(),
                file_handle
            ),
        }
    }

    pub fn readdirplus(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
    ) -> FuseResult<Vec<(OsString, TId::Metadata)>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "readdirplus(file_id: {}, file_handle: {:?})",
                        file_id.display(),
                        file_handle
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] readdirplus(file_id: {}, file_handle: {:?})",
                file_id.display(),
                file_handle
            ),
        }
    }

    pub fn readlink(&self, _req: &RequestInfo, file_id: TId) -> FuseResult<Vec<u8>> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!("readlink(file_id: {})", file_id.display())
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => {
                panic!("[Not Implemented] readlink(file_id: {})", file_id.display())
            }
        }
    }

    pub fn release(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: OwnedFileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "release(file_id: {}, file_handle: {:?}, flags: {:?}, lock_owner: {:?}, flush: {})",
                        file_id.display(),
                        file_handle,
                        flags,
                        lock_owner,
                        flush
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] release(file_id: {}, file_handle: {:?}, flags: {:?}, lock_owner: {:?}, flush: {})",
                file_id.display(),
                file_handle,
                flags,
                lock_owner,
                flush
            ),
        }
    }

    pub fn releasedir(
        &self,
        _req: &RequestInfo,
        _file_id: TId,
        _file_handle: OwnedFileHandle,
        _flags: OpenFlags,
    ) -> FuseResult<()> {
        Ok(())
    }

    pub fn removexattr(&self, _req: &RequestInfo, file_id: TId, name: &OsStr) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "removexattr(file_id: {}, name: {:?})",
                        file_id.display(),
                        name
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] removexattr(file_id: {}, name: {:?})",
                file_id.display(),
                name
            ),
        }
    }

    pub fn rename(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        name: &OsStr,
        newparent: TId,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "rename(parent_id: {}, name: {:?}, newparent: {}, newname: {:?}, flags: {:?})",
                        parent_id.display(),
                        Path::new(name),
                        newparent.display(),
                        Path::new(newname),
                        flags
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] rename(parent_id: {}, name: {:?}, newparent: {}, newname: {:?}, flags: {:?})",
                parent_id.display(),
                Path::new(name),
                newparent.display(),
                Path::new(newname),
                flags
            ),
        }
    }

    pub fn rmdir(&self, _req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "rmdir(parent_id: {}, name: {:?})",
                        parent_id.display(),
                        Path::new(name)
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] rmdir(parent_id: {}, name: {:?})",
                parent_id.display(),
                Path::new(name)
            ),
        }
    }

    pub fn setattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "setattr(file_id: {}, attrs: {:?})",
                        file_id.display(),
                        attrs
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] setattr(file_id: {}, attrs: {:?})",
                file_id.display(),
                attrs
            ),
        }
    }

    pub fn setlk(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "setlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?}, sleep: {})",
                        file_id.display(),
                        file_handle,
                        lock_owner,
                        lock_info,
                        sleep
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] setlk(file_id: {}, file_handle: {:?}, lock_owner: {}, lock_info: {:?}, sleep: {})",
                file_id.display(),
                file_handle,
                lock_owner,
                lock_info,
                sleep
            ),
        }
    }

    pub fn setxattr(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        name: &OsStr,
        _value: Vec<u8>,
        flags: SetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "setxattr(file_id: {}, name: {:?}, flags: {:?}, position: {})",
                        file_id.display(),
                        name,
                        flags,
                        position
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] setxattr(file_id: {}, name: {:?}, flags: {:?}, position: {})",
                file_id.display(),
                name,
                flags,
                position
            ),
        }
    }

    pub fn statfs(&self, _req: &RequestInfo, _file_id: TId) -> FuseResult<StatFs> {
        Ok(StatFs::default())
    }

    pub fn symlink(
        &self,
        _req: &RequestInfo,
        parent_id: TId,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<TId::Metadata> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "symlink(parent_id: {}, link_name: {:?}, target: {:?})",
                        parent_id.display(),
                        Path::new(link_name),
                        target
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] symlink(parent_id: {}, link_name: {:?}, target: {:?})",
                parent_id.display(),
                Path::new(link_name),
                target
            ),
        }
    }

    pub fn unlink(&self, _req: &RequestInfo, parent_id: TId, name: &OsStr) -> FuseResult<()> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "unlink(parent_id: {}, name: {:?})",
                        parent_id.display(),
                        Path::new(name)
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] unlink(parent_id: {}, name: {:?})",
                parent_id.display(),
                Path::new(name)
            ),
        }
    }

    pub fn write(
        &self,
        _req: &RequestInfo,
        file_id: TId,
        file_handle: BorrowedFileHandle,
        seek: SeekFrom,
        data: Vec<u8>,
        write_flags: WriteFlags,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        match self.handling {
            HandlingMethod::Error(kind) => Err(PosixError::new(
                kind,
                if cfg!(debug_assertions) {
                    format!(
                        "write(file_id: {}, file_handle: {:?}, seek: {:?}, data_len: {}, write_flags: {:?}, flags: {:?}, lock_owner: {:?})",
                        file_id.display(),
                        file_handle,
                        seek,
                        data.len(),
                        write_flags,
                        flags,
                        lock_owner
                    )
                } else {
                    String::new()
                },
            )),
            HandlingMethod::Panic => panic!(
                "[Not Implemented] write(file_id: {}, file_handle: {:?}, seek: {:?}, data_len: {}, write_flags: {:?}, flags: {:?}, lock_owner: {:?})",
                file_id.display(),
                file_handle,
                seek,
                data.len(),
                write_flags,
                flags,
                lock_owner
            ),
        }
    }
}
