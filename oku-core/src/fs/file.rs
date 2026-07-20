use super::*;
use crate::error::OkuFsError;
use crate::fs::util::entry_key_to_path;
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
use miette::IntoDiagnostic;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tempfile::NamedTempFile;
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
    ) -> miette::Result<Vec<PathBuf>> {
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
        let files = entries
            .filter_map(|entry| async {
                entry
                    .ok()
                    .map(|x| entry_key_to_path(x.key()).ok())
                    .flatten()
            })
            .collect::<Vec<_>>()
            .await;
        Ok(files)
    }

    /// Creates a file.
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
    pub async fn create_file(
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
        let query = iroh_docs::store::Query::single_latest_per_key()
            .key_exact(&file_key)
            .build();
        let entry = document.get_one(query).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotReadFile
        })?;
        match entry {
            None => {
                // The file doesn't exist
                Ok(document
                    .set_bytes(self.default_author().await, file_key, data)
                    .await
                    .map_err(|e| {
                        error!("{}", e);
                        OkuFsError::CannotCreateOrModifyFile
                    })?)
            }
            Some(_old_hash) => {
                // The file already exists
                let namespace_id_str = crate::fs::util::fmt(namespace_id);
                Err(miette::miette!("File at {path:?} in replica {namespace_id_str} cannot be created as it already exists."))
            }
        }
    }

    /// Creates a file (if it does not exist) or replaces an existing file's contents.
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
    /// The hash of the file, if it was created.
    pub async fn create_or_replace_file(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
        data: impl Into<Bytes>,
    ) -> miette::Result<Option<Hash>> {
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
            .key_exact(&file_key)
            .build();
        let entry = document.get_one(query).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotReadFile
        })?;
        match entry {
            None => {
                // The file doesn't exist
                Ok(Some(
                    document
                        .set_bytes(self.default_author().await, file_key, data)
                        .await
                        .map_err(|e| {
                            error!("{}", e);
                            OkuFsError::CannotCreateOrModifyFile
                        })?,
                ))
            }
            Some(_old_hash) => {
                // The file already exists, so we're modifying it
                self.write_file_using_cache(namespace_id, path, data, &None)
                    .await?;
                Ok(None)
            }
        }
    }

    /// Writes directly to a file in a replica, bypassing the file cache layer.
    /// This is useful when making very small writes, or when making the initial write to a file.
    /// Since this bypasses the file cache layer, if a write is made this way _while_ a file is
    /// being written to in the file cache (ie, if the most recent cache write is within the cache's TTL)
    /// then reading will still read from the cached version of the file, _and_ writes made this way will be overwritten
    /// once the cached file is committed.
    ///
    /// You should only use this method in the following circumstances:
    /// - When this is the first write to a file
    /// - When you're otherwise certain no concurrent (with a granularity of the cache's TTL) writes are being made to this file via the file cache
    pub async fn write_file_using_buffer(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
        data: impl Into<Bytes>,
        seek: &Option<SeekFrom>,
    ) -> miette::Result<Hash> {
        let data = data.into();
        let len = (&data.len()).clone() as u64;
        let file_bytes = self.read_file(namespace_id, path, seek, &Some(len)).await?;
        let mut writer = BufWriter::new(Cursor::new(file_bytes.to_vec()));
        if let Some(seek) = seek {
            writer.seek(*seek).into_diagnostic()?;
        }
        writer.write(&data).into_diagnostic()?;
        let inner_cursor = writer.into_inner().into_diagnostic()?;
        let new_data = inner_cursor.into_inner();

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
            .set_bytes(self.default_author().await, file_key, new_data)
            .await
            .map_err(|e| {
                error!("{}", e);
                OkuFsError::CannotCreateOrModifyFile
            })?;

        Ok(entry_hash)
    }

    pub async fn write_file_using_cache(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
        data: impl Into<Bytes>,
        seek: &Option<SeekFrom>,
    ) -> miette::Result<()> {
        let data = data.into();
        self.file_cache.run_pending_tasks().await;
        let cache_entry = self
            .file_cache
            .get(&(namespace_id.clone(), path.clone()))
            .await;

        let tempfile_lock = cache_entry.clone().unwrap_or(Arc::new(Mutex::new(
            NamedTempFile::with_prefix_in("oku_", "./").into_diagnostic()?,
        )));

        let mut tempfile = tempfile_lock.try_lock().into_diagnostic()?;
        tempfile.seek(SeekFrom::Start(0)).into_diagnostic()?; // Reset seek from previous writes back to start of file

        // If the cache entry doesn't exist, we need to prepare the temporary file.
        if cache_entry.is_none() {
            let entry = self.get_entry(namespace_id, path).await?;
            let current_bytes = self
                .content_bytes(&entry, &None, &None)
                .await
                .map_err(|e| {
                    error!("{}", e);
                    OkuFsError::CannotReadFile
                })?;
            tempfile.write_all(&current_bytes).into_diagnostic()?;
        }

        if let Some(seek) = seek {
            tempfile.seek(*seek).into_diagnostic()?;
        }
        tempfile.write_all(&data).into_diagnostic()
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

    pub async fn get_last_modified(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<u64> {
        self.file_cache.run_pending_tasks().await;
        let cache_entry = self
            .file_cache
            .get(&(namespace_id.clone(), path.clone()))
            .await;

        match cache_entry {
            Some(tempfile_lock) => {
                let mut tempfile: tokio::sync::MutexGuard<'_, NamedTempFile> =
                    tempfile_lock.try_lock().into_diagnostic()?;
                tempfile.seek(SeekFrom::Start(0)).into_diagnostic()?; // Reset seek from previous writes back to start of file
                Ok(tempfile
                    .as_file()
                    .metadata()
                    .into_diagnostic()?
                    .modified()
                    .into_diagnostic()?
                    .duration_since(UNIX_EPOCH)
                    .into_diagnostic()?
                    .as_micros() as u64)
            }
            None => Ok(self.get_entry(namespace_id, path).await?.timestamp()),
        }
    }

    pub async fn get_file_size(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<u64> {
        self.file_cache.run_pending_tasks().await;
        let cache_entry = self
            .file_cache
            .get(&(namespace_id.clone(), path.clone()))
            .await;

        match cache_entry {
            Some(tempfile_lock) => {
                let tempfile = tempfile_lock.try_lock().into_diagnostic()?;
                Ok(tempfile.as_file().metadata().into_diagnostic()?.len())
            }
            None => Ok(self.get_entry(namespace_id, path).await?.content_len()),
        }
    }

    /// Gets the Iroh entries for a file.
    ///
    /// # Arguments
    ///
    /// * `namespace_id` - The ID of the replica containing the file.
    ///
    /// * `path` - The path of the file.
    ///
    /// # Returns
    ///
    /// The entries in the file.
    pub async fn get_entries(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
    ) -> miette::Result<Vec<Entry>> {
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
        let query = iroh_docs::store::Query::key_exact(file_key).build();
        let entries_stream = document.get_many(query).await.map_err(|e| {
            error!("{}", e);
            OkuFsError::CannotListFiles
        })?;
        pin_mut!(entries_stream);
        let entries: Vec<Entry> = entries_stream
            .filter_map(|entry| async move { entry.ok() })
            .collect()
            .await;
        Ok(entries)
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
    /// * `seek` - Optional direction of where in the file to read from.
    ///
    /// * `len` - Optional number of bytes to read.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn read_file(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
        seek: &Option<SeekFrom>,
        len: &Option<u64>,
    ) -> miette::Result<Bytes> {
        self.file_cache.run_pending_tasks().await;
        let cache_entry = self
            .file_cache
            .get(&(namespace_id.clone(), path.clone()))
            .await;

        match cache_entry {
            None => {
                // The file is not in the file cache; we should read it directly from the replica
                let entry = self.get_entry(namespace_id, path).await?;
                Ok(self.content_bytes(&entry, seek, len).await.map_err(|e| {
                    error!("{}", e);
                    OkuFsError::CannotReadFile
                })?)
            }
            Some(tempfile_lock) => {
                let mut tempfile = tempfile_lock.try_lock().into_diagnostic()?;
                let mut buffer = Vec::new();
                let bytes_read = tempfile.read_to_end(&mut buffer).into_diagnostic()?;
                buffer.truncate(bytes_read);
                Ok(buffer.into())
            }
        }
    }

    /// Reads a file.
    ///
    /// # Arguments
    ///
    /// * `document` - A handle to the replica containing the file to read.
    ///
    /// * `path` - The path of the file to read.
    ///
    /// * `seek` - Optional direction of where in the file to read from.
    ///
    /// * `len` - Optional number of bytes to read.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn read_file_from_replica_handle(
        &self,
        document: &Doc,
        path: &PathBuf,
        seek: &Option<SeekFrom>,
        len: &Option<u64>,
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
        self.content_bytes(&entry, seek, len)
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
        let data = self
            .read_file(from_namespace_id, from_path, &None, &None)
            .await?;
        let hash = self.create_file(to_namespace_id, to_path, data).await?;
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
    /// * `seek` - Optional direction of where in the file to read from.
    ///
    /// * `len` - Optional number of bytes to read.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn fetch_file(
        &self,
        namespace_id: &NamespaceId,
        path: &PathBuf,
        filters: &Option<Vec<FilterKind>>,
        seek: &Option<SeekFrom>,
        len: &Option<u64>,
    ) -> anyhow::Result<Bytes> {
        match self.resolve_namespace_id(namespace_id).await {
            Ok(ticket) => match self
                .fetch_file_with_ticket(&ticket, path, filters, seek, len)
                .await
            {
                Ok(bytes) => Ok(bytes),
                Err(e) => {
                    error!("{}", e);
                    Ok(self
                        .read_file(namespace_id, path, seek, len)
                        .await
                        .map_err(|e| anyhow!("{}", e))?)
                }
            },
            Err(e) => {
                error!("{}", e);
                Ok(self
                    .read_file(namespace_id, path, seek, len)
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
    /// * `seek` - Optional direction of where in the file to read from.
    ///
    /// * `len` - Optional number of bytes to read.
    ///
    /// # Returns
    ///
    /// The data read from the file.
    pub async fn fetch_file_with_ticket(
        &self,
        ticket: &DocTicket,
        path: &PathBuf,
        filters: &Option<Vec<FilterKind>>,
        seek: &Option<SeekFrom>,
        len: &Option<u64>,
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
        self.read_file(&namespace_id, path, seek, len)
            .await
            .map_err(|e| anyhow!("{}", e))
    }
}
