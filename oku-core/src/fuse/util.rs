use crate::error::OkuFuseError;
use crate::fs::OkuFs;
use chrono::TimeZone;
use easy_fuser::prelude::FileKind::Directory;
use easy_fuser::prelude::FileKind::RegularFile;
use easy_fuser::prelude::*;
use iroh_docs::sync::Entry;
use iroh_docs::NamespaceId;
use miette::IntoDiagnostic;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

/// Returns whether or not the given path is the root path.
///
/// # Arguments
///
/// * `path` - The path to check.
///
/// # Returns
///
/// `true` if the path is empty or `/`, `false` otherwise.
pub fn is_root_path(path: &Path) -> bool {
    path.is_empty() || path == &PathBuf::from("/")
}

/// Parse a FUSE path to retrieve the replica and path.
///
/// # Arguments
///
/// * `path` - The FUSE path.
///
/// # Returns
///
/// A replica ID, if the FUSE path is not the root directory, and a path in the optional replica.
pub fn parse_fuse_path(path: &Path) -> miette::Result<Option<(NamespaceId, PathBuf)>> {
    let mut components = path.components();
    if let Some(_root) = components.next() {
        if let Some(replica_id) = components.next() {
            let replica_id_string = replica_id.as_os_str().to_str().unwrap_or_default();
            let namespace_id = NamespaceId::from(crate::fs::util::parse_array_hex_or_base32::<32>(
                replica_id_string,
            )?);
            let replica_path = PathBuf::from("/").join(components.as_path()).to_path_buf();
            return Ok(Some((namespace_id, replica_path)));
        } else {
            return Ok(None);
        }
    }
    Err(OkuFuseError::NoRoot.into())
}

/// Determines the immediate contents of a directory.
///
/// # Arguments
///
/// * `prefix_path` - The path to the directory.
///
/// * `files` - The recursive contents of the directory.
///
/// # Returns
///
/// The file system entries in the directory.
pub fn get_immediate_children(
    prefix_path: PathBuf,
    files: Vec<Entry>,
) -> miette::Result<Vec<(OsString, <PathBuf as FileIdType>::MinimalMetadata)>> {
    let mut directory_set: HashSet<OsString> = HashSet::new();
    let mut directory_entries: Vec<(OsString, <PathBuf as FileIdType>::MinimalMetadata)> = vec![
        (std::ffi::OsString::from("."), Directory),
        (std::ffi::OsString::from(".."), Directory),
    ];
    // For all descending files …
    for file in files {
        let file_path = PathBuf::from(std::str::from_utf8(file.key()).unwrap_or_default());
        let stripped_file_path = file_path
            .strip_prefix(prefix_path.clone())
            .into_diagnostic()?;
        let number_of_components = stripped_file_path.components().count();
        if let Some(first_component) = stripped_file_path.components().next() {
            // Check if this file is a direct child of the prefix path
            // If the file isn't a direct child, it must be in a folder under the prefix path
            if number_of_components == 1 {
                directory_entries.push((
                    stripped_file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_os_string(),
                    RegularFile,
                ));
            } else {
                directory_set.insert(first_component.as_os_str().to_os_string());
            }
        }
    }
    for directory in directory_set {
        directory_entries.push((directory, Directory));
    }
    Ok(directory_entries)
}

impl OkuFs {
    /// Determines if the file system entry at a path is a file or a directory.
    ///
    /// # Arguments
    ///
    /// * `path` - The path pointing to the file system entry.
    ///
    /// # Returns
    ///
    /// The file system entry type, being either a file or a directory.
    pub async fn is_file_or_directory(
        &self,
        path: &Path,
    ) -> miette::Result<<PathBuf as FileIdType>::MinimalMetadata> {
        let parsed_path = parse_fuse_path(path)?;
        if let Some((namespace_id, replica_path)) = parsed_path {
            if self.get_entry(&namespace_id, &replica_path).await.is_ok()
                && !is_root_path(&replica_path)
            {
                Ok(RegularFile)
            } else {
                match path.parent() {
                    Some(parent_path) => {
                        let parent_path_buf = parent_path.to_path_buf();
                        if is_root_path(&parent_path_buf) {
                            // The children of the root are the replica directories
                            Ok(Directory)
                        } else {
                            match parse_fuse_path(&parent_path_buf.clone())? {
                                Some((namespace_id, parsed_parent_path)) => {
                                    let parent_children = self
                                        .list_files(
                                            &namespace_id,
                                            &Some(parsed_parent_path.clone()),
                                        )
                                        .await?;
                                    let parent_immediate_children = get_immediate_children(
                                        parsed_parent_path.clone(),
                                        parent_children,
                                    )?;
                                    if parent_immediate_children
                                        .par_iter()
                                        .find_any(|immediate_child| {
                                            immediate_child.0
                                                == path.file_name().unwrap_or_default()
                                        })
                                        .is_some()
                                    {
                                        Ok(Directory)
                                    } else {
                                        Err(OkuFuseError::NoFileAtPath(path.to_path_buf()).into())
                                    }
                                }
                                None => Err(OkuFuseError::NoFileAtPath(path.to_path_buf()).into()),
                            }
                        }
                    }
                    None => Err(OkuFuseError::NoFileAtPath(path.to_path_buf()).into()),
                }
            }
        } else {
            Ok(Directory)
        }
    }

    /// Determines the attributes of a file system entry.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file system entry.
    ///
    /// # Returns
    ///
    /// The attributes of the file system entry.
    pub async fn get_fs_entry_attributes(&self, path: &Path) -> miette::Result<FileAttribute> {
        let parsed_path = parse_fuse_path(path)?;
        if let Some((namespace_id, replica_path)) = parsed_path {
            let fs_entry_permission = match self.get_replica_capability(&namespace_id).await? {
                iroh_docs::CapabilityKind::Read => 0o444u16,
                iroh_docs::CapabilityKind::Write => 0o777u16,
            };
            let fs_entry_type = self.is_file_or_directory(path).await?;
            match fs_entry_type {
                RegularFile => {
                    let file_entry = self.get_entry(&namespace_id, &replica_path).await?;
                    let estimated_creation_time = SystemTime::from(
                        chrono::Utc.timestamp_nanos(
                            (self
                                .get_oldest_entry_timestamp(&namespace_id, &replica_path)
                                .await?)
                                .try_into()
                                .unwrap_or(0),
                        ),
                    );
                    Ok(FileAttribute {
                        size: file_entry.content_len(),
                        blocks: file_entry.content_len() / 512,
                        atime: SystemTime::now(),
                        mtime: SystemTime::from(chrono::Utc.timestamp_nanos(
                            (file_entry.timestamp() * 1000).try_into().unwrap_or(0),
                        )),
                        ctime: estimated_creation_time,
                        crtime: estimated_creation_time,
                        kind: fs_entry_type,
                        perm: fs_entry_permission,
                        nlink: 0,
                        uid: 0,
                        gid: 0,
                        rdev: 0,
                        flags: 0,
                        blksize: 512,
                        ttl: None,
                        generation: None,
                    })
                }
                Directory => {
                    let directory_creation_time_estimate = self
                        .get_oldest_timestamp_in_folder(&namespace_id, &replica_path)
                        .await?;
                    let directory_modification_time_estimate = self
                        .get_newest_timestamp_in_folder(&namespace_id, &replica_path)
                        .await?;
                    let directory_size_estimate =
                        self.get_folder_size(&namespace_id, &replica_path).await?;
                    Ok(FileAttribute {
                        size: directory_size_estimate,
                        blocks: directory_size_estimate / 512,
                        atime: SystemTime::now(),
                        mtime: SystemTime::from(
                            chrono::Utc.timestamp_nanos(
                                (directory_modification_time_estimate * 1000)
                                    .try_into()
                                    .unwrap_or(0),
                            ),
                        ),
                        ctime: SystemTime::from(
                            chrono::Utc.timestamp_nanos(
                                (directory_creation_time_estimate * 1000)
                                    .try_into()
                                    .unwrap_or(0),
                            ),
                        ),
                        crtime: SystemTime::from(
                            chrono::Utc.timestamp_nanos(
                                (directory_creation_time_estimate * 1000)
                                    .try_into()
                                    .unwrap_or(0),
                            ),
                        ),
                        kind: Directory,
                        perm: fs_entry_permission,
                        nlink: 0,
                        uid: 0,
                        gid: 0,
                        rdev: 0,
                        flags: 0,
                        blksize: 512,
                        ttl: None,
                        generation: None,
                    })
                }
                _ => unreachable!(),
            }
        } else if is_root_path(path) {
            let root_creation_time_estimate = self.get_oldest_timestamp().await?;
            let root_modification_time_estimate = self.get_newest_timestamp().await?;
            let root_size_estimate = self.get_size().await?;
            Ok(FileAttribute {
                size: root_size_estimate,
                blocks: root_size_estimate / 512,
                atime: SystemTime::now(),
                mtime: SystemTime::from(
                    chrono::Utc.timestamp_nanos(
                        (root_modification_time_estimate * 1000)
                            .try_into()
                            .unwrap_or(0),
                    ),
                ),
                ctime: SystemTime::from(
                    chrono::Utc.timestamp_nanos(
                        (root_creation_time_estimate * 1000).try_into().unwrap_or(0),
                    ),
                ),
                crtime: SystemTime::from(
                    chrono::Utc.timestamp_nanos(
                        (root_creation_time_estimate * 1000).try_into().unwrap_or(0),
                    ),
                ),
                kind: Directory,
                perm: 0o444u16,
                nlink: 0,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
                blksize: 512,
                ttl: None,
                generation: None,
            })
        } else {
            Err(OkuFuseError::NoFileAtPath(path.to_path_buf()).into())
        }
    }

    /// Calculate file system statistics for an entry, given a path to it.
    ///
    /// # Arguments
    ///
    /// * `path` – The path to the file system entry.
    ///
    /// # Returns
    ///
    /// Statistics regarding the file system entry referenced by the path.
    pub async fn get_fs_entry_stats(&self, path: &Path) -> miette::Result<StatFs> {
        let parsed_path = parse_fuse_path(path)?;
        match parsed_path {
            None => {
                let total_files = {
                    let mut file_count = 0u64;
                    if let Ok(replicas) = self.list_replicas().await {
                        for (replica, _capability_kind, _is_home_replica) in replicas {
                            if let Ok(files) = self.list_files(&replica, &None).await {
                                file_count += files.len().try_into().unwrap_or(0);
                            }
                        }
                    }
                    file_count
                };
                let root_size_estimate = self.get_size().await?;
                Ok(StatFs {
                    total_files: total_files,
                    block_size: 512,
                    max_filename_length: 256,
                    total_blocks: root_size_estimate / 512,
                    free_blocks: 0,
                    free_files: 0,
                    available_blocks: 0,
                    fragment_size: 0,
                })
            }
            Some((namespace_id, replica_path)) => {
                let directory_size_estimate =
                    self.get_folder_size(&namespace_id, &replica_path).await?;
                let total_files = self
                    .list_files(&namespace_id, &Some(replica_path))
                    .await
                    .map(|x| x.len())
                    .unwrap_or(0);
                Ok(StatFs {
                    total_files: total_files as u64,
                    block_size: 512,
                    max_filename_length: 256,
                    total_blocks: directory_size_estimate / 512,
                    free_blocks: 0,
                    free_files: 0,
                    available_blocks: 0,
                    fragment_size: 0,
                })
            }
        }
    }
}
