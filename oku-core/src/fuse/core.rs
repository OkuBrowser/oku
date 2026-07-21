use crate::fs::util::normalise_path;
use crate::fs::OkuFs;
use async_trait::async_trait;
use easy_fuser::fuse_async::prelude::*;
use easy_fuser::fuse_async::FuseHandler;
use easy_fuser::fuse_presets::DefaultFuseHandler;
use easy_fuser::types::BorrowedFileHandle;
use easy_fuser::types::ErrorKind;
use easy_fuser::types::FileAttribute;
use easy_fuser::types::FileIdType;
use easy_fuser::types::FuseResult;
use easy_fuser::types::OpenFlags;
use easy_fuser::types::OwnedFileHandle;
use easy_fuser::types::PosixError;
use easy_fuser::types::RequestInfo;
use easy_fuser::types::StatFs;
use log::debug;
use log::error;
use log::info;
use log::trace;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;

impl OkuFs {
    fn get_inner_fuse_handler(&self) -> &DefaultFuseHandler<PathBuf> {
        &self.fuse_handler
    }
}

#[async_trait]
impl FuseHandler for OkuFs {
    type TId = PathBuf;

    fn destroy(&self) {
        if let Some(handle) = &self.handle {
            handle.block_on(async move { self.clone().shutdown().await });
        }
        info!("Node unmounting … ");
    }

    async fn flush(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        lock_owner: u64,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[flush] file_id = {file_id:?}, file_handle = {file_handle:?}, lock_owner = {lock_owner}");
        self.get_inner_fuse_handler()
            .flush(req, file_id, file_handle, lock_owner)
    }

    async fn fsync(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        datasync: bool,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!(
            "[fsync] file_id = {file_id:?}, file_handle = {file_handle:?}, datasync = {datasync}"
        );
        self.get_inner_fuse_handler()
            .fsync(req, file_id, file_handle, datasync)
    }

    async fn access(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        mask: AccessFlags,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[access] file_id = {file_id:?}, mask = {mask:?}");
        self.get_inner_fuse_handler().access(req, file_id, mask)
    }

    async fn statfs(&self, _req: &RequestInfo, file_id: Self::TId) -> FuseResult<StatFs> {
        let file_id = normalise_path(&file_id);
        trace!("[statfs] file_id = {file_id:?}");
        self.get_fs_entry_stats(&file_id).await.map_err(|e| {
            error!("[statfs]: {e}");
            PosixError::new(ErrorKind::InputOutputError, e.to_string())
        })
    }

    async fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: Self::TId,
        file_handle: Option<BorrowedFileHandle<'_>>,
    ) -> FuseResult<FileAttribute> {
        let file_id = normalise_path(&file_id);
        debug!("[getattr] file_id = {file_id:?}, file_handle = {file_handle:?}");
        self.getattr(file_id).await.map_err(|e| {
            error!("[getattr]: {e}");
            PosixError::new(ErrorKind::FileNotFound, e.to_string())
        })
    }

    async fn read(
        &self,
        _req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        seek: SeekFrom,
        size: u32,
        flags: OpenFlags,
        lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let file_id = normalise_path(&file_id);
        debug!(
            "[read] file_id = {file_id:?}, file_handle = {file_handle:?}, seek = {seek:?}, size = {size}, flags = {flags:?}, lock_owner = {lock_owner:?}"
        );
        self.read(file_id, seek, size).await.map_err(|e| {
            error!("[read]: {e}");
            PosixError::new(ErrorKind::InputOutputError, e.to_string())
        })
    }

    async fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
    ) -> FuseResult<Vec<(OsString, <Self::TId as FileIdType>::MinimalMetadata)>> {
        let file_id = normalise_path(&file_id);
        debug!("[readdir] file_id = {file_id:?}, file_handle = {file_handle:?}");
        self.readdir(file_id).await.map_err(|e| {
            error!("[readdir]: {e}");
            PosixError::new(ErrorKind::InputOutputError, e.to_string())
        })
    }

    async fn rmdir(
        &self,
        _req: &RequestInfo,
        parent_id: Self::TId,
        name: &OsStr,
    ) -> FuseResult<()> {
        let parent_id = normalise_path(&parent_id);
        debug!("[rmdir] parent_id = {parent_id:?}, name = {name:?}");
        self.rmdir(parent_id, name).await.map_err(|e| {
            error!("[rmdir]: {e}");
            PosixError::new(ErrorKind::InputOutputError, e.to_string())
        })
    }

    async fn create(
        &self,
        req: &RequestInfo,
        parent_id: Self::TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(
        OwnedFileHandle,
        <Self::TId as FileIdType>::Metadata,
        FopenFlags,
    )> {
        let parent_id = normalise_path(&parent_id);
        debug!("[create] parent_id = {parent_id:?}, name = {name:?}, mode = {mode:#06o}, umask = {umask:#06o}, flags = {flags:?}");
        self.create(req, parent_id, name, mode, umask, flags)
            .await
            .map_err(|e| {
                error!("[create]: {e}");
                PosixError::new(ErrorKind::InputOutputError, e.to_string())
            })
    }

    async fn lookup(
        &self,
        req: &RequestInfo,
        parent_id: Self::TId,
        name: &OsStr,
    ) -> FuseResult<<Self::TId as FileIdType>::Metadata> {
        let parent_id = normalise_path(&parent_id);
        debug!("[lookup] parent_id = {parent_id:?}, name = {name:?}");
        FuseHandler::getattr(self, req, parent_id.join(name), None).await
    }

    async fn readdirplus(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
    ) -> FuseResult<Vec<(OsString, <Self::TId as FileIdType>::Metadata)>> {
        let file_id = normalise_path(&file_id);
        debug!("[readdirplus] file_id = {file_id:?}, file_handle = {file_handle:?}");
        let readdir_result = FuseHandler::readdir(self, req, file_id.clone(), file_handle).await?;
        let mut result = Vec::with_capacity(readdir_result.len());
        for (name, _) in readdir_result.into_iter() {
            let metadata = self.lookup(req, file_id.clone(), &name).await?;
            result.push((name, metadata));
        }
        Ok(result)
    }

    async fn rename(
        &self,
        _req: &RequestInfo,
        parent_id: Self::TId,
        name: &OsStr,
        newparent: Self::TId,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        let parent_id = normalise_path(&parent_id);
        let newparent = normalise_path(&newparent);
        debug!("[rename] parent_id = {parent_id:?}, name = {name:?}, newparent = {newparent:?}, newname = {newname:?}, flags = {flags:?}");
        self.rename(parent_id, name, newparent, newname)
            .await
            .map_err(|e| {
                error!("[rename]: {e}");
                PosixError::new(ErrorKind::InputOutputError, e.to_string())
            })
    }

    async fn write(
        &self,
        _req: &RequestInfo,
        file_id: Self::TId,
        _file_handle: BorrowedFileHandle<'_>,
        seek: SeekFrom,
        data: Vec<u8>,
        write_flags: WriteFlags,
        flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        let file_id = normalise_path(&file_id);
        let data_len = data.len();
        debug!("[write] file_id = {file_id:?}, seek = {seek:?}, data_len = {data_len}, write_flags = {write_flags:?}, flags = {flags:?}");
        self.write(file_id, seek, data).await.map_err(|e| {
            error!("[write]: {e}");
            PosixError::new(ErrorKind::InputOutputError, e.to_string())
        })
    }

    async fn unlink(
        &self,
        _req: &RequestInfo,
        parent_id: Self::TId,
        name: &OsStr,
    ) -> FuseResult<()> {
        let parent_id = normalise_path(&parent_id);
        debug!("[unlink] parent_id = {parent_id:?}, name = {name:?}");
        self.unlink(parent_id, name).await.map_err(|e| {
            error!("[unlink]: {e}");
            PosixError::new(ErrorKind::InputOutputError, e.to_string())
        })
    }

    #[doc = " Map block index within file to block index within device"]
    #[doc = ""]
    #[doc = " Note: This makes sense only for block device backed filesystems mounted"]
    #[doc = " with the \'blkdev\' option"]
    async fn bmap(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        blocksize: u32,
        idx: u64,
    ) -> FuseResult<u64> {
        let file_id = normalise_path(&file_id);
        debug!("[bmap] file_id = {file_id:?}, blocksize = {blocksize}, idx = {idx}");
        self.get_inner_fuse_handler()
            .bmap(req, file_id, blocksize, idx)
    }

    #[doc = " Copy the specified range from the source inode to the destination inode"]
    async fn copy_file_range(
        &self,
        _req: &RequestInfo,
        file_in: Self::TId,
        file_handle_in: BorrowedFileHandle<'_>,
        offset_in: u64,
        file_out: Self::TId,
        file_handle_out: BorrowedFileHandle<'_>,
        offset_out: u64,
        len: u64,
        flags: CopyFileRangeFlags,
    ) -> FuseResult<u32> {
        let file_in = normalise_path(&file_in);
        let file_out = normalise_path(&file_out);
        debug!(
            "[copy_file_range] file_in = {file_in:?}, file_handle_in = {file_handle_in:?}, offset_in = {offset_in:?}, file_out = {file_out:?}, file_handle_out = {file_handle_out:?}, offset_out = {offset_out:?}, len = {len}, flags = {flags:?}"
        );
        self.copy_file_range(file_in, offset_in, file_out, offset_out, len)
            .await
            .map_err(|e| {
                error!("[copy_file_range]: {e}");
                PosixError::new(ErrorKind::InputOutputError, e.to_string())
            })
    }

    #[doc = " Preallocate or deallocate space to a file"]
    async fn fallocate(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        offset: i64,
        length: i64,
        mode: FallocateFlags,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[fallocate] file_id = {file_id:?}, file_handle = {file_handle:?}, offset = {offset}, length = {length}, mode = {mode:?}");
        self.get_inner_fuse_handler()
            .fallocate(req, file_id, file_handle, offset, length, mode)
    }

    #[doc = " Synchronize directory contents"]
    #[doc = ""]
    #[doc = " If the datasync parameter is true, then only the directory contents should"]
    #[doc = " be flushed, not the metadata. The file_handle will contain the value set"]
    #[doc = " by the opendir method, or will be undefined if the opendir method didn\'t"]
    #[doc = " set any value."]
    async fn fsyncdir(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        datasync: bool,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[fsyncdir] file_id = {file_id:?}, file_handle = {file_handle:?}, datasync = {datasync}");
        self.get_inner_fuse_handler()
            .fsyncdir(req, file_id, file_handle, datasync)
    }

    #[doc = " Test for a POSIX file lock."]
    async fn getlk(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        let file_id = normalise_path(&file_id);
        debug!("[getlk] file_id = {file_id:?}, file_handle = {file_handle:?}, lock_owner = {lock_owner}, lock_info = {lock_info:?}");
        self.get_inner_fuse_handler()
            .getlk(req, file_id, file_handle, lock_owner, lock_info)
    }

    #[doc = " Get an extended attribute"]
    async fn getxattr(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        let file_id = normalise_path(&file_id);
        debug!("[getxattr] file_id = {file_id:?}, name = {name:?}, size = {size}");
        self.get_inner_fuse_handler()
            .getxattr(req, file_id, name, size)
    }

    #[doc = " control device"]
    async fn ioctl(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        flags: IoctlFlags,
        cmd: u32,
        in_data: Vec<u8>,
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        let file_id = normalise_path(&file_id);
        let in_data_len = in_data.len();
        debug!("[ioctl] file_id = {file_id:?}, file_handle = {file_handle:?}, flags = {flags:?}, cmd = {cmd}, in_data_len = {in_data_len}, out_size = {out_size}");
        self.get_inner_fuse_handler().ioctl(
            req,
            file_id,
            file_handle,
            flags,
            cmd,
            in_data,
            out_size,
        )
    }

    #[doc = " Create a hard link."]
    async fn link(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        newparent: Self::TId,
        newname: &OsStr,
    ) -> FuseResult<<Self::TId as FileIdType>::Metadata> {
        let file_id = normalise_path(&file_id);
        let newparent = normalise_path(&newparent);
        debug!("[link] file_id = {file_id:?}, newparent = {newparent:?}, newname = {newname:?}");
        self.get_inner_fuse_handler()
            .link(req, file_id, newparent, newname)
    }

    #[doc = " List extended attribute names"]
    async fn listxattr(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        let file_id = normalise_path(&file_id);
        debug!("[listxattr] file_id = {file_id:?}, size = {size}");
        self.get_inner_fuse_handler().listxattr(req, file_id, size)
    }

    #[doc = " Reposition read/write file offset"]
    async fn lseek(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        seek: SeekFrom,
    ) -> FuseResult<i64> {
        let file_id = normalise_path(&file_id);
        debug!("[lseek] file_id = {file_id:?}, file_handle = {file_handle:?}, seek = {seek:?}");
        self.get_inner_fuse_handler()
            .lseek(req, file_id, file_handle, seek)
    }

    #[doc = " Create a new directory"]
    async fn mkdir(
        &self,
        _req: &RequestInfo,
        parent_id: Self::TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<<Self::TId as FileIdType>::Metadata> {
        let parent_id = normalise_path(&parent_id);
        debug!("[mkdir] parent_id = {parent_id:?}, name = {name:?}, mode = {mode:#06o}, umask = {umask:#06o}");
        self.mkdir(parent_id, name).await.map_err(|e| {
            error!("[mkdir]: {e}");
            PosixError::new(ErrorKind::InputOutputError, e.to_string())
        })
    }

    #[doc = " Create a new file node (regular file, device, FIFO, socket, etc)"]
    async fn mknod(
        &self,
        req: &RequestInfo,
        parent_id: Self::TId,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<<Self::TId as FileIdType>::Metadata> {
        let parent_id = normalise_path(&parent_id);
        debug!("[mknod] parent_id = {parent_id:?}, name = {name:?}, mode = {mode:#06o}, umask = {umask:#06o}, rdev = {rdev:?}");
        self.get_inner_fuse_handler()
            .mknod(req, parent_id, name, mode, umask, rdev)
    }

    #[doc = " Open a file and return a file handle."]
    #[doc = ""]
    #[doc = " Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are available in flags. You may store an arbitrary file handle (pointer, index, etc) in file_handle response, and use this in other all other file operations (read, write, flush, release, fsync). Filesystem may also implement stateless file I/O and not store anything in fh. There are also some flags (direct_io, keep_cache) which the filesystem may set, to change the way the file is opened. See fuse_file_info structure in <fuse_common.h> for more details."]
    async fn open(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
        let file_id = normalise_path(&file_id);
        debug!("[open] file_id = {file_id:?}, flags = {flags:?}");
        self.get_inner_fuse_handler().open(req, file_id, flags)
    }

    #[doc = " Open a directory"]
    #[doc = ""]
    #[doc = " Allows storing a file handle for use in subsequent directory operations."]
    async fn opendir(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FopenFlags)> {
        let file_id = normalise_path(&file_id);
        debug!("[opendir] file_id = {file_id:?}, flags = {flags:?}");
        self.get_inner_fuse_handler().opendir(req, file_id, flags)
    }

    #[doc = " Read the target of a symbolic link"]
    async fn readlink(&self, req: &RequestInfo, file_id: Self::TId) -> FuseResult<Vec<u8>> {
        let file_id = normalise_path(&file_id);
        debug!("[readlink] file_id = {file_id:?}");
        self.get_inner_fuse_handler().readlink(req, file_id)
    }

    #[doc = " Release an open file"]
    #[doc = ""]
    #[doc = " Called when all file descriptors are closed and all memory mappings are unmapped."]
    #[doc = " Guaranteed to be called once for every open() call."]
    async fn release(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: OwnedFileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[release] file_id = {file_id:?}, file_handle = {file_handle:?}, flags = {flags:?}, lock_owner = {lock_owner:?}, flush = {flush}");
        self.get_inner_fuse_handler()
            .release(req, file_id, file_handle, flags, lock_owner, flush)
    }

    #[doc = " Release an open directory"]
    #[doc = ""]
    #[doc = " This method is called exactly once for every successful opendir operation."]
    #[doc = " The file_handle parameter will contain the value set by the opendir method,"]
    #[doc = " or will be undefined if the opendir method didn\'t set any value."]
    async fn releasedir(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: OwnedFileHandle,
        flags: OpenFlags,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!(
            "[releasedir] file_id = {file_id:?}, file_handle = {file_handle:?}, flags = {flags:?}"
        );
        self.get_inner_fuse_handler()
            .releasedir(req, file_id, file_handle, flags)
    }

    #[doc = " Remove an extended attribute."]
    async fn removexattr(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        name: &OsStr,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[removexattr] file_id = {file_id:?}, name = {name:?}");
        self.get_inner_fuse_handler()
            .removexattr(req, file_id, name)
    }

    async fn setattr(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        attrs: SetAttrRequest<'_>,
    ) -> FuseResult<FileAttribute> {
        let file_id = normalise_path(&file_id);
        debug!("[setattr] file_id = {file_id:?}, attrs = {attrs:?}");
        self.get_inner_fuse_handler().setattr(req, file_id, attrs)
    }

    #[doc = " Acquire, modify or release a POSIX file lock"]
    #[doc = ""]
    #[doc = " For POSIX threads (NPTL) there\'s a 1-1 relation between pid and owner, but"]
    #[doc = " otherwise this is not always the case. For checking lock ownership, \'fi->owner\'"]
    #[doc = " must be used. The l_pid field in \'struct flock\' should only be used to fill"]
    #[doc = " in this field in getlk()."]
    async fn setlk(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        file_handle: BorrowedFileHandle<'_>,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[setlk] file_id = {file_id:?}, file_handle = {file_handle:?}, lock_owner = {lock_owner}, lock_info = {lock_info:?}, sleep = {sleep}");
        self.get_inner_fuse_handler()
            .setlk(req, file_id, file_handle, lock_owner, lock_info, sleep)
    }

    #[doc = " Set an extended attribute"]
    async fn setxattr(
        &self,
        req: &RequestInfo,
        file_id: Self::TId,
        name: &OsStr,
        value: Vec<u8>,
        flags: SetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        let value_str = String::from_utf8_lossy(&value);
        debug!("[setxattr] file_id = {file_id:?}, name = {name:?}, value = {value_str}, flags = {flags:?}, position = {position}");
        self.get_inner_fuse_handler()
            .setxattr(req, file_id, name, value, flags, position)
    }

    #[doc = " Create a symbolic link."]
    async fn symlink(
        &self,
        req: &RequestInfo,
        parent_id: Self::TId,
        link_name: &OsStr,
        target: &Path,
    ) -> FuseResult<<Self::TId as FileIdType>::Metadata> {
        let parent_id = normalise_path(&parent_id);
        let target = normalise_path(&target.to_path_buf());
        debug!(
            "[symlink] parent_id = {parent_id:?}, link_name = {link_name:?}, target = {target:?}"
        );
        self.get_inner_fuse_handler()
            .symlink(req, parent_id, link_name, target.as_path())
    }
}
