use crate::fs::OkuFs;
use crate::fuse::util::*;
use easy_fuser::fuse_serial::prelude::SeekFrom;
use easy_fuser::types::FUSEOpenResponseFlags;
use easy_fuser::types::FileAttribute;
use easy_fuser::types::FileIdType;
use easy_fuser::types::FileKind::Directory;
use easy_fuser::types::OpenFlags;
use easy_fuser::types::OwnedFileHandle;
use easy_fuser::types::RequestInfo;
use log::info;
use miette::IntoDiagnostic;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Seek;
use std::io::Write;
use std::path::PathBuf;

impl OkuFs {
    pub(super) async fn getattr(&self, file_id: PathBuf) -> miette::Result<FileAttribute> {
        self.get_fs_entry_attributes(&file_id).await
    }

    pub(super) async fn read(
        &self,
        file_id: PathBuf,
        seek: SeekFrom,
        size: u32,
    ) -> miette::Result<Vec<u8>> {
        let (namespace_id, replica_path) = parse_fuse_path(&file_id)
            .map(|x| x.ok_or(miette::miette!("Cannot read root directory as file")))??;
        self.read_file(
            &namespace_id,
            &replica_path,
            &Some(seek),
            &Some(size.into()),
        )
        .await
        .map(|x| x.into())
    }

    pub(super) async fn copy_file_range(
        &self,
        file_in: PathBuf,
        offset_in: i64,
        file_out: PathBuf,
        offset_out: i64,
        len: u64,
    ) -> miette::Result<u32> {
        let data = self
            .read(file_in, SeekFrom::Start(offset_in as u64), len as u32)
            .await?;
        self.write(file_out, SeekFrom::Start(offset_out as u64), data)
            .await
    }

    pub(super) async fn readdir(
        &self,
        file_id: PathBuf,
    ) -> miette::Result<Vec<(OsString, <PathBuf as FileIdType>::MinimalMetadata)>> {
        let mut directory_entries: Vec<(OsString, <PathBuf as FileIdType>::MinimalMetadata)> = vec![
            (std::ffi::OsString::from("."), Directory),
            (std::ffi::OsString::from(".."), Directory),
        ];
        let parsed_path = parse_fuse_path(&file_id)?;
        match parsed_path {
            None => {
                let replicas = self.list_replicas().await?;
                for (replica, _capability_kind, _is_home_replica) in replicas {
                    directory_entries.push((crate::fs::util::fmt(replica).into(), Directory));
                }
                Ok(directory_entries)
            }
            Some((namespace_id, replica_path)) => {
                let files = self
                    .list_files(&namespace_id, &Some(replica_path.clone()))
                    .await?;
                let immediate_children = get_immediate_children(replica_path, files)?;
                directory_entries.extend(immediate_children);
                Ok(directory_entries)
            }
        }
    }

    pub(super) async fn rmdir(&self, parent_id: PathBuf, name: &OsStr) -> miette::Result<()> {
        let handle = self.get_handle()?;
        let path = parent_id.join(name);
        let (namespace_id, replica_path) = parse_fuse_path(&path)
            .map(|x| x.ok_or(miette::miette!("Cannot remove root directory")))??;
        match is_root_path(&replica_path) {
            true => {
                self.delete_replica(&namespace_id).await?;
                info!("Replica {namespace_id} deleted");
            }
            false => {
                let entries_deleted = handle.block_on(async {
                    self.delete_directory(&namespace_id, &replica_path).await
                })?;
                info!("{entries_deleted} entries deleted in {path:?}");
            }
        }
        Ok(())
    }

    pub(super) async fn create(
        &self,
        _req: &RequestInfo,
        parent_id: PathBuf,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: OpenFlags,
    ) -> miette::Result<(
        OwnedFileHandle,
        <PathBuf as FileIdType>::Metadata,
        FUSEOpenResponseFlags,
    )> {
        let path = parent_id.join(name);
        let (namespace_id, replica_path) = parse_fuse_path(&path)
            .map(|x| x.ok_or(miette::miette!("Cannot create file at root path")))??;
        let file_hash = self
            .create_or_modify_file(&namespace_id, &replica_path, b"\0".as_slice())
            .await?;
        info!("File created at {path:?} with hash {file_hash}");
        let file_attr = self.getattr(path).await?;
        Ok((
            unsafe { OwnedFileHandle::from_raw(0) },
            file_attr,
            FUSEOpenResponseFlags::empty(),
        ))
    }

    pub(super) async fn rename(
        &self,
        parent_id: PathBuf,
        name: &OsStr,
        newparent: PathBuf,
        newname: &OsStr,
    ) -> miette::Result<()> {
        let old_path = parent_id.join(name);
        let path_type = self.is_file_or_directory(&old_path).await?;
        let new_path = newparent.join(newname);
        let (old_namespace_id, old_replica_path) = parse_fuse_path(&old_path)
            .map(|x| x.ok_or(miette::miette!("Cannot rename root directory")))??;
        let (new_namespace_id, new_replica_path) = parse_fuse_path(&new_path)
            .map(|x| x.ok_or(miette::miette!("Cannot rename root directory")))??;
        match path_type {
            easy_fuser::types::FileKind::RegularFile => {
                let (new_hash, files_moved) = self
                    .move_file(
                        &old_namespace_id,
                        &old_replica_path,
                        &new_namespace_id,
                        &new_replica_path,
                    )
                    .await?;
                info!("File {old_path:?} moved to {new_path:?} (files moved: {files_moved}, new hash: {new_hash})");
                Ok(())
            }
            easy_fuser::types::FileKind::Directory => {
                let (new_hashes, files_moved) = self
                    .move_directory(
                        &old_namespace_id,
                        &old_replica_path,
                        &new_namespace_id,
                        &new_replica_path,
                    )
                    .await?;
                info!("Directory {old_path:?} moved to {new_path:?} (files moved: {files_moved}, new hashes: {new_hashes:?})");
                Ok(())
            }
            _ => Err(miette::miette!(
                "File system entry type {path_type:?} at {old_path:?} not supported"
            )),
        }
    }

    pub(super) async fn write(
        &self,
        file_id: PathBuf,
        seek: SeekFrom,
        data: Vec<u8>,
    ) -> miette::Result<u32> {
        let (namespace_id, replica_path) = parse_fuse_path(&file_id).map(|x| {
            x.ok_or(miette::miette!(
                "Cannot write bytes to root directory as it's not a file"
            ))
        })??;
        let file_bytes = self
            .read_file(&namespace_id, &replica_path, &None, &None)
            .await?;
        let mut writer = BufWriter::new(Cursor::new(file_bytes.to_vec()));
        writer.seek(seek).into_diagnostic()?;
        writer.write(&data).into_diagnostic()?;
        let inner_cursor = writer.into_inner().into_diagnostic()?;
        let file_hash = self
            .create_or_modify_file(&namespace_id, &replica_path, inner_cursor.into_inner())
            .await?;
        info!("File at {file_id:?} updated (hash: {file_hash})");
        Ok(data.len().try_into().unwrap_or(u32::MAX))
    }

    pub(super) async fn unlink(&self, parent_id: PathBuf, name: &OsStr) -> miette::Result<()> {
        let path = parent_id.join(name);
        let (namespace_id, replica_path) = parse_fuse_path(&path)
            .map(|x| x.ok_or(miette::miette!("Cannot remove root directory")))??;
        let entries_deleted = self.delete_file(&namespace_id, &replica_path).await?;
        info!("File deleted at {path:?} (files deleted: {entries_deleted})");
        Ok(())
    }
}
