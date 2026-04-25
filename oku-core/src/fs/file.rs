use super::*;
use crate::error::OkuFsError;
use anyhow::anyhow;
use bytes::Bytes;
use futures::{pin_mut, StreamExt};
use iroh_blobs::Hash;
use iroh_docs::api::Doc;
use iroh_docs::engine::LiveEvent;
use iroh_docs::store::FilterKind;
use iroh_docs::sync::Entry;
use iroh_docs::DocTicket;
use iroh_docs::NamespaceId;
use log::{error, info};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;
use util::path_to_entry_key;
use util::path_to_entry_prefix;

impl OkuFs {
    /// Lists files in a replica.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica to list files in.
    ///
    /// * `path` - An optional path within the replica.
    ///
    /// # Returns
    ///
    /// A list of files in the replica.
    pub async fn list_files(
        &self,
        namespace_id: &NamespaceId,
        path: &Option<PathBuf>,
    ) -> miette::Result<Vec<Entry>> {
        let docs_client = &self.docs;
        let document = docs_client
            .open(*namespace_id)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotOpenReplica
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        let query = if let Some(path) = path {
            let file_key = path_to_entry_prefix(path);
            iroh_docs::store::Query::single_latest_per_key()
                .key_prefix(file_key)
                .build()
        } else {
            iroh_docs::store::Query::single_latest_per_key().build()
        };
        let entries = document.get_many(query).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotListFiles
        })?;
        pin_mut!(entries);
        let files: Vec<Entry> = entries.map(|entry| entry.unwrap()).collect().await;
        Ok(files)
    }

    /// Creates a file (if it does not exist) or modifies an existing file.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the file to create or modify.
    ///
    /// * `path` - The path of the file to create or modify.
    ///
    /// * `data` - The data to write to the file.
    ///
    /// # Returns
    ///
    /// The hash of the file.
    pub async fn create_or_modify_file(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
        data: impl Into<Bytes>,
    ) -> miette::Result<Hash> {
        let file_key = path_to_entry_key(path);
        let docs_client = &self.docs;
        let document = docs_client
            .open(*namespace_id)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotOpenReplica
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        let entry_hash = document
            .set_bytes(self.default_author().await, file_key, data)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotCreateOrModifyFile
            })?;

        Ok(entry_hash)
    }

    /// Deletes a file.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the file to delete.
    ///
    /// * `path` - The path of the file to delete.
    ///
    /// # Returns
    ///
    /// The number of entries deleted in the replica, which should be 1 if the file was successfully deleted.
    pub async fn delete_file(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<usize> {
        let file_key = path_to_entry_key(path);
        let docs_client = &self.docs;
        let document = docs_client
            .open(*namespace_id)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotOpenReplica
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        let query = iroh_docs::store::Query::single_latest_per_key()
            .key_exact(file_key)
            .build();
        let entry = document
            .get_one(query)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotReadFile
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        let entries_deleted = document
            .del(entry.author(), entry.key().to_vec())
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotDeleteFile
            })?;
        Ok(entries_deleted)
    }

    /// Gets an Iroh entry for a file.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the file.
    ///
    /// * `path` - The path of the file.
    ///
    /// # Returns
    ///
    /// The entry representing the file.
    pub async fn get_entry(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<Entry> {
        let file_key = path_to_entry_key(path);
        let docs_client = &self.docs;
        let document = docs_client
            .open(*namespace_id)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotOpenReplica
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        let query = iroh_docs::store::Query::single_latest_per_key()
            .key_exact(file_key)
            .build();
        let entry = document
            .get_one(query)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotReadFile
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        Ok(entry)
    }

    /// Determines the oldest timestamp of a file.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the file.
    ///
    /// * `path` - The path to the file.
    ///
    /// # Returns
    ///
    /// The timestamp, in microseconds from the Unix epoch, of the oldest entry in the file.
    pub async fn get_oldest_entry_timestamp(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<u64> {
        let file_key = path_to_entry_key(path);
        let docs_client = &self.docs;
        let document = docs_client
            .open(*namespace_id)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotOpenReplica
            })?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        let query = iroh_docs::store::Query::all().key_exact(file_key).build();
        let entries = document.get_many(query).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotListFiles
        })?;
        pin_mut!(entries);
        let timestamps: Vec<u64> = entries
            .map(|entry| entry.unwrap().timestamp())
            .collect()
            .await;
        Ok(*timestamps.par_iter().min().unwrap_or(&u64::MIN))
    }

    /// Reads a file.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the file to read.
    ///
    /// * `path` - The path of the file to read.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn read_file(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<Bytes> {
        let entry = self.get_entry(namespace_id, path).await?;
        Ok(self.content_bytes(&entry).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotReadFile
        })?)
    }

    /// Reads a file.
    ///
    /// # Arguments
    ///
    /// * `document` - A handle to the replica containing the file to read.
    ///
    /// * `path` - The path of the file to read.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn read_file_from_replica_handle(
        &self,
        document: &Doc,
        path: &PathBuf,
    ) -> miette::Result<Bytes> {
        let file_key = path_to_entry_key(path);
        let query = iroh_docs::store::Query::single_latest_per_key()
            .key_exact(file_key)
            .build();
        let entry = document
            .get_one(query)
            .await
            .map_err(|e| miette::miette!("{}", e))?
            .ok_or(OkuFsError::FsEntryNotFound)?;
        self.content_bytes(&entry)
            .await
            .map_err(|e| miette::miette!("{}", e))
    }

    /// Moves a file by copying it to a new location and deleting the original.
    ///
    /// # Arguments
    ///
    /// * `from_namespace_id` - The ID of the replica containing the file to move.
    ///
    /// * `to_namespace_id` - The ID of the replica to move the file to.
    ///
    /// * `from_path` - The path of the file to move.
    ///
    /// * `to_path` - The path to move the file to.
    ///
    /// # Returns
    ///
    /// A tuple containing the hash of the file at the new destination and the number of replica entries deleted during the operation, which should be 1 if the file at the original path was deleted.
    pub async fn move_file(
        &self,
        from_namespace_id: &NamespaceId,
        from_path: &PathBuf,
        to_namespace_id: &NamespaceId,
        to_path: &PathBuf,
    ) -> miette::Result<(Hash, usize)> {
        let data = self.read_file(from_namespace_id, from_path).await?;
        let hash = self
            .create_or_modify_file(to_namespace_id, to_path, data)
            .await?;
        let entries_deleted = self.delete_file(from_namespace_id, from_path).await?;
        Ok((hash, entries_deleted))
    }

    /// Retrieve a file locally after attempting to retrieve the latest version from the Internet.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the file to retrieve.
    ///
    /// * `path` - The path to the file to retrieve.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn fetch_file(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
        filters: &Option<Vec<FilterKind>>,
    ) -> anyhow::Result<Bytes> {
        match self.resolve_namespace_id(namespace_id).await {
            Ok(ticket) => match self.fetch_file_with_ticket(&ticket, path, filters).await {
                Ok(bytes) => Ok(bytes),
                Err(e) => {
                    error!("{}", e);
                    Ok(self
                        .read_file(namespace_id, path)
                        .await
                        .map_err(|e| anyhow!("{}", e))?)
                }
            },
            Err(e) => {
                error!("{}", e);
                Ok(self
                    .read_file(namespace_id, path)
                    .await
                    .map_err(|e| anyhow!("{}", e))?)
            }
        }
    }

    /// Join a swarm to fetch the latest version of a file and save it to the local machine.
    ///
    /// # Arguments
    ///
    /// * `ticket` - A ticket for the replica containing the file to retrieve.
    ///
    /// * `path` - The path to the file to retrieve.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn fetch_file_with_ticket(
        &self,
        ticket: &DocTicket,
        path: &PathBuf,
        filters: &Option<Vec<FilterKind>>,
    ) -> anyhow::Result<Bytes> {
        let docs_client = &self.docs;
        let replica = docs_client
            .import_namespace(ticket.capability.clone())
            .await?;
        let filters = filters
            .clone()
            .unwrap_or(vec![FilterKind::Exact(path_to_entry_key(path))]);
        replica
            .set_download_policy(iroh_docs::store::DownloadPolicy::NothingExcept(filters))
            .await?;
        replica.start_sync(ticket.nodes.clone()).await?;
        let namespace_id = ticket.capability.id();
        let mut events = replica.subscribe().await?;
        let sync_start = std::time::Instant::now();
        while let Some(event) = events.next().await {
            if matches!(event?, LiveEvent::SyncFinished(_)) {
                let elapsed = sync_start.elapsed();
                info!(
                    "Synchronisation took {elapsed:?} for {} … ",
                    crate::fs::util::fmt(namespace_id),
                );
                break;
            }
        }
        self.read_file(&namespace_id, path)
            .await
            .map_err(|e| anyhow!("{}", e))
    }
}
