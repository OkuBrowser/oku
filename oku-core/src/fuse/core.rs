use crate::fs::util::normalise_path;
use crate::fs::OkuFs;
use easy_fuser::prelude::*;
use log::debug;
use log::error;
use log::info;
use log::trace;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;

impl FuseHandler<PathBuf> for OkuFs {
    fn get_inner(&self) -> &dyn FuseHandler<PathBuf> {
        &**self.fuse_handler
    }

    fn destroy(&self) {
        self.handle
            .block_on(async move { self.clone().shutdown().await });
        info!("Node unmounting … ");
    }

    fn flush(
        &self,
        _req: &RequestInfo,
        _file_id: PathBuf,
        _file_handle: BorrowedFileHandle,
        _lock_owner: u64,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn fsync(
        &self,
        _req: &RequestInfo,
        _file_id: PathBuf,
        _file_handle: BorrowedFileHandle,
        _datasync: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    fn access(&self, _req: &RequestInfo, file_id: PathBuf, mode: AccessMask) -> FuseResult<()> {
        let file_id = normalise_path(&file_id);
        debug!("[access] file_id = {file_id:?}, mode = {mode:#06o}");
        return FuseResult::Ok(());
    }

    fn statfs(&self, _req: &RequestInfo, file_id: PathBuf) -> FuseResult<StatFs> {
        let file_id = normalise_path(&file_id);
        trace!("[statfs] file_id = {file_id:?}");
        self.handle
            .block_on(async { self.get_fs_entry_stats(&file_id).await })
            .map_err(|e| {
                error!("[statfs]: {e}");
                PosixError::new(ErrorKind::FileNotFound, e.to_string())
            })
    }

    fn getattr(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        file_handle: Option<BorrowedFileHandle>,
    ) -> FuseResult<FileAttribute> {
        let file_id = normalise_path(&file_id);
        debug!("[getattr] file_id = {file_id:?}, file_handle = {file_handle:?}");
        self.getattr(file_id).map_err(|e| {
            error!("[getattr]: {e}");
            PosixError::new(ErrorKind::FileNotFound, e.to_string())
        })
    }

    fn read(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
        seek: SeekFrom,
        size: u32,
        _flags: FUSEOpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<Vec<u8>> {
        let file_id = normalise_path(&file_id);
        debug!(
            "[read] file_id = {file_id:?}, file_handle = {file_handle:?}, seek = {seek:?}, size = {size}"
        );
        self.read(file_id, seek).map_err(|e| {
            error!("[read]: {e}");
            PosixError::new(ErrorKind::FileNotFound, e.to_string())
        })
    }

    fn readdir(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
    ) -> FuseResult<Vec<(OsString, <PathBuf as FileIdType>::MinimalMetadata)>> {
        let file_id = normalise_path(&file_id);
        debug!("[readdir] file_id = {file_id:?}, file_handle = {file_handle:?}");
        self.readdir(file_id).map_err(|e| {
            error!("[readdir]: {e}");
            PosixError::new(ErrorKind::FileNotFound, e.to_string())
        })
    }

    fn rmdir(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        let parent_id = normalise_path(&parent_id);
        debug!("[rmdir] parent_id = {parent_id:?}, name = {name:?}");
        self.rmdir(parent_id, name).map_err(|e| {
            error!("[rmdir]: {e}");
            PosixError::new(ErrorKind::FileNotFound, e.to_string())
        })
    }

    fn create(
        &self,
        req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        mode: u32,
        umask: u32,
        flags: OpenFlags,
    ) -> FuseResult<(
        OwnedFileHandle,
        <PathBuf as FileIdType>::Metadata,
        FUSEOpenResponseFlags,
    )> {
        let parent_id = normalise_path(&parent_id);
        debug!("[create] parent_id = {parent_id:?}, name = {name:?}, mode = {mode}, umask = {umask}, flags = {flags:?}");
        self.create(req, parent_id, name, mode, umask, flags)
            .map_err(|e| {
                error!("[create]: {e}");
                PosixError::new(ErrorKind::FileNotFound, e.to_string())
            })
    }

    fn get_default_ttl(&self) -> std::time::Duration {
        std::time::Duration::from_secs(1)
    }

    fn init(&self, req: &RequestInfo, config: &mut KernelConfig) -> FuseResult<()> {
        self.get_inner().init(req, config)
    }

    fn bmap(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        blocksize: u32,
        idx: u64,
    ) -> FuseResult<u64> {
        self.get_inner().bmap(req, file_id, blocksize, idx)
    }

    fn copy_file_range(
        &self,
        req: &RequestInfo,
        file_in: PathBuf,
        file_handle_in: BorrowedFileHandle,
        offset_in: i64,
        file_out: PathBuf,
        file_handle_out: BorrowedFileHandle,
        offset_out: i64,
        len: u64,
        flags: u32, // Not implemented yet in standard
    ) -> FuseResult<u32> {
        self.get_inner().copy_file_range(
            req,
            file_in,
            file_handle_in,
            offset_in,
            file_out,
            file_handle_out,
            offset_out,
            len,
            flags,
        )
    }

    fn fallocate(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
        offset: i64,
        length: i64,
        mode: FallocateFlags,
    ) -> FuseResult<()> {
        self.get_inner()
            .fallocate(req, file_id, file_handle, offset, length, mode)
    }

    fn forget(&self, req: &RequestInfo, file_id: PathBuf, nlookup: u64) {
        self.get_inner().forget(req, file_id, nlookup);
    }

    fn fsyncdir(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
        datasync: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .fsyncdir(req, file_id, file_handle, datasync)
    }

    fn getlk(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
    ) -> FuseResult<LockInfo> {
        self.get_inner()
            .getlk(req, file_id, file_handle, lock_owner, lock_info)
    }

    fn getxattr(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        size: u32,
    ) -> FuseResult<Vec<u8>> {
        debug!("[getxattr] file_id = {file_id:?}, name = {name:?}, size = {size}");
        self.get_inner().getxattr(req, file_id, name, size)
    }

    fn ioctl(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
        flags: IOCtlFlags,
        cmd: u32,
        in_data: Vec<u8>,
        out_size: u32,
    ) -> FuseResult<(i32, Vec<u8>)> {
        self.get_inner()
            .ioctl(req, file_id, file_handle, flags, cmd, in_data, out_size)
    }

    fn link(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        newparent: PathBuf,
        newname: &OsStr,
    ) -> FuseResult<<PathBuf as FileIdType>::Metadata> {
        self.get_inner().link(req, file_id, newparent, newname)
    }

    fn listxattr(&self, req: &RequestInfo, file_id: PathBuf, size: u32) -> FuseResult<Vec<u8>> {
        self.get_inner().listxattr(req, file_id, size)
    }

    fn lookup(
        &self,
        req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
    ) -> FuseResult<<PathBuf as FileIdType>::Metadata> {
        let parent_id = normalise_path(&parent_id);
        debug!("[lookup] parent_id = {parent_id:?}, name = {name:?}");
        FuseHandler::getattr(self, req, parent_id.join(name), None)
    }

    fn lseek(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
        seek: SeekFrom,
    ) -> FuseResult<i64> {
        self.get_inner().lseek(req, file_id, file_handle, seek)
    }

    fn mkdir(
        &self,
        req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        mode: u32,
        umask: u32,
    ) -> FuseResult<<PathBuf as FileIdType>::Metadata> {
        self.get_inner().mkdir(req, parent_id, name, mode, umask)
    }

    fn mknod(
        &self,
        req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        mode: u32,
        umask: u32,
        rdev: DeviceType,
    ) -> FuseResult<<PathBuf as FileIdType>::Metadata> {
        self.get_inner()
            .mknod(req, parent_id, name, mode, umask, rdev)
    }

    fn open(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FUSEOpenResponseFlags)> {
        debug!("[open] file_id = {file_id:?}, flags = {flags:?}");
        self.get_inner().open(req, file_id, flags)
    }

    fn opendir(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        flags: OpenFlags,
    ) -> FuseResult<(OwnedFileHandle, FUSEOpenResponseFlags)> {
        debug!("[opendir] file_id = {file_id:?}, flags = {flags:?}");
        self.get_inner().opendir(req, file_id, flags)
    }

    fn readdirplus(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
    ) -> FuseResult<Vec<(OsString, <PathBuf as FileIdType>::Metadata)>> {
        let readdir_result = FuseHandler::readdir(self, req, file_id.clone(), file_handle)?;
        let mut result = Vec::with_capacity(readdir_result.len());
        for (name, _) in readdir_result.into_iter() {
            let metadata = self.lookup(req, file_id.clone(), &name)?;
            result.push((name, metadata));
        }
        Ok(result)
    }

    fn readlink(&self, req: &RequestInfo, file_id: PathBuf) -> FuseResult<Vec<u8>> {
        self.get_inner().readlink(req, file_id)
    }

    fn release(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: OwnedFileHandle,
        flags: OpenFlags,
        lock_owner: Option<u64>,
        flush: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .release(req, file_id, file_handle, flags, lock_owner, flush)
    }

    fn releasedir(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: OwnedFileHandle,
        flags: OpenFlags,
    ) -> FuseResult<()> {
        self.get_inner()
            .releasedir(req, file_id, file_handle, flags)
    }

    fn removexattr(&self, req: &RequestInfo, file_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        self.get_inner().removexattr(req, file_id, name)
    }

    fn rename(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        newparent: PathBuf,
        newname: &OsStr,
        flags: RenameFlags,
    ) -> FuseResult<()> {
        let parent_id = normalise_path(&parent_id);
        let newparent = normalise_path(&newparent);
        debug!("[rename] parent_id = {parent_id:?}, name = {name:?}, newparent = {newparent:?}, newname = {newname:?}, flags = {flags:?}");
        self.rename(parent_id, name, newparent, newname)
            .map_err(|e| {
                error!("[rename]: {e}");
                PosixError::new(ErrorKind::FileNotFound, e.to_string())
            })
    }

    fn setattr(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        attrs: SetAttrRequest,
    ) -> FuseResult<FileAttribute> {
        self.get_inner().setattr(req, file_id, attrs)
    }

    fn setlk(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        file_handle: BorrowedFileHandle,
        lock_owner: u64,
        lock_info: LockInfo,
        sleep: bool,
    ) -> FuseResult<()> {
        self.get_inner()
            .setlk(req, file_id, file_handle, lock_owner, lock_info, sleep)
    }

    fn setxattr(
        &self,
        req: &RequestInfo,
        file_id: PathBuf,
        name: &OsStr,
        value: Vec<u8>,
        flags: FUSESetXAttrFlags,
        position: u32,
    ) -> FuseResult<()> {
        self.get_inner()
            .setxattr(req, file_id, name, value, flags, position)
    }

    fn symlink(
        &self,
        req: &RequestInfo,
        parent_id: PathBuf,
        link_name: &OsStr,
        target: &std::path::Path,
    ) -> FuseResult<<PathBuf as FileIdType>::Metadata> {
        self.get_inner().symlink(req, parent_id, link_name, target)
    }

    fn write(
        &self,
        _req: &RequestInfo,
        file_id: PathBuf,
        _file_handle: BorrowedFileHandle,
        seek: SeekFrom,
        data: Vec<u8>,
        write_flags: FUSEWriteFlags,
        flags: OpenFlags,
        _lock_owner: Option<u64>,
    ) -> FuseResult<u32> {
        let file_id = normalise_path(&file_id);
        debug!("[write] file_id = {file_id:?}, seek = {seek:?}, data = {data:?}, write_flags = {write_flags:?}, flags = {flags:?}");
        self.write(file_id, seek, data).map_err(|e| {
            error!("[write]: {e}");
            PosixError::new(ErrorKind::FileNotFound, e.to_string())
        })
    }

    fn unlink(&self, _req: &RequestInfo, parent_id: PathBuf, name: &OsStr) -> FuseResult<()> {
        let parent_id = normalise_path(&parent_id);
        debug!("[unlink] parent_id = {parent_id:?}, name = {name:?}");
        self.unlink(parent_id, name).map_err(|e| {
            error!("[unlink]: {e}");
            PosixError::new(ErrorKind::FileNotFound, e.to_string())
        })
    }
}
