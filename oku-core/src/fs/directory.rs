use super::*;
use crate::error::OkuFsError;
use anyhow::anyhow;
use bytes::Bytes;
use futures::{future, pin_mut, StreamExt};
use iroh_blobs::Hash;
use iroh_docs::store::FilterKind;
use iroh_docs::sync::Entry;
use iroh_docs::DocTicket;
use iroh_docs::NamespaceId;
use log::error;
use miette::IntoDiagnostic;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use std::path::Path;
use std::path::PathBuf;
use util::entry_key_to_path;
use util::normalise_path;
use util::path_to_entry_prefix;

impl OkuFs {
    /// Reads the contents of the files in a directory.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the folder.
    ///
    /// * `path` - The folder whose contents will be read.
    ///
    /// # Returns
    ///
    /// A list of file entries and the corresponding content as bytes.
    pub async fn read_directory(
        &self,
        namespace_id: &NamespaceId,
        path: &Path,
    ) -> miette::Result<Vec<(Entry, Bytes)>> {
        let entries = self
            .list_files(namespace_id, &Some(path.to_path_buf()))
            .await?;
        let bytes = future::try_join_all(entries.iter().map(|entry| self.content_bytes(entry)))
            .await
            .map_err(|e| miette::miette!("{}", e))?;
        Ok(entries.into_par_iter().zip(bytes.into_par_iter()).collect())
    }

    /// Moves a directory by copying it to a new location and deleting the original.
    ///
    /// # Arguments
    ///
    /// * `from_namespace_id` - The ID of the replica containing the directory to move.
    ///
    /// * `to_namespace_id` - The ID of the replica to move the directory to.
    ///
    /// * `from_path` - The path of the directory to move.
    ///
    /// * `to_path` - The path to move the directory to.
    ///
    /// # Returns
    ///
    /// A tuple containing the list of file hashes for files at their new destinations, and the total number of replica entries deleted during the operation.
    pub async fn move_directory(
        &self,
        from_namespace_id: &NamespaceId,
        from_path: &Path,
        to_namespace_id: &NamespaceId,
        to_path: &Path,
    ) -> miette::Result<(Vec<Hash>, usize)> {
        let mut entries_deleted = 0;
        let mut moved_file_hashes = Vec::new();
        let old_directory_files = self
            .list_files(from_namespace_id, &Some(from_path.to_path_buf()))
            .await?;
        for old_directory_file in old_directory_files {
            let old_file_path = entry_key_to_path(old_directory_file.key())?;
            let new_file_path = to_path.join(old_file_path.file_name().unwrap_or_default());
            let file_move_info = self
                .move_file(
                    from_namespace_id,
                    &old_file_path,
                    to_namespace_id,
                    &new_file_path,
                )
                .await?;
            moved_file_hashes.push(file_move_info.0);
            entries_deleted += file_move_info.1;
        }
        Ok((moved_file_hashes, entries_deleted))
    }

    /// Deletes a directory and all its contents.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the directory to delete.
    ///
    /// * `path` - The path of the directory to delete.
    ///
    /// # Returns
    ///
    /// The number of entries deleted.
    pub async fn delete_directory(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<usize> {
        let path = normalise_path(path).join(""); // Ensure path ends with a slash
        let file_key = path_to_entry_prefix(&path);
        let docs_client = &self.docs;
        let document = docs_client
            .open(*namespace_id)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotOpenReplica
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        let mut entries_deleted = 0;
        let query = iroh_docs::store::Query::single_latest_per_key()
            .key_prefix(file_key)
            .build();
        let entries = document.get_many(query).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotListFiles
        })?;
        pin_mut!(entries);
        let files: Vec<Entry> = entries.map(|entry| entry.unwrap()).collect().await;
        for file in files {
            entries_deleted += document
                .del(
                    file.author(),
                    (std::str::from_utf8(&path_to_entry_prefix(&entry_key_to_path(file.key())?))
                        .into_diagnostic()?)
                    .to_string(),
                )
                .await
                .map_err(|e| {
                    error!("{}", e);
                    OkuFsError::CannotDeleteDirectory
                })?;
        }
        Ok(entries_deleted)
    }

    /// Determines the oldest timestamp of a file entry in a folder.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the folder.
    ///
    /// * `path` - The folder whose oldest timestamp is to be determined.
    ///
    /// # Returns
    ///
    /// The oldest timestamp of any file descending from this folder, in microseconds from the Unix epoch.
    pub async fn get_oldest_timestamp_in_folder(
        &self,
        namespace_id: &NamespaceId,
        path: &Path,
    ) -> miette::Result<u64> {
        let files = self
            .list_files(namespace_id, &Some(path.to_path_buf()))
            .await?;
        let mut timestamps: Vec<u64> = Vec::new();
        for file in files {
            timestamps.push(
                self.get_oldest_entry_timestamp(namespace_id, &entry_key_to_path(file.key())?)
                    .await?,
            );
        }
        Ok(*timestamps.par_iter().min().unwrap_or(&u64::MIN))
    }

    /// Determines the latest timestamp of a file entry in a folder.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the folder.
    ///
    /// * `path` - The folder whose latest timestamp is to be determined.
    ///
    /// # Returns
    ///
    /// The latest timestamp of any file descending from this folder, in microseconds from the Unix epoch.
    pub async fn get_newest_timestamp_in_folder(
        &self,
        namespace_id: &NamespaceId,
        path: &Path,
    ) -> miette::Result<u64> {
        let files = self
            .list_files(namespace_id, &Some(path.to_path_buf()))
            .await?;
        let mut timestamps: Vec<u64> = Vec::new();
        for file in files {
            timestamps.push(file.timestamp());
        }
        Ok(*timestamps.par_iter().max().unwrap_or(&u64::MIN))
    }

    /// Determines the size of a folder.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the folder.
    ///
    /// * `path` - The path to the folder within the replica.
    ///
    /// # Returns
    ///
    /// The total size, in bytes, of the files descending from this folder.
    pub async fn get_folder_size(
        &self,
        namespace_id: &NamespaceId,
        path: &Path,
    ) -> miette::Result<u64> {
        let files = self
            .list_files(namespace_id, &Some(path.to_path_buf()))
            .await?;
        let mut size = 0;
        for file in files {
            size += file.content_len();
        }
        Ok(size)
    }

    /// Join a swarm to fetch the latest version of a directory and save it to the local machine.
    ///
    /// # Arguments
    ///
    /// * `ticket` - A ticket for the replica containing the directory to retrieve.
    ///
    /// * `path` - The path to the directory to retrieve.
    ///
    /// # Returns
    ///
    /// The content of the files in the directory.
    pub async fn fetch_directory_with_ticket(
        &self,
        ticket: &DocTicket,
        path: &Path,
        filters: &Option<Vec<FilterKind>>,
    ) -> anyhow::Result<Vec<(Entry, Bytes)>> {
        self.fetch_replica_by_ticket(ticket, &Some(path.to_path_buf()), filters)
            .await?;
        self.read_directory(&ticket.capability.id(), path)
            .await
            .map_err(|e| anyhow!("{}", e))
    }
}
