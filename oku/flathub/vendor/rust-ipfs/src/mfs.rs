//! A mutable filesystem (MFS) layer over immutable UnixFS DAGs.

use crate::path::{IpfsPath, PathRoot};
use crate::repo::{DataStore, DefaultStorage, GCGuard, Repo};
use crate::{Block, Error};
use anyhow::anyhow;
use bytes::Bytes;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::{FutureExt, Stream, StreamExt};
use ipld_core::cid::{Cid, Version};
use multihash_codetable::{Code, MultihashDigest};
use rust_unixfs::dir::builder::{BufferingTreeBuilder, TreeOptions};
use rust_unixfs::dir::{DirLink, NodeDescription, describe};
use rust_unixfs::file::adder::{
    FileAdder, FileBranchLink, build_file_from_leaves, parse_file_branch, rebuild_file_branch,
};
use rust_unixfs::file::visit::IdleFileVisit;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::future::IntoFuture;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::AsyncWriteExt;

/// Datastore key holding the current MFS root Cid.
const ROOT_KEY: &[u8] = b"/mfs/root";

const VERSION: Version = Version::V1;
const HASHER: Code = Code::Sha2_256;
const RAW_LEAF_CODEC: u64 = 0x55;
const DAG_PB_CODEC: u64 = 0x70;
const CHUNK: u64 = 256 * 1024;
const MAX_FILE_SIZE: u64 = 8 << 30;

/// A directory's immediate children. name to target Cid, cumulative dag size used as the link Tsize.
type DirMap = BTreeMap<String, DirEntry>;

#[derive(Debug, Clone, Copy)]
struct DirEntry {
    cid: Cid,
    tsize: u64,
}

/// The kind of an MFS entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfsKind {
    File { size: u64 },
    Directory,
    Symlink,
}

/// A single entry in an MFS directory listing.
#[derive(Debug, Clone)]
pub struct MfsEntry {
    pub name: String,
    pub cid: Cid,
    pub kind: MfsKind,
    /// Logical size: file size for files, cumulative dag size for directories.
    pub size: u64,
}

/// The result of `stat`.
#[derive(Debug, Clone)]
pub struct MfsStat {
    pub cid: Cid,
    pub kind: MfsKind,
    /// Logical size: file size for files, cumulative dag size (link Tsize) for directories.
    pub size: u64,
    /// Total size of the entry's whole dag, including all descendant blocks.
    pub cumulative_size: u64,
    /// Number of links in the entry's root node.
    pub blocks: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum MfsError {
    #[error("'{0}' does not exist")]
    NotFound(String),
    #[error("'{0}' is a directory")]
    IsDirectory(String),
    #[error("'{0}' already exists")]
    AlreadyExists(String),
    #[error("'{0}' is a non-empty directory; pass recursive")]
    NotEmpty(String),
    #[error("'{0}' is a symlink")]
    IsSymlink(String),
    #[error("the write would exceed the MFS file size limit")]
    FileSizeLimit,
    #[error("the root directory cannot be modified")]
    RootImmutable,
}

#[derive(Debug, Clone, Default)]
pub struct WriteOptions {
    pub offset: u64,
    pub create: bool,
    pub parents: bool,
    pub truncate: bool,
}

#[derive(Debug)]
pub enum WriteStatus {
    Progress { written: u64, total: Option<u64> },
    Completed { cid: Cid },
    Failed { error: Error },
}

pub struct MfsWrite {
    inner: BoxStream<'static, WriteStatus>,
}

impl Stream for MfsWrite {
    type Item = WriteStatus;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl IntoFuture for MfsWrite {
    type Output = Result<Cid, Error>;
    type IntoFuture = BoxFuture<'static, Result<Cid, Error>>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            let mut stream = self.inner;
            let mut cid = None;
            while let Some(status) = stream.next().await {
                match status {
                    WriteStatus::Completed { cid: completed } => cid = Some(completed),
                    WriteStatus::Failed { error } => return Err(error),
                    WriteStatus::Progress { .. } => {}
                }
            }
            cid.ok_or_else(|| anyhow!("streaming write produced no result"))
        }
        .boxed()
    }
}

#[derive(Debug)]
pub enum ReadStatus {
    Progress { written: u64, total: u64 },
    Completed { written: u64 },
    Failed { error: Error },
}

pub struct MfsRead {
    inner: BoxStream<'static, ReadStatus>,
}

impl Stream for MfsRead {
    type Item = ReadStatus;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl IntoFuture for MfsRead {
    type Output = Result<u64, Error>;
    type IntoFuture = BoxFuture<'static, Result<u64, Error>>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            let mut stream = self.inner;
            let mut written = 0;
            while let Some(status) = stream.next().await {
                match status {
                    ReadStatus::Completed { written: total } => written = total,
                    ReadStatus::Failed { error } => return Err(error),
                    ReadStatus::Progress { .. } => {}
                }
            }
            Ok(written)
        }
        .boxed()
    }
}

/// Handle to the node's mutable filesystem. Obtain it via [`crate::Ipfs::mfs`].
#[derive(Clone)]
pub struct Mfs {
    repo: Repo<DefaultStorage>,
    shard_threshold: Option<u64>,
}

impl Mfs {
    pub(crate) fn new(repo: Repo<DefaultStorage>) -> Self {
        Self {
            repo,
            shard_threshold: Some(256 * 1024),
        }
    }

    pub fn with_shard_threshold(mut self, threshold: Option<u64>) -> Self {
        self.shard_threshold = threshold;
        self
    }

    fn repo(&self) -> &Repo<DefaultStorage> {
        &self.repo
    }

    /// Returns the current MFS root Cid, or `None` if nothing has been written yet.
    pub async fn root(&self) -> Result<Option<Cid>, Error> {
        let mut guard = self.repo().inner.mfs_root.lock().await;
        self.cached_root(&mut guard).await
    }

    /// Creates a directory at `path`. With `parents`, missing intermediate directories are created;
    /// otherwise a missing parent is an error.
    pub async fn mkdir(&self, path: &str, parents: bool) -> Result<(), Error> {
        let (cid, tsize, blocks) = encode_dir(&DirMap::new(), self.shard_threshold)?;
        self.set_entry(path, DirEntry { cid, tsize }, blocks, parents, false, None)
            .await
    }

    /// Writes `data` as the whole file at `path`, creating or replacing it.
    pub async fn write(&self, path: &str, data: &[u8], parents: bool) -> Result<(), Error> {
        self.write_with(
            path,
            data,
            WriteOptions {
                offset: 0,
                create: true,
                parents,
                truncate: true,
            },
        )
        .await
    }

    /// Writes `data` into the file at `path` starting at `opts.offset`, preserving the surrounding
    /// bytes (read-modify-rewrite). `create` makes a missing file, `truncate` discards existing
    /// content first, `parents` creates missing directories.
    pub async fn write_with(
        &self,
        path: &str,
        data: &[u8],
        opts: WriteOptions,
    ) -> Result<(), Error> {
        let comps = split_path(path)?;
        if comps.is_empty() {
            return Err(MfsError::RootImmutable.into());
        }

        let new_size = opts.offset.saturating_add(data.len() as u64);
        if new_size > MAX_FILE_SIZE {
            return Err(MfsError::FileSizeLimit.into());
        }

        let gc_guard = self.repo().gc_guard().await;
        let mut guard = self.repo().inner.mfs_root.lock().await;

        let existing = self
            .resolve_file_locked(&mut guard, &comps, &gc_guard)
            .await?;
        if existing.is_none() && !opts.create {
            return Err(MfsError::NotFound(path.to_string()).into());
        }

        if let Some((cid, tsize, filesize)) = existing
            && !opts.truncate
            && new_size <= filesize
        {
            let edited = self.overwrite_subtree(cid, 0, data, opts.offset).await?;
            if let Some((new_cid, blocks)) = edited {
                return self
                    .set_entry_locked(
                        &mut guard,
                        path,
                        DirEntry {
                            cid: new_cid,
                            tsize,
                        },
                        blocks,
                        opts.parents,
                        true,
                        &gc_guard,
                    )
                    .await;
            }
        }

        if let Some((cid, _, filesize)) = existing
            && !opts.truncate
            && new_size > filesize
            && let Some((new_cid, new_tsize, blocks)) = self
                .grow_file(cid, filesize, opts.offset, data, &gc_guard)
                .await?
        {
            return self
                .set_entry_locked(
                    &mut guard,
                    path,
                    DirEntry {
                        cid: new_cid,
                        tsize: new_tsize,
                    },
                    blocks,
                    opts.parents,
                    true,
                    &gc_guard,
                )
                .await;
        }

        let mut content = if opts.truncate || existing.is_none() {
            Vec::new()
        } else {
            self.read_existing_file_locked(&mut guard, &comps, &gc_guard)
                .await?
                .unwrap_or_default()
        };
        let offset = opts.offset as usize;
        let end = offset + data.len();
        if content.len() < end {
            content.resize(end, 0);
        }
        content[offset..end].copy_from_slice(data);

        let (cid, tsize, blocks) = encode_file(&content)?;
        self.set_entry_locked(
            &mut guard,
            path,
            DirEntry { cid, tsize },
            blocks,
            opts.parents,
            true,
            &gc_guard,
        )
        .await
    }

    /// Sets the file at `path` to exactly `size` bytes, truncating or zero-extending.
    pub async fn truncate(&self, path: &str, size: u64) -> Result<(), Error> {
        let comps = split_path(path)?;
        if comps.is_empty() {
            return Err(MfsError::RootImmutable.into());
        }
        if size > MAX_FILE_SIZE {
            return Err(MfsError::FileSizeLimit.into());
        }

        let gc_guard = self.repo().gc_guard().await;
        let mut guard = self.repo().inner.mfs_root.lock().await;
        let (file_cid, _, filesize) = self
            .resolve_file_locked(&mut guard, &comps, &gc_guard)
            .await?
            .ok_or_else(|| MfsError::NotFound(path.to_string()))?;
        if size == filesize {
            return Ok(());
        }

        let boundary = (size.min(filesize) / CHUNK) * CHUNK;
        let read_end = size.min(filesize);
        let result = {
            let mut new_tail = if boundary < read_end {
                self.read_file_range(&file_cid, boundary, read_end).await?
            } else {
                Vec::new()
            };
            new_tail.resize((size - boundary) as usize, 0);
            self.rebuild_from_boundary(file_cid, filesize, boundary, new_tail)
                .await?
        };

        let (cid, tsize, blocks) = match result {
            Some(r) => r,
            None => {
                let mut content = self
                    .read_existing_file_locked(&mut guard, &comps, &gc_guard)
                    .await?
                    .unwrap_or_default();
                content.resize(size as usize, 0);
                encode_file(&content)?
            }
        };
        self.set_entry_locked(
            &mut guard,
            path,
            DirEntry { cid, tsize },
            blocks,
            false,
            true,
            &gc_guard,
        )
        .await
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Write the whole content of `file` at `path`.
    pub fn write_from_file(&self, path: &str, file: impl AsRef<Path>, parents: bool) -> MfsWrite {
        let mfs = self.clone();
        let path = path.to_string();
        let file = file.as_ref().to_path_buf();

        let inner = async_stream::stream! {
            let meta = match tokio::fs::metadata(&file).await {
                Ok(meta) => meta,
                Err(e) => {
                    yield WriteStatus::Failed { error: e.into() };
                    return;
                }
            };
            if !meta.is_file() {
                yield WriteStatus::Failed {
                    error: std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("'{}' is not a valid file", file.display()),
                    )
                    .into(),
                };
                return;
            }
            if meta.len() > MAX_FILE_SIZE {
                yield WriteStatus::Failed {
                    error: std::io::Error::new(
                        std::io::ErrorKind::FileTooLarge,
                        format!("'{}' is too large", file.display()),
                    )
                    .into(),
                };
                return;
            }
            let opened = match tokio::fs::File::open(&file).await {
                Ok(opened) => opened,
                Err(e) => {
                    yield WriteStatus::Failed { error: e.into() };
                    return;
                }
            };
            let reader = tokio_util::io::ReaderStream::new(opened).boxed();
            for await status in mfs.write_progress(path, reader, parents, Some(meta.len())) {
                yield status;
            }
        };

        MfsWrite {
            inner: inner.boxed(),
        }
    }

    /// Writes the whole file at `path` from a byte stream without buffering the content, creating or
    /// replacing it.
    pub fn write_stream<S>(&self, path: &str, stream: S, parents: bool) -> MfsWrite
    where
        S: Stream<Item = std::io::Result<Bytes>> + Send + 'static,
    {
        let inner = self
            .clone()
            .write_progress(path.to_string(), stream.boxed(), parents, None);
        MfsWrite {
            inner: inner.boxed(),
        }
    }

    fn write_progress(
        self,
        path: String,
        mut stream: BoxStream<'static, std::io::Result<Bytes>>,
        parents: bool,
        total: Option<u64>,
    ) -> impl Stream<Item = WriteStatus> {
        async_stream::stream! {
            let gc_guard = self.repo().gc_guard().await;
            let mut adder = FileAdder::builder()
                .with_cid_version(VERSION)
                .with_hasher(HASHER)
                .build();
            let mut tsize = 0u64;
            let mut written = 0u64;
            let mut root = None;

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        yield WriteStatus::Failed { error: e.into() };
                        return;
                    }
                };
                let mut offset = 0;
                while offset < chunk.len() {
                    let (ready, consumed) = adder.push(&chunk[offset..]);
                    let mut batch = Vec::new();
                    for (cid, block) in ready {
                        tsize += block.len() as u64;
                        root = Some(cid);
                        match Block::new(cid, block) {
                            Ok(block) => batch.push(block),
                            Err(e) => {
                                yield WriteStatus::Failed { error: e.into() };
                                return;
                            }
                        }
                    }
                    if !batch.is_empty()
                        && let Err(e) = self.repo().put_blocks(batch).with_gc_guard(gc_guard.clone()).await
                    {
                        yield WriteStatus::Failed { error: e };
                        return;
                    }
                    offset += consumed;
                    written += consumed as u64;
                    if written > MAX_FILE_SIZE {
                        yield WriteStatus::Failed { error: MfsError::FileSizeLimit.into() };
                        return;
                    }
                }
                yield WriteStatus::Progress { written, total };
            }

            let mut batch = Vec::new();
            for (cid, block) in adder.finish() {
                tsize += block.len() as u64;
                root = Some(cid);
                match Block::new(cid, block) {
                    Ok(block) => batch.push(block),
                    Err(e) => {
                        yield WriteStatus::Failed { error: e.into() };
                        return;
                    }
                }
            }
            if !batch.is_empty()
                && let Err(e) = self.repo().put_blocks(batch).with_gc_guard(gc_guard.clone()).await
            {
                yield WriteStatus::Failed { error: e };
                return;
            }

            let cid = match root {
                Some(cid) => cid,
                None => {
                    yield WriteStatus::Failed { error: anyhow!("file produced no blocks") };
                    return;
                }
            };

            match self
                .set_entry(&path, DirEntry { cid, tsize }, Vec::new(), parents, true, Some(gc_guard))
                .await
            {
                Ok(()) => yield WriteStatus::Completed { cid },
                Err(e) => yield WriteStatus::Failed { error: e },
            }
        }
    }

    /// Reads the whole content of the file at `path`.
    pub async fn read(&self, path: &str) -> Result<Vec<u8>, Error> {
        self.read_range(path, 0, None).await
    }

    /// Reads `count` bytes (or to EOF if `None`) of the file at `path`, starting at `offset`.
    pub async fn read_range(
        &self,
        path: &str,
        offset: u64,
        count: Option<u64>,
    ) -> Result<Vec<u8>, Error> {
        let stream = self.read_stream_range(path, offset, count);
        futures::pin_mut!(stream);
        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            out.extend_from_slice(&chunk?);
        }
        Ok(out)
    }

    /// Streams the content of the file at `path` as it is read, without buffering the whole file.
    pub fn read_stream(&self, path: &str) -> impl Stream<Item = Result<Bytes, Error>> {
        self.read_stream_range(path, 0, None)
    }

    /// Streams `count` bytes (or to EOF if `None`) of the file at `path`, starting at `offset`,
    /// reading only the blocks that overlap the range.
    pub fn read_stream_range(
        &self,
        path: &str,
        offset: u64,
        count: Option<u64>,
    ) -> impl Stream<Item = Result<Bytes, Error>> {
        let mfs = self.clone();
        let path = path.to_string();
        let end = count.map_or(u64::MAX, |c| offset.saturating_add(c));

        async_stream::try_stream! {
            let comps = split_path(&path)?;
            if comps.is_empty() {
                Err::<(), _>(anyhow!("cannot read the root directory"))?;
            }
            let _gc = mfs.repo().gc_guard().await;
            let root = mfs
                .snapshot_root()
                .await?
                .ok_or_else(|| anyhow!("MFS is empty"))?;
            let cid = mfs.resolve_from(root, &comps).await?.0;

            let block = mfs.get_block(&cid).await?;
            match describe_block(&block) {
                NodeDescription::Directory { .. } | NodeDescription::HamtShard { .. } => {
                    Err::<(), _>(MfsError::IsDirectory(path.clone()))?;
                }
                NodeDescription::Symlink => {
                    Err::<(), _>(MfsError::IsSymlink(path.clone()))?;
                }
                NodeDescription::Other => {
                    let leaf = block.data();
                    let start = (offset as usize).min(leaf.len());
                    let stop = match count {
                        Some(c) => start.saturating_add(c as usize).min(leaf.len()),
                        None => leaf.len(),
                    };
                    if start < stop {
                        yield Bytes::copy_from_slice(&leaf[start..stop]);
                    }
                }
                _ => {
                    let mut cache = None;
                    let (content, _, _, mut step) = IdleFileVisit::default()
                        .with_target_range(offset..end)
                        .start(block.data())?;
                    if !content.is_empty() {
                        yield Bytes::copy_from_slice(content);
                    }
                    while let Some(visit) = step {
                        let next = *visit.pending_links().0;
                        let block = mfs.get_block(&next).await?;
                        let (content, next_step) = visit.continue_walk(block.data(), &mut cache)?;
                        if !content.is_empty() {
                            yield Bytes::copy_from_slice(content);
                        }
                        step = next_step;
                    }
                }
            }
        }
    }

    /// Reads the file at `path` and writes it to the local filesystem at `dest`, streaming through
    /// without buffering. The returned [`MfsRead`] is both a progress [`Stream`] and a future
    /// resolving to the number of bytes written.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_to_file(&self, path: &str, dest: impl AsRef<Path>) -> MfsRead {
        let mfs = self.clone();
        let path = path.to_string();
        let dest = dest.as_ref().to_path_buf();

        let inner = async_stream::stream! {
            let total = match mfs.stat(&path).await {
                Ok(stat) => match stat.kind {
                    MfsKind::File { size } => size,
                    _ => {
                        yield ReadStatus::Failed { error: anyhow!("'{path}' is not a file") };
                        return;
                    }
                },
                Err(e) => {
                    yield ReadStatus::Failed { error: e };
                    return;
                }
            };

            let mut file = match tokio::fs::File::create(&dest).await {
                Ok(file) => file,
                Err(e) => {
                    yield ReadStatus::Failed { error: e.into() };
                    return;
                }
            };

            let mut written = 0u64;
            let stream = mfs.read_stream(&path);
            futures::pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        yield ReadStatus::Failed { error: e };
                        return;
                    }
                };
                if let Err(e) = file.write_all(&chunk).await {
                    yield ReadStatus::Failed { error: e.into() };
                    return;
                }
                written += chunk.len() as u64;
                yield ReadStatus::Progress { written, total };
            }

            if let Err(e) = file.flush().await {
                yield ReadStatus::Failed { error: e.into() };
                return;
            }
            yield ReadStatus::Completed { written };
        };

        MfsRead {
            inner: inner.boxed(),
        }
    }

    /// Removes the entry at `path`. A non-empty directory requires `recursive`.
    pub async fn rm(&self, path: &str, recursive: bool) -> Result<(), Error> {
        self.rm_inner(path, recursive, false).await
    }

    /// Removes the entry at `path` if present, succeeding when it is already absent.
    pub async fn rm_force(&self, path: &str, recursive: bool) -> Result<(), Error> {
        self.rm_inner(path, recursive, true).await
    }

    async fn rm_inner(&self, path: &str, recursive: bool, force: bool) -> Result<(), Error> {
        let comps = split_path(path)?;
        let Some((name, dirs)) = comps.split_last() else {
            return Err(MfsError::RootImmutable.into());
        };

        let gc_guard = self.repo().gc_guard().await;
        let mut guard = self.repo().inner.mfs_root.lock().await;
        let (mut frames, names) = match self.load_chain(&mut guard, dirs, false).await {
            Ok(chain) => chain,
            Err(e) if force && is_not_found(&e) => return Ok(()),
            Err(e) => return Err(e),
        };

        let entry = match frames.last().expect("root frame").get(name).copied() {
            Some(entry) => entry,
            None if force => return Ok(()),
            None => return Err(MfsError::NotFound(path.to_string()).into()),
        };

        if !recursive
            && let Ok(map) = self.load_dir(&entry.cid).await
            && !map.is_empty()
        {
            return Err(MfsError::NotEmpty(path.to_string()).into());
        }

        frames.last_mut().expect("root frame").remove(name);
        self.reencode_and_commit(&mut guard, frames, names, Vec::new(), &gc_guard)
            .await
    }

    /// Copies `from` to the MFS path `to`. `from` is either another MFS path or an `/ipfs` (or
    /// `/ipld`) path, in which case the referenced DAG is fetched locally and imported.
    pub async fn cp(&self, from: &str, to: &str, parents: bool) -> Result<(), Error> {
        let (cid, tsize) = if is_ipfs_path(from) {
            let path = IpfsPath::from_str(from)?;
            let root = match path.root() {
                PathRoot::Ipld(cid) => *cid,
                _ => return Err(anyhow!("cp source must resolve to an /ipfs or /ipld cid")),
            };
            let sub: Vec<String> = path.iter().map(|s| s.to_string()).collect();
            let source = self.resolve_ipfs(root, &sub).await?;
            let tsize = self.fetch_and_measure(source).await?;
            (source, tsize)
        } else {
            let from_comps = split_path(from)?;
            self.resolve(&from_comps).await?
        };

        let (dest, _) = self.resolve_dest(to, from).await?;
        self.set_entry(
            &dest,
            DirEntry { cid, tsize },
            Vec::new(),
            parents,
            false,
            None,
        )
        .await
    }

    /// Moves the MFS entry at `from` to `to`. Non-atomic: it links the destination then unlinks the
    /// source as two commits, so a failure between them leaves the entry at both paths (never lost).
    pub async fn mv(&self, from: &str, to: &str, parents: bool) -> Result<(), Error> {
        let from_comps = split_path(from)?;
        if from_comps.is_empty() {
            return Err(MfsError::RootImmutable.into());
        }
        let (cid, tsize) = self.resolve(&from_comps).await?;
        let (dest, into_dir) = self.resolve_dest(to, from).await?;
        let dest_comps = split_path(&dest)?;
        if dest_comps == from_comps {
            return Ok(());
        }
        if dest_comps.len() > from_comps.len() && dest_comps[..from_comps.len()] == from_comps[..] {
            return Err(anyhow!("cannot move '{from}' into its own subdirectory"));
        }
        // link at the destination first (safe to fail), then unlink the source
        self.set_entry(
            &dest,
            DirEntry { cid, tsize },
            Vec::new(),
            parents,
            !into_dir,
            None,
        )
        .await?;
        self.rm(from, true).await
    }

    /// Lists the immediate entries of the directory at `path` (`/` for the root). A file path lists
    /// the single file itself, matching `ipfs files ls`.
    pub async fn ls(&self, path: &str) -> Result<Vec<MfsEntry>, Error> {
        let comps = split_path(path)?;
        let Some(root) = self.snapshot_root().await? else {
            if comps.is_empty() {
                return Ok(Vec::new());
            }
            return Err(anyhow!("MFS is empty"));
        };

        let _gc = self.repo().gc_guard().await;
        let (cid, tsize) = self.resolve_from(root, &comps).await?;
        let block = self.get_block(&cid).await?;

        let map = match describe_block(&block) {
            NodeDescription::Directory { links } => links_to_map(links),
            NodeDescription::HamtShard { links } => {
                let mut map = DirMap::new();
                self.collect_shard(links, &mut map).await?;
                map
            }
            _ => {
                let name = comps.last().cloned().unwrap_or_default();
                let kind = self.classify(&cid, tsize).await?;
                return Ok(vec![MfsEntry {
                    name,
                    cid,
                    kind,
                    size: entry_size(kind, tsize),
                }]);
            }
        };

        let mut out = Vec::with_capacity(map.len());
        for (name, entry) in map {
            let kind = self.classify(&entry.cid, entry.tsize).await?;
            out.push(MfsEntry {
                name,
                cid: entry.cid,
                kind,
                size: entry_size(kind, entry.tsize),
            });
        }
        Ok(out)
    }

    /// Returns type and size information for the entry at `path`.
    pub async fn stat(&self, path: &str) -> Result<MfsStat, Error> {
        let comps = split_path(path)?;
        let Some(root) = self.snapshot_root().await? else {
            if comps.is_empty() {
                let (cid, tsize, _) = encode_dir(&DirMap::new(), self.shard_threshold)?;
                return Ok(MfsStat {
                    cid,
                    kind: MfsKind::Directory,
                    size: tsize,
                    cumulative_size: tsize,
                    blocks: 0,
                });
            }
            return Err(anyhow!("MFS is empty"));
        };

        let _gc = self.repo().gc_guard().await;
        let (cid, tsize) = self.resolve_from(root, &comps).await?;
        let block = self.get_block(&cid).await?;
        let (kind, blocks, link_tsize) = match describe_block(&block) {
            NodeDescription::Directory { links } | NodeDescription::HamtShard { links } => {
                let sum = links.iter().map(|l| l.tsize).sum();
                (MfsKind::Directory, links.len(), sum)
            }
            NodeDescription::Symlink => (MfsKind::Symlink, 0, 0),
            NodeDescription::File { size } => (
                MfsKind::File { size },
                parse_file_branch(block.data()).map_or(0, |b| b.links.len()),
                0,
            ),
            NodeDescription::Other => (
                MfsKind::File {
                    size: block.data().len() as u64,
                },
                0,
                0,
            ),
        };

        // the root carries no parent link, so reconstruct its Tsize the way a parent would store
        // it (block + summed child Tsizes), consistent with what every non-root dir reports
        let cumulative_size = if comps.is_empty() {
            block.data().len() as u64 + link_tsize
        } else {
            tsize
        };
        Ok(MfsStat {
            cid,
            kind,
            size: entry_size(kind, cumulative_size),
            cumulative_size,
            blocks,
        })
    }

    /// Returns whether anything exists at `path`.
    pub async fn exists(&self, path: &str) -> Result<bool, Error> {
        let comps = split_path(path)?;
        Ok(self.resolve_kind(&comps).await?.is_some())
    }

    /// Resolves a path to its kind, or `None` if nothing is there. The root is always a directory.
    async fn resolve_kind(&self, comps: &[String]) -> Result<Option<MfsKind>, Error> {
        let Some(root) = self.snapshot_root().await? else {
            return Ok(comps.is_empty().then_some(MfsKind::Directory));
        };
        let _gc = self.repo().gc_guard().await;
        match self.resolve_from(root, comps).await {
            Ok((cid, tsize)) => Ok(Some(self.classify(&cid, tsize).await?)),
            Err(_) => Ok(None),
        }
    }

    /// Resolves the destination path for `cp`/`mv`. When `to` ends in `/` or names an existing
    /// directory, the source basename is appended (so `cp /a/f /dir` lands at `/dir/f`).
    async fn resolve_dest(&self, to: &str, source: &str) -> Result<(String, bool), Error> {
        let trailing = to.ends_with('/');
        let mut comps = split_path(to)?;
        let into_dir =
            trailing || matches!(self.resolve_kind(&comps).await?, Some(MfsKind::Directory));
        if into_dir {
            let name = base_name(source).ok_or_else(|| anyhow!("cp/mv source has no name"))?;
            comps.push(name.to_string());
        }
        Ok((format!("/{}", comps.join("/")), into_dir))
    }

    /// Loads the current root Cid, reading the datastore on first access. The cache distinguishes
    /// "not loaded yet" (`bool == false`) from "loaded, no root" (`None`).
    async fn cached_root(&self, guard: &mut (bool, Option<Cid>)) -> Result<Option<Cid>, Error> {
        if !guard.0 {
            guard.1 = match self.repo().data_store().get(ROOT_KEY).await? {
                Some(bytes) => Some(Cid::try_from(bytes.as_slice())?),
                None => None,
            };
            guard.0 = true;
        }
        Ok(guard.1)
    }

    /// A point-in-time snapshot of the root Cid for read operations.
    async fn snapshot_root(&self) -> Result<Option<Cid>, Error> {
        let mut guard = self.repo().inner.mfs_root.lock().await;
        self.cached_root(&mut guard).await
    }

    /// Inserts or replaces the entry at `path` with `entry`, storing `entry_blocks`. With `parents`,
    /// missing intermediate directories are created. When `overwrite` is false an existing entry is
    /// an error.
    async fn set_entry(
        &self,
        path: &str,
        entry: DirEntry,
        entry_blocks: Vec<Block>,
        parents: bool,
        overwrite: bool,
        gc_guard: Option<GCGuard>,
    ) -> Result<(), Error> {
        let gc_guard = match gc_guard {
            Some(guard) => guard,
            None => self.repo().gc_guard().await,
        };
        let mut guard = self.repo().inner.mfs_root.lock().await;
        self.set_entry_locked(
            &mut guard,
            path,
            entry,
            entry_blocks,
            parents,
            overwrite,
            &gc_guard,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn set_entry_locked(
        &self,
        guard: &mut (bool, Option<Cid>),
        path: &str,
        entry: DirEntry,
        entry_blocks: Vec<Block>,
        parents: bool,
        overwrite: bool,
        gc_guard: &GCGuard,
    ) -> Result<(), Error> {
        let comps = split_path(path)?;
        let Some((name, dirs)) = comps.split_last() else {
            return Err(MfsError::RootImmutable.into());
        };

        let (mut frames, names) = self.load_chain(guard, dirs, parents).await?;

        let parent = frames.last_mut().expect("root frame");
        if !overwrite && parent.contains_key(name) {
            return Err(MfsError::AlreadyExists(path.to_string()).into());
        }
        parent.insert(name.clone(), entry);

        self.reencode_and_commit(guard, frames, names, entry_blocks, gc_guard)
            .await
    }

    /// Loads the chain of directory maps from the root along `dirs`. `frames[0]` is the root and
    /// `names[i]` is the component linking `frames[i]` to `frames[i+1]`. With `create`, missing
    /// intermediate directories become empty maps.
    async fn load_chain(
        &self,
        guard: &mut (bool, Option<Cid>),
        dirs: &[String],
        create: bool,
    ) -> Result<(Vec<DirMap>, Vec<String>), Error> {
        let root_map = match self.cached_root(guard).await? {
            Some(cid) => self.load_dir(&cid).await?,
            None => DirMap::new(),
        };

        let mut frames = vec![root_map];
        let mut names = Vec::with_capacity(dirs.len());

        for comp in dirs {
            let next = match frames.last().expect("non-empty").get(comp).copied() {
                Some(entry) => self.load_dir(&entry.cid).await?,
                None if create => DirMap::new(),
                None => return Err(MfsError::NotFound(comp.clone()).into()),
            };
            names.push(comp.clone());
            frames.push(next);
        }

        Ok((frames, names))
    }

    /// Re-encodes every frame bottom-up (each parent's link updated to the child's new Cid), stores
    /// all produced blocks, and commits the new root.
    async fn reencode_and_commit(
        &self,
        guard: &mut (bool, Option<Cid>),
        mut frames: Vec<DirMap>,
        names: Vec<String>,
        mut blocks: Vec<Block>,
        gc_guard: &GCGuard,
    ) -> Result<(), Error> {
        for i in (0..frames.len()).rev() {
            let (cid, tsize, mut blks) = encode_dir(&frames[i], self.shard_threshold)?;
            blocks.append(&mut blks);
            if i == 0 {
                return self.commit_root(guard, cid, blocks, gc_guard).await;
            }
            frames[i - 1].insert(names[i - 1].clone(), DirEntry { cid, tsize });
        }
        unreachable!("frames always contains the root at index 0")
    }

    /// Stores and pins the new tree, saves its root, then unpins the old tree.
    async fn commit_root(
        &self,
        guard: &mut (bool, Option<Cid>),
        new_root: Cid,
        blocks: Vec<Block>,
        gc_guard: &GCGuard,
    ) -> Result<(), Error> {
        let old = guard.1;

        self.repo()
            .put_blocks(blocks)
            .with_gc_guard(gc_guard.clone())
            .await?;
        self.repo()
            .pin(new_root)
            .with_gc_guard(gc_guard.clone())
            .recursive()
            .await?;
        self.repo()
            .data_store()
            .put(ROOT_KEY, &new_root.to_bytes())
            .await?;

        guard.1 = Some(new_root);
        guard.0 = true;

        if let Some(old) = old
            && old != new_root
        {
            // The new tree is pinned, so a failed unpin only delays GC.
            let _ = self
                .repo()
                .remove_pin(old)
                .with_gc_guard(gc_guard.clone())
                .recursive()
                .await;
        }
        Ok(())
    }

    /// Resolves a path (component list, empty == root) to its `(cid, link tsize)`, holding off GC
    /// for the duration of the navigation.
    async fn resolve(&self, comps: &[String]) -> Result<(Cid, u64), Error> {
        let root = self
            .snapshot_root()
            .await?
            .ok_or_else(|| anyhow!("MFS is empty"))?;
        let _gc = self.repo().gc_guard().await;
        self.resolve_from(root, comps).await
    }

    /// Navigates from `root` along `comps`. The caller must hold a GC guard, since this reads
    /// immutable blocks with `get_block_now`.
    async fn resolve_from(&self, root: Cid, comps: &[String]) -> Result<(Cid, u64), Error> {
        let mut cid = root;
        let mut tsize = 0;
        for comp in comps {
            let map = self.load_dir(&cid).await?;
            let entry = map
                .get(comp)
                .ok_or_else(|| anyhow!("path not found: {comp}"))?;
            cid = entry.cid;
            tsize = entry.tsize;
        }
        Ok((cid, tsize))
    }

    /// Loads a directory's children, flattening a HAMT shard if present. Errors on a non-directory.
    async fn load_dir(&self, cid: &Cid) -> Result<DirMap, Error> {
        let block = self.get_block(cid).await?;
        match describe_block(&block) {
            NodeDescription::Directory { links } => Ok(links_to_map(links)),
            NodeDescription::HamtShard { links } => {
                let mut map = DirMap::new();
                self.collect_shard(links, &mut map).await?;
                Ok(map)
            }
            _ => Err(anyhow!("{cid} is not a directory")),
        }
    }

    async fn resolve_ipfs(&self, root: Cid, sub: &[String]) -> Result<Cid, Error> {
        let mut cid = root;
        for seg in sub {
            cid = self
                .resolve_name(cid, seg)
                .await?
                .ok_or_else(|| anyhow!("path not found in source: {seg}"))?;
        }
        Ok(cid)
    }

    async fn resolve_name(&self, dir_cid: Cid, name: &str) -> Result<Option<Cid>, Error> {
        use rust_unixfs::dir::{MaybeResolved, resolve};

        if dir_cid.codec() != DAG_PB_CODEC {
            return Err(anyhow!("{dir_cid} is not a directory"));
        }
        let block = self.repo().get_block(dir_cid).await?;
        let mut cache = None;
        let mut step = resolve(block.data(), name, &mut cache)?;
        loop {
            match step {
                MaybeResolved::Found(cid) => return Ok(Some(cid)),
                MaybeResolved::NotFound => return Ok(None),
                MaybeResolved::NeedToLoadMore(lookup) => {
                    let next = *lookup.pending_links().0;
                    if next.codec() != DAG_PB_CODEC {
                        return Err(anyhow!("malformed HAMT shard under {next}"));
                    }
                    let block = self.repo().get_block(next).await?;
                    step = lookup.continue_walk(block.data(), &mut cache)?;
                }
            }
        }
    }

    async fn fetch_and_measure(&self, root: Cid) -> Result<u64, Error> {
        let mut total = 0u64;
        let mut seen = HashSet::new();
        let mut stack = vec![root];
        while let Some(cid) = stack.pop() {
            if !seen.insert(cid) {
                continue;
            }
            let block = self.repo().get_block(cid).await?;
            total += block.data().len() as u64;
            let mut refs = BTreeSet::new();
            let _ = block.references(&mut refs);
            stack.extend(refs);
        }
        Ok(total)
    }

    fn collect_shard<'a>(
        &'a self,
        links: Vec<DirLink>,
        map: &'a mut DirMap,
    ) -> BoxFuture<'a, Result<(), Error>> {
        async move {
            for link in links {
                if is_shard_prefix(&link.name) {
                    let block = self.get_block(&link.target).await?;
                    match describe_block(&block) {
                        NodeDescription::HamtShard { links } => {
                            self.collect_shard(links, map).await?;
                        }
                        _ => return Err(anyhow!("malformed HAMT shard under {}", link.target)),
                    }
                } else {
                    let name = link.name.get(2..).unwrap_or_default().to_string();
                    map.insert(
                        name,
                        DirEntry {
                            cid: link.target,
                            tsize: link.tsize,
                        },
                    );
                }
            }
            Ok(())
        }
        .boxed()
    }

    async fn classify(&self, cid: &Cid, _link_tsize: u64) -> Result<MfsKind, Error> {
        let block = self.get_block(cid).await?;
        Ok(match describe_block(&block) {
            NodeDescription::Directory { .. } | NodeDescription::HamtShard { .. } => {
                MfsKind::Directory
            }
            NodeDescription::File { size } => MfsKind::File { size },
            NodeDescription::Symlink => MfsKind::Symlink,
            NodeDescription::Other => MfsKind::File {
                size: block.data().len() as u64,
            },
        })
    }

    async fn resolve_file_locked(
        &self,
        guard: &mut (bool, Option<Cid>),
        comps: &[String],
        _gc_guard: &GCGuard,
    ) -> Result<Option<(Cid, u64, u64)>, Error> {
        let Some(root) = self.cached_root(guard).await? else {
            return Ok(None);
        };
        let (cid, tsize) = match self.resolve_from(root, comps).await {
            Ok(resolved) => resolved,
            Err(_) => return Ok(None),
        };
        match self.classify(&cid, tsize).await? {
            MfsKind::File { size } => Ok(Some((cid, tsize, size))),
            MfsKind::Symlink => Err(MfsError::IsSymlink(format!("/{}", comps.join("/"))).into()),
            MfsKind::Directory => {
                Err(MfsError::IsDirectory(format!("/{}", comps.join("/"))).into())
            }
        }
    }

    #[allow(clippy::type_complexity)]
    fn overwrite_subtree<'a>(
        &'a self,
        cid: Cid,
        node_start: u64,
        data: &'a [u8],
        data_start: u64,
    ) -> BoxFuture<'a, Result<Option<(Cid, Vec<Block>)>, Error>> {
        async move {
            let block = self.get_block(&cid).await?;

            if cid.codec() == DAG_PB_CODEC
                && let Some(branch) = parse_file_branch(block.data())
            {
                let mut blocks = Vec::new();
                let mut links: Vec<FileBranchLink> = branch.links.clone();
                let write_end = data_start + data.len() as u64;
                let mut child_start = node_start;
                for (i, link) in branch.links.iter().enumerate() {
                    let child_end = child_start + link.blocksize;
                    if child_end > data_start && child_start < write_end {
                        match self
                            .overwrite_subtree(link.cid, child_start, data, data_start)
                            .await?
                        {
                            Some((new_cid, mut child_blocks)) => {
                                links[i].cid = new_cid;
                                blocks.append(&mut child_blocks);
                            }
                            None => return Ok(None),
                        }
                    }
                    child_start = child_end;
                }
                let (new_cid, bytes) =
                    rebuild_file_branch(branch.filesize, &links, VERSION, HASHER);
                blocks.push(Block::new(new_cid, bytes)?);
                return Ok(Some((new_cid, blocks)));
            }

            if cid.codec() != RAW_LEAF_CODEC {
                return Ok(None);
            }

            let leaf = block.data();
            let ov_start = node_start.max(data_start);
            let ov_end = (node_start + leaf.len() as u64).min(data_start + data.len() as u64);
            if ov_start >= ov_end {
                return Ok(Some((cid, Vec::new())));
            }
            let mut new_leaf = leaf.to_vec();
            let dst = (ov_start - node_start) as usize..(ov_end - node_start) as usize;
            let src = (ov_start - data_start) as usize..(ov_end - data_start) as usize;
            new_leaf[dst].copy_from_slice(&data[src]);
            let new_cid = Cid::new_v1(RAW_LEAF_CODEC, HASHER.digest(&new_leaf));
            Ok(Some((new_cid, vec![Block::new(new_cid, new_leaf)?])))
        }
        .boxed()
    }

    async fn read_file_range(&self, cid: &Cid, start: u64, end: u64) -> Result<Vec<u8>, Error> {
        let block = self.get_block(cid).await?;
        if matches!(describe_block(&block), NodeDescription::Other) {
            let leaf = block.data();
            let s = (start as usize).min(leaf.len());
            let e = (end as usize).min(leaf.len());
            return Ok(leaf[s..e].to_vec());
        }

        let mut out = Vec::new();
        let mut cache = None;
        let (content, _, _, mut step) = IdleFileVisit::default()
            .with_target_range(start..end)
            .start(block.data())?;
        out.extend_from_slice(content);
        while let Some(visit) = step {
            let next = *visit.pending_links().0;
            let block = self.get_block(&next).await?;
            let (content, next_step) = visit.continue_walk(block.data(), &mut cache)?;
            out.extend_from_slice(content);
            step = next_step;
        }
        Ok(out)
    }

    fn collect_prefix_leaves<'a>(
        &'a self,
        cid: Cid,
        node_start: u64,
        boundary: u64,
        out: &'a mut Vec<FileBranchLink>,
    ) -> BoxFuture<'a, Result<bool, Error>> {
        async move {
            if cid.codec() != DAG_PB_CODEC {
                return Ok(false);
            }
            let block = self.get_block(&cid).await?;
            let Some(branch) = parse_file_branch(block.data()) else {
                return Ok(false);
            };
            let mut child_start = node_start;
            for link in &branch.links {
                if child_start >= boundary {
                    break;
                }
                let child_end = child_start + link.blocksize;
                if link.cid.codec() == RAW_LEAF_CODEC {
                    if child_end <= boundary && link.blocksize == CHUNK {
                        out.push(link.clone());
                    } else {
                        return Ok(false);
                    }
                } else if !self
                    .collect_prefix_leaves(link.cid, child_start, boundary, out)
                    .await?
                {
                    return Ok(false);
                }
                child_start = child_end;
            }
            Ok(true)
        }
        .boxed()
    }

    async fn prefix_leaves(
        &self,
        file_cid: Cid,
        filesize: u64,
        boundary: u64,
    ) -> Result<Option<Vec<FileBranchLink>>, Error> {
        if file_cid.codec() == RAW_LEAF_CODEC {
            let leaf = FileBranchLink {
                cid: file_cid,
                blocksize: filesize,
                tsize: filesize,
            };
            return Ok(Some(if filesize > 0 && filesize <= boundary {
                vec![leaf]
            } else {
                vec![]
            }));
        }
        let mut out = Vec::new();
        if self
            .collect_prefix_leaves(file_cid, 0, boundary, &mut out)
            .await?
        {
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    async fn rebuild_from_boundary(
        &self,
        file_cid: Cid,
        filesize: u64,
        boundary: u64,
        new_tail: Vec<u8>,
    ) -> Result<Option<(Cid, u64, Vec<Block>)>, Error> {
        let Some(mut leaves) = self.prefix_leaves(file_cid, filesize, boundary).await? else {
            return Ok(None);
        };

        let mut new_blocks = Vec::new();
        for chunk in new_tail.chunks(CHUNK as usize) {
            let cid = Cid::new_v1(RAW_LEAF_CODEC, HASHER.digest(chunk));
            leaves.push(FileBranchLink {
                cid,
                blocksize: chunk.len() as u64,
                tsize: chunk.len() as u64,
            });
            new_blocks.push(Block::new(cid, chunk.to_vec())?);
        }

        if leaves.is_empty() {
            let (cid, tsize, blocks) = encode_file(&[])?;
            return Ok(Some((cid, tsize, blocks)));
        }

        let (root_cid, root_tsize, branch_blocks) =
            build_file_from_leaves(&leaves, VERSION, HASHER);
        for (cid, bytes) in branch_blocks {
            new_blocks.push(Block::new(cid, bytes)?);
        }
        Ok(Some((root_cid, root_tsize, new_blocks)))
    }

    async fn grow_file(
        &self,
        file_cid: Cid,
        filesize: u64,
        offset: u64,
        data: &[u8],
        _gc_guard: &GCGuard,
    ) -> Result<Option<(Cid, u64, Vec<Block>)>, Error> {
        let boundary = (offset.min(filesize) / CHUNK) * CHUNK;
        let mut new_tail = if boundary < filesize {
            self.read_file_range(&file_cid, boundary, filesize).await?
        } else {
            Vec::new()
        };
        let rel_off = (offset - boundary) as usize;
        let rel_end = rel_off + data.len();
        if new_tail.len() < rel_end {
            new_tail.resize(rel_end, 0);
        }
        new_tail[rel_off..rel_end].copy_from_slice(data);
        self.rebuild_from_boundary(file_cid, filesize, boundary, new_tail)
            .await
    }

    async fn read_existing_file_locked(
        &self,
        guard: &mut (bool, Option<Cid>),
        comps: &[String],
        _gc_guard: &GCGuard,
    ) -> Result<Option<Vec<u8>>, Error> {
        let Some(root) = self.cached_root(guard).await? else {
            return Ok(None);
        };
        let (cid, tsize) = match self.resolve_from(root, comps).await {
            Ok(resolved) => resolved,
            Err(_) => return Ok(None),
        };
        match self.classify(&cid, tsize).await? {
            MfsKind::Directory => {
                return Err(MfsError::IsDirectory(format!("/{}", comps.join("/"))).into());
            }
            MfsKind::Symlink => {
                return Err(MfsError::IsSymlink(format!("/{}", comps.join("/"))).into());
            }
            MfsKind::File { .. } => {}
        }
        Ok(Some(self.read_file(&cid).await?))
    }

    /// Reads a UnixFS file DAG (or raw leaf) rooted at `cid` into a byte vector, fetching blocks
    /// locally (the MFS tree is pinned and present).
    async fn read_file(&self, cid: &Cid) -> Result<Vec<u8>, Error> {
        let block = self.get_block(cid).await?;

        // a single raw leaf is the content itself
        if matches!(describe_block(&block), NodeDescription::Other) {
            return Ok(block.data().to_vec());
        }

        let mut out = Vec::new();
        let mut cache = None;
        let (content, _, _, mut step) = IdleFileVisit::default().start(block.data())?;
        out.extend_from_slice(content);

        while let Some(visit) = step {
            let next = *visit.pending_links().0;
            let block = self.get_block(&next).await?;
            let (content, next_step) = visit.continue_walk(block.data(), &mut cache)?;
            out.extend_from_slice(content);
            step = next_step;
        }

        Ok(out)
    }

    async fn get_block(&self, cid: &Cid) -> Result<Block, Error> {
        self.repo()
            .get_block_now(cid)
            .await?
            .ok_or_else(|| anyhow!("missing block {cid}"))
    }
}

fn describe_block(block: &Block) -> NodeDescription {
    match block.cid().codec() {
        DAG_PB_CODEC => describe(block.data()),
        _ => NodeDescription::Other,
    }
}

/// Splits an MFS path into its components, rejecting `.`/`..` and treating `/` as the root (empty).
fn split_path(path: &str) -> Result<Vec<String>, Error> {
    let mut comps = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" => continue,
            "." | ".." => return Err(anyhow!("'.' and '..' are not supported in MFS paths")),
            other => comps.push(other.to_string()),
        }
    }
    Ok(comps)
}

/// The logical size reported for a listing/stat entry: a file's own size, else the cumulative size.
fn entry_size(kind: MfsKind, cumulative: u64) -> u64 {
    match kind {
        MfsKind::File { size } => size,
        MfsKind::Directory | MfsKind::Symlink => cumulative,
    }
}

/// The last non-empty path segment, mirroring `path.Base` (works for MFS and `/ipfs` paths).
fn base_name(path: &str) -> Option<&str> {
    path.rsplit('/').find(|s| !s.is_empty())
}

fn is_not_found(err: &Error) -> bool {
    matches!(err.downcast_ref::<MfsError>(), Some(MfsError::NotFound(_)))
}

fn is_shard_prefix(name: &str) -> bool {
    name.len() == 2 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_ipfs_path(path: &str) -> bool {
    path.starts_with("/ipfs/") || path.starts_with("/ipld/")
}

fn links_to_map(links: Vec<DirLink>) -> DirMap {
    links
        .into_iter()
        .map(|l| {
            (
                l.name,
                DirEntry {
                    cid: l.target,
                    tsize: l.tsize,
                },
            )
        })
        .collect()
}

fn encode_dir(
    entries: &DirMap,
    shard_threshold: Option<u64>,
) -> Result<(Cid, u64, Vec<Block>), Error> {
    let mut opts = TreeOptions::default();
    opts.wrap_with_directory();
    opts.cid_version(VERSION);
    opts.hasher(HASHER);
    opts.shard_threshold(shard_threshold);

    let mut builder = BufferingTreeBuilder::new(opts);
    for (name, entry) in entries {
        builder.put_link(name, entry.cid, entry.tsize)?;
    }

    let mut blocks = Vec::new();
    let mut root = None;
    for node in builder.build() {
        let node = node?;
        root = Some((node.cid, node.total_size));
        blocks.push(Block::new(node.cid, node.block.into_vec())?);
    }

    let (cid, tsize) = root.ok_or_else(|| anyhow!("directory produced no node"))?;
    Ok((cid, tsize, blocks))
}

/// Encodes `data` as a UnixFS file DAG, returning its root Cid, cumulative dag size (the link
/// Tsize), and every produced block (root last).
fn encode_file(data: &[u8]) -> Result<(Cid, u64, Vec<Block>), Error> {
    let mut adder = FileAdder::builder()
        .with_cid_version(VERSION)
        .with_hasher(HASHER)
        .build();

    let mut blocks = Vec::new();
    let mut tsize = 0u64;
    let mut root = None;

    let mut push = |cid: Cid, block: Vec<u8>| -> Result<(), Error> {
        tsize += block.len() as u64;
        root = Some(cid);
        blocks.push(Block::new(cid, block)?);
        Ok(())
    };

    let mut offset = 0;
    while offset < data.len() {
        let (ready, consumed) = adder.push(&data[offset..]);
        for (cid, block) in ready {
            push(cid, block)?;
        }
        offset += consumed;
    }
    for (cid, block) in adder.finish() {
        push(cid, block)?;
    }

    let cid = root.ok_or_else(|| anyhow!("file produced no blocks"))?;
    Ok((cid, tsize, blocks))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mfs() -> Mfs {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        Mfs::new(repo)
    }

    #[tokio::test]
    async fn mkdir_ls_stat() {
        let mfs = mfs().await;

        mfs.mkdir("/a/b/c", true).await.unwrap();

        let top = mfs.ls("/").await.unwrap();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].name, "a");
        assert_eq!(top[0].kind, MfsKind::Directory);

        let inner = mfs.ls("/a/b").await.unwrap();
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].name, "c");

        let st = mfs.stat("/a/b/c").await.unwrap();
        assert_eq!(st.kind, MfsKind::Directory);
    }

    #[tokio::test]
    async fn empty_root_lists_empty() {
        let mfs = mfs().await;
        assert!(mfs.ls("/").await.unwrap().is_empty());
        assert_eq!(mfs.stat("/").await.unwrap().kind, MfsKind::Directory);
        assert!(
            mfs.root().await.unwrap().is_none(),
            "reads must not create a root"
        );
    }

    #[tokio::test]
    async fn write_read_roundtrip() {
        let mfs = mfs().await;

        mfs.write("/docs/hello.txt", b"hello mfs", true)
            .await
            .unwrap();
        assert_eq!(mfs.read("/docs/hello.txt").await.unwrap(), b"hello mfs");

        let st = mfs.stat("/docs/hello.txt").await.unwrap();
        assert_eq!(st.kind, MfsKind::File { size: 9 });

        // overwrite
        mfs.write("/docs/hello.txt", b"changed", false)
            .await
            .unwrap();
        assert_eq!(mfs.read("/docs/hello.txt").await.unwrap(), b"changed");

        // a directory listing reflects the file
        let entries = mfs.ls("/docs").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello.txt");
        assert!(matches!(entries[0].kind, MfsKind::File { .. }));
    }

    #[tokio::test]
    async fn write_at_offset_and_truncate() {
        let mfs = mfs().await;
        mfs.write("/f", b"hello world", true).await.unwrap();

        mfs.write_with(
            "/f",
            b"MFS",
            WriteOptions {
                offset: 6,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(mfs.read("/f").await.unwrap(), b"hello MFSld");

        mfs.write_with(
            "/f",
            b"!!",
            WriteOptions {
                offset: 13,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(mfs.read("/f").await.unwrap(), b"hello MFSld\0\0!!");

        mfs.write_with(
            "/f",
            b"fresh",
            WriteOptions {
                truncate: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(mfs.read("/f").await.unwrap(), b"fresh");

        mfs.truncate("/f", 3).await.unwrap();
        assert_eq!(mfs.read("/f").await.unwrap(), b"fre");
        mfs.truncate("/f", 5).await.unwrap();
        assert_eq!(mfs.read("/f").await.unwrap(), b"fre\0\0");

        assert!(
            mfs.write_with("/missing", b"x", WriteOptions::default())
                .await
                .is_err()
        );
        mfs.write_with(
            "/created",
            b"y",
            WriteOptions {
                create: true,
                parents: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(mfs.read("/created").await.unwrap(), b"y");
    }

    #[tokio::test]
    async fn offset_edit_multiblock() {
        let mfs = mfs().await;
        let mut data: Vec<u8> = (0..1_000_000u32).map(|i| i as u8).collect();
        mfs.write("/big", &data, false).await.unwrap();

        let patch = b"PATCHED";
        let off = 500_000usize;
        mfs.write_with(
            "/big",
            patch,
            WriteOptions {
                offset: off as u64,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        data[off..off + patch.len()].copy_from_slice(patch);
        assert_eq!(mfs.read("/big").await.unwrap(), data);
    }

    #[tokio::test]
    async fn overwrite_in_place_is_canonical_and_reuses_blocks() {
        use futures::StreamExt as _;

        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo.clone());

        let mut content = vec![0u8; 4_000_000];
        let mut x = 0x1234_5678u32;
        for b in content.iter_mut() {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = (x >> 24) as u8;
        }
        mfs.write("/big", &content, false).await.unwrap();

        let before = repo.list_blocks().await.collect::<Vec<_>>().await.len();
        assert!(before > 8, "file should be many blocks, got {before}");

        let patch = b"PATCHED-IN-PLACE";
        let off = 1_500_000u64;
        mfs.write_with(
            "/big",
            patch,
            WriteOptions {
                offset: off,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        content[off as usize..off as usize + patch.len()].copy_from_slice(patch);
        let (expected_cid, _, _) = encode_file(&content).unwrap();
        assert_eq!(
            mfs.stat("/big").await.unwrap().cid,
            expected_cid,
            "editor must produce the canonical tree"
        );
        assert_eq!(mfs.read("/big").await.unwrap(), content);

        let after = repo.list_blocks().await.collect::<Vec<_>>().await.len();
        assert!(
            after - before < 8,
            "expected few new blocks (reuse), added {}",
            after - before
        );

        let patch2 = b"CROSS-CHUNK-BOUNDARY";
        let off2 = 262_144usize - 5;
        mfs.write_with(
            "/big",
            patch2,
            WriteOptions {
                offset: off2 as u64,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        content[off2..off2 + patch2.len()].copy_from_slice(patch2);
        let (expected2, _, _) = encode_file(&content).unwrap();
        assert_eq!(mfs.stat("/big").await.unwrap().cid, expected2);
        assert_eq!(mfs.read("/big").await.unwrap(), content);
    }

    fn fill(n: usize, seed: u32) -> Vec<u8> {
        let mut v = vec![0u8; n];
        let mut x = seed;
        for b in v.iter_mut() {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = (x >> 24) as u8;
        }
        v
    }

    async fn assert_canonical(mfs: &Mfs, path: &str, expected: &[u8]) {
        let (cid, _, _) = encode_file(expected).unwrap();
        assert_eq!(
            mfs.stat(path).await.unwrap().cid,
            cid,
            "non-canonical tree for {path}"
        );
        assert_eq!(mfs.read(path).await.unwrap(), expected);
    }

    #[tokio::test]
    async fn grow_append_truncate_are_canonical() {
        use futures::StreamExt as _;
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo.clone());

        let mut content = fill(800_000, 1);
        mfs.write("/f", &content, true).await.unwrap();
        let before = repo.list_blocks().await.collect::<Vec<_>>().await.len();

        let app = fill(300_000, 2);
        mfs.write_with(
            "/f",
            &app,
            WriteOptions {
                offset: content.len() as u64,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        content.extend_from_slice(&app);
        assert_canonical(&mfs, "/f", &content).await;
        let after = repo.list_blocks().await.collect::<Vec<_>>().await.len();
        assert!(
            after - before < 8,
            "append should reuse, added {}",
            after - before
        );

        let patch = fill(200_000, 3);
        let off = content.len() - 50_000;
        mfs.write_with(
            "/f",
            &patch,
            WriteOptions {
                offset: off as u64,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        content.resize((off + patch.len()).max(content.len()), 0);
        content[off..off + patch.len()].copy_from_slice(&patch);
        assert_canonical(&mfs, "/f", &content).await;

        mfs.truncate("/f", 600_000).await.unwrap();
        content.truncate(600_000);
        assert_canonical(&mfs, "/f", &content).await;

        mfs.truncate("/f", 900_000).await.unwrap();
        content.resize(900_000, 0);
        assert_canonical(&mfs, "/f", &content).await;

        mfs.truncate("/f", 0).await.unwrap();
        assert_canonical(&mfs, "/f", &[]).await;
    }

    #[tokio::test]
    async fn small_file_grows_to_multiblock() {
        let mfs = mfs().await;
        mfs.write("/s", b"small", true).await.unwrap();

        let app = fill(600_000, 9);
        mfs.write_with(
            "/s",
            &app,
            WriteOptions {
                offset: 5,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let mut content = b"small".to_vec();
        content.resize(5 + app.len(), 0);
        content[5..5 + app.len()].copy_from_slice(&app);
        assert_canonical(&mfs, "/s", &content).await;
    }

    #[tokio::test]
    async fn write_stream_is_canonical() {
        let mfs = mfs().await;

        let content = fill(700_000, 11);
        let chunks: Vec<bytes::Bytes> = content
            .chunks(33_333)
            .map(bytes::Bytes::copy_from_slice)
            .collect();
        let stream = futures::stream::iter(chunks.into_iter().map(Ok::<_, std::io::Error>));
        mfs.write_stream("/streamed", stream, true).await.unwrap();
        assert_canonical(&mfs, "/streamed", &content).await;

        let empty = futures::stream::iter(Vec::<std::io::Result<bytes::Bytes>>::new());
        mfs.write_stream("/empty", empty, false).await.unwrap();
        assert_canonical(&mfs, "/empty", &[]).await;
    }

    #[tokio::test]
    async fn write_stream_reports_progress() {
        use futures::StreamExt as _;
        let mfs = mfs().await;

        let content = fill(700_000, 12);
        let chunks: Vec<bytes::Bytes> = content
            .chunks(50_000)
            .map(bytes::Bytes::copy_from_slice)
            .collect();
        let stream = futures::stream::iter(chunks.into_iter().map(Ok::<_, std::io::Error>));

        let mut write = mfs.write_stream("/big", stream, true);
        let mut last = 0u64;
        let mut completed = None;
        while let Some(status) = write.next().await {
            match status {
                WriteStatus::Progress { written, total } => {
                    assert!(written >= last);
                    assert_eq!(total, None);
                    last = written;
                }
                WriteStatus::Completed { cid } => completed = Some(cid),
                WriteStatus::Failed { error } => panic!("write failed: {error}"),
            }
        }
        let (expected, _, _) = encode_file(&content).unwrap();
        assert_eq!(completed, Some(expected));
        assert_eq!(last, content.len() as u64);
        assert_canonical(&mfs, "/big", &content).await;
    }

    #[tokio::test]
    async fn write_from_file_roundtrips_with_total() {
        use futures::StreamExt as _;
        let mfs = mfs().await;

        let content = fill(400_000, 13);
        let file = std::env::temp_dir().join("rust_ipfs_mfs_write_from_file_test.bin");
        std::fs::write(&file, &content).unwrap();

        let cid = mfs.write_from_file("/imported", &file, true).await.unwrap();
        let (expected, _, _) = encode_file(&content).unwrap();
        assert_eq!(cid, expected);
        assert_canonical(&mfs, "/imported", &content).await;

        let mut write = mfs.write_from_file("/imported2", &file, true);
        let mut saw_total = false;
        while let Some(status) = write.next().await {
            if let WriteStatus::Progress { total, .. } = status {
                assert_eq!(total, Some(content.len() as u64));
                saw_total = true;
            }
        }
        assert!(saw_total);

        assert!(
            mfs.write_from_file("/x", file.join("missing"), false)
                .await
                .is_err()
        );

        std::fs::remove_file(&file).ok();
    }

    #[tokio::test]
    async fn read_stream_roundtrips() {
        use futures::StreamExt as _;
        let mfs = mfs().await;
        let content = fill(600_000, 21);
        mfs.write("/f", &content, true).await.unwrap();

        let stream = mfs.read_stream("/f");
        futures::pin_mut!(stream);
        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            out.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(out, content);

        mfs.mkdir("/d", false).await.unwrap();
        let dir_stream = mfs.read_stream("/d");
        futures::pin_mut!(dir_stream);
        assert!(dir_stream.next().await.unwrap().is_err());
    }

    #[tokio::test]
    async fn read_to_file_roundtrips_with_progress() {
        use futures::StreamExt as _;
        let mfs = mfs().await;
        let content = fill(500_000, 22);
        mfs.write("/f", &content, true).await.unwrap();

        let dest = std::env::temp_dir().join("rust_ipfs_mfs_read_to_file_test.bin");
        std::fs::remove_file(&dest).ok();

        let written = mfs.read_to_file("/f", &dest).await.unwrap();
        assert_eq!(written, content.len() as u64);
        assert_eq!(std::fs::read(&dest).unwrap(), content);

        let mut read = mfs.read_to_file("/f", &dest);
        let mut last = 0u64;
        while let Some(status) = read.next().await {
            match status {
                ReadStatus::Progress { written, total } => {
                    assert!(written >= last);
                    assert_eq!(total, content.len() as u64);
                    last = written;
                }
                ReadStatus::Completed { written } => assert_eq!(written, content.len() as u64),
                ReadStatus::Failed { error } => panic!("read failed: {error}"),
            }
        }
        assert_eq!(last, content.len() as u64);

        std::fs::remove_file(&dest).ok();
    }

    #[tokio::test]
    async fn empty_file_grow_is_canonical() {
        let mfs = mfs().await;

        mfs.write("/e", b"", true).await.unwrap();
        let data = fill(CHUNK as usize + 5, 4);
        mfs.write_with(
            "/e",
            &data,
            WriteOptions {
                offset: 0,
                create: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_canonical(&mfs, "/e", &data).await;

        mfs.write("/t", &fill(500_000, 5), true).await.unwrap();
        mfs.truncate("/t", 0).await.unwrap();
        let grow = fill(CHUNK as usize + 5, 6);
        mfs.write_with(
            "/t",
            &grow,
            WriteOptions {
                offset: 0,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_canonical(&mfs, "/t", &grow).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_writes_to_same_file_dont_lose_updates() {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo);
        mfs.write("/f", &vec![0u8; 2000], true).await.unwrap();

        let a = mfs.clone();
        let b = mfs.clone();
        let ha = tokio::spawn(async move {
            a.write_with(
                "/f",
                &[1, 1, 1, 1],
                WriteOptions {
                    offset: 0,
                    ..Default::default()
                },
            )
            .await
        });
        let hb = tokio::spawn(async move {
            b.write_with(
                "/f",
                &[2, 2, 2, 2],
                WriteOptions {
                    offset: 1000,
                    ..Default::default()
                },
            )
            .await
        });
        ha.await.unwrap().unwrap();
        hb.await.unwrap().unwrap();

        let content = mfs.read("/f").await.unwrap();
        assert_eq!(&content[0..4], &[1, 1, 1, 1], "write A was lost");
        assert_eq!(&content[1000..1004], &[2, 2, 2, 2], "write B was lost");
        assert_eq!(content.len(), 2000);
    }

    #[tokio::test]
    async fn imported_nonstandard_chunk_file_falls_back() {
        use rust_unixfs::file::adder::{Chunker, FileAdder};
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo.clone());

        // a file chunked at 400KiB (leaves larger than our 256KiB CHUNK), imported by reference
        let content = fill(900_000, 7);
        let mut adder = FileAdder::builder()
            .with_chunker(Chunker::Size(400 * 1024))
            .with_cid_version(VERSION)
            .with_hasher(HASHER)
            .build();
        let mut blocks = Vec::new();
        let mut root = None;
        let mut off = 0;
        while off < content.len() {
            let (ready, consumed) = adder.push(&content[off..]);
            for (cid, block) in ready {
                root = Some(cid);
                blocks.push(Block::new(cid, block).unwrap());
            }
            off += consumed;
        }
        for (cid, block) in adder.finish() {
            root = Some(cid);
            blocks.push(Block::new(cid, block).unwrap());
        }
        let root = root.unwrap();
        repo.put_blocks(blocks).await.unwrap();

        mfs.cp(&format!("/ipfs/{root}"), "/imported", true)
            .await
            .unwrap();

        // grow it: the 400KiB leaves straddle our 256KiB boundary, so the editor must bail to
        // read-modify-rewrite, which re-encodes canonically (correct content, our chunking)
        let app = fill(100_000, 8);
        let mut expected = content.clone();
        expected.extend_from_slice(&app);
        mfs.write_with(
            "/imported",
            &app,
            WriteOptions {
                offset: content.len() as u64,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_canonical(&mfs, "/imported", &expected).await;
    }

    #[tokio::test]
    async fn write_read_multiblock() {
        let mfs = mfs().await;
        // larger than the default chunk size so the file becomes a multi-block DAG
        let data: Vec<u8> = (0..1_000_000u32).map(|i| i as u8).collect();

        mfs.write("/big.bin", &data, false).await.unwrap();
        assert_eq!(mfs.read("/big.bin").await.unwrap(), data);

        match mfs.stat("/big.bin").await.unwrap().kind {
            MfsKind::File { size } => assert_eq!(size, data.len() as u64),
            other => panic!("expected file, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn large_dir_shards_and_roundtrips() {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo).with_shard_threshold(Some(0));

        let n = 64usize;
        for i in 0..n {
            mfs.write(
                &format!("/d/file{i:03}"),
                format!("content {i}").as_bytes(),
                true,
            )
            .await
            .unwrap();
        }

        let cid = mfs.stat("/d").await.unwrap().cid;
        let block = mfs.get_block(&cid).await.unwrap();
        assert!(
            matches!(describe(block.data()), NodeDescription::HamtShard { .. }),
            "directory should be HAMT-sharded"
        );

        let entries = mfs.ls("/d").await.unwrap();
        assert_eq!(entries.len(), n);

        assert_eq!(mfs.read("/d/file007").await.unwrap(), b"content 7");
        assert_eq!(mfs.read("/d/file063").await.unwrap(), b"content 63");

        mfs.write("/d/file064", b"new", false).await.unwrap();
        assert_eq!(mfs.ls("/d").await.unwrap().len(), n + 1);

        mfs.rm("/d/file007", false).await.unwrap();
        assert_eq!(mfs.ls("/d").await.unwrap().len(), n);
        assert!(mfs.read("/d/file007").await.is_err());
    }

    #[tokio::test]
    async fn cp_from_ipfs_through_hamt_dir() {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo).with_shard_threshold(Some(0));

        for i in 0..64u32 {
            mfs.write(
                &format!("/d/file{i:03}"),
                format!("content {i}").as_bytes(),
                true,
            )
            .await
            .unwrap();
        }

        let dir_cid = mfs.stat("/d").await.unwrap().cid;
        let block = mfs.get_block(&dir_cid).await.unwrap();
        assert!(
            matches!(describe(block.data()), NodeDescription::HamtShard { .. }),
            "source dir should be HAMT-sharded"
        );

        mfs.cp(&format!("/ipfs/{dir_cid}/file042"), "/copied", false)
            .await
            .unwrap();
        assert_eq!(mfs.read("/copied").await.unwrap(), b"content 42");

        assert!(
            mfs.cp(&format!("/ipfs/{dir_cid}/missing"), "/x", false)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn cp_from_ipfs() {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo.clone());

        let (file_cid, _, blocks) = encode_file(b"imported content").unwrap();
        repo.put_blocks(blocks).await.unwrap();

        mfs.cp(&format!("/ipfs/{file_cid}"), "/imported.txt", true)
            .await
            .unwrap();
        assert_eq!(
            mfs.read("/imported.txt").await.unwrap(),
            b"imported content"
        );
        assert!(matches!(
            mfs.stat("/imported.txt").await.unwrap().kind,
            MfsKind::File { .. }
        ));

        let (greeting_cid, greeting_tsize, gblocks) = encode_file(b"hi").unwrap();
        repo.put_blocks(gblocks).await.unwrap();
        let mut dirmap = DirMap::new();
        dirmap.insert(
            "greeting".into(),
            DirEntry {
                cid: greeting_cid,
                tsize: greeting_tsize,
            },
        );
        let (dir_cid, _, dblocks) = encode_dir(&dirmap, None).unwrap();
        repo.put_blocks(dblocks).await.unwrap();

        mfs.cp(&format!("/ipfs/{dir_cid}"), "/srcdir", false)
            .await
            .unwrap();
        assert_eq!(mfs.ls("/srcdir").await.unwrap().len(), 1);

        mfs.cp(&format!("/ipfs/{dir_cid}/greeting"), "/hi.txt", false)
            .await
            .unwrap();
        assert_eq!(mfs.read("/hi.txt").await.unwrap(), b"hi");
    }

    #[tokio::test]
    async fn rm_cp_mv() {
        let mfs = mfs().await;
        mfs.write("/a/file", b"data", true).await.unwrap();

        // cp duplicates
        mfs.cp("/a/file", "/a/copy", false).await.unwrap();
        assert_eq!(mfs.read("/a/copy").await.unwrap(), b"data");
        assert_eq!(mfs.read("/a/file").await.unwrap(), b"data");

        // mv relocates (source gone, dest present)
        mfs.mkdir("/b", false).await.unwrap();
        mfs.mv("/a/copy", "/b/moved", false).await.unwrap();
        assert_eq!(mfs.read("/b/moved").await.unwrap(), b"data");
        assert!(mfs.read("/a/copy").await.is_err());

        // rm removes
        mfs.rm("/a/file", false).await.unwrap();
        assert!(mfs.read("/a/file").await.is_err());

        // non-empty dir requires recursive
        assert!(mfs.rm("/b", false).await.is_err());
        mfs.rm("/b", true).await.unwrap();
        assert!(mfs.ls("/b").await.is_err());
    }

    #[tokio::test]
    async fn mkdir_without_parents_errors_on_missing_parent() {
        let mfs = mfs().await;
        assert!(mfs.mkdir("/x/y", false).await.is_err());
        mfs.mkdir("/x", false).await.unwrap();
        mfs.mkdir("/x/y", false).await.unwrap();
        assert!(
            mfs.mkdir("/x/y", false).await.is_err(),
            "duplicate mkdir must fail"
        );
    }

    #[tokio::test]
    async fn mfs_tree_survives_gc() {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo.clone());

        mfs.write("/a/keep.txt", b"survive gc", true).await.unwrap();

        // GC removes everything not pinned; the recursively-pinned MFS root must keep the tree alive
        repo.cleanup().await.unwrap();

        assert_eq!(mfs.read("/a/keep.txt").await.unwrap(), b"survive gc");
        assert_eq!(mfs.ls("/a").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn root_persists_across_handles() {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();

        let root_cid = {
            let mfs = Mfs::new(repo.clone());
            mfs.mkdir("/keep", true).await.unwrap();
            mfs.root().await.unwrap().unwrap()
        };

        // a fresh handle on the same repo reads the persisted root and sees the directory
        let reopened = Mfs::new(repo);
        assert_eq!(reopened.root().await.unwrap(), Some(root_cid));
        let entries = reopened.ls("/").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "keep");
    }

    #[tokio::test]
    async fn read_range_partial() {
        let mfs = mfs().await;
        let content = fill(600_000, 41);
        mfs.write("/f", &content, true).await.unwrap();

        assert_eq!(
            mfs.read_range("/f", 100, Some(50)).await.unwrap(),
            content[100..150]
        );
        assert_eq!(
            mfs.read_range("/f", 261_000, Some(2000)).await.unwrap(),
            content[261_000..263_000],
            "range crossing a chunk boundary"
        );
        assert_eq!(
            mfs.read_range("/f", 590_000, None).await.unwrap(),
            content[590_000..]
        );
        assert_eq!(mfs.read_range("/f", 0, None).await.unwrap(), content);
        assert_eq!(
            mfs.read_range("/f", 599_990, Some(1000)).await.unwrap(),
            content[599_990..],
            "count past EOF clamps"
        );
        assert!(
            mfs.read_range("/f", 600_000, Some(10))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            mfs.read_range("/f", 10_000_000, Some(5))
                .await
                .unwrap()
                .is_empty()
        );

        mfs.write("/s", b"hello", true).await.unwrap();
        assert_eq!(mfs.read_range("/s", 1, Some(3)).await.unwrap(), b"ell");
        assert_eq!(mfs.read_range("/s", 2, None).await.unwrap(), b"llo");
        assert!(mfs.read_range("/s", 9, Some(1)).await.unwrap().is_empty());

        let stream = mfs.read_stream_range("/f", 1000, Some(500));
        futures::pin_mut!(stream);
        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            out.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(out, content[1000..1500]);
    }

    #[tokio::test]
    async fn cp_mv_into_existing_directory() {
        let mfs = mfs().await;
        mfs.write("/src/file.txt", b"hi", true).await.unwrap();

        mfs.mkdir("/dst", false).await.unwrap();
        mfs.cp("/src/file.txt", "/dst", false).await.unwrap();
        assert_eq!(mfs.read("/dst/file.txt").await.unwrap(), b"hi");

        mfs.mkdir("/d2", false).await.unwrap();
        mfs.cp("/src/file.txt", "/d2/", false).await.unwrap();
        assert_eq!(
            mfs.read("/d2/file.txt").await.unwrap(),
            b"hi",
            "trailing slash targets the dir"
        );

        mfs.mkdir("/dst3", false).await.unwrap();
        mfs.mv("/src/file.txt", "/dst3", false).await.unwrap();
        assert_eq!(mfs.read("/dst3/file.txt").await.unwrap(), b"hi");
        assert!(!mfs.exists("/src/file.txt").await.unwrap());
    }

    #[tokio::test]
    async fn cp_onto_existing_errors_mv_replaces() {
        let mfs = mfs().await;
        mfs.write("/a.txt", b"aaa", true).await.unwrap();
        mfs.write("/b.txt", b"bbb", true).await.unwrap();

        let err = mfs.cp("/a.txt", "/b.txt", false).await.unwrap_err();
        assert!(matches!(
            err.downcast_ref::<MfsError>(),
            Some(MfsError::AlreadyExists(_))
        ));
        assert_eq!(mfs.read("/b.txt").await.unwrap(), b"bbb");

        mfs.mv("/a.txt", "/b.txt", false).await.unwrap();
        assert_eq!(mfs.read("/b.txt").await.unwrap(), b"aaa");
        assert!(!mfs.exists("/a.txt").await.unwrap());
    }

    #[tokio::test]
    async fn mv_self_and_descendant_are_guarded() {
        let mfs = mfs().await;
        mfs.write("/d/f.txt", b"data", true).await.unwrap();

        mfs.mv("/d/f.txt", "/d/f.txt", false).await.unwrap();
        assert_eq!(
            mfs.read("/d/f.txt").await.unwrap(),
            b"data",
            "self-move must not destroy"
        );

        mfs.mv("/d/f.txt", "/d", false).await.unwrap();
        assert_eq!(
            mfs.read("/d/f.txt").await.unwrap(),
            b"data",
            "move into own parent is a no-op"
        );

        mfs.mkdir("/d/sub", true).await.unwrap();
        assert!(
            mfs.mv("/d", "/d/sub/inner", false).await.is_err(),
            "into own descendant"
        );
        assert_eq!(
            mfs.read("/d/f.txt").await.unwrap(),
            b"data",
            "subtree preserved"
        );
    }

    #[tokio::test]
    async fn rm_force_is_idempotent() {
        let mfs = mfs().await;
        mfs.write("/x.txt", b"x", true).await.unwrap();

        mfs.rm_force("/x.txt", false).await.unwrap();
        assert!(!mfs.exists("/x.txt").await.unwrap());
        mfs.rm_force("/x.txt", false).await.unwrap();
        mfs.rm_force("/never/existed", false).await.unwrap();
        assert!(
            mfs.rm("/x.txt", false).await.is_err(),
            "plain rm still errors on missing"
        );

        mfs.write("/dir/child", b"c", true).await.unwrap();
        assert!(
            mfs.rm_force("/dir", false).await.is_err(),
            "non-empty still needs recursive"
        );
        mfs.rm_force("/dir", true).await.unwrap();
        assert!(!mfs.exists("/dir").await.unwrap());
    }

    #[tokio::test]
    async fn ls_on_file_lists_itself() {
        let mfs = mfs().await;
        mfs.write("/docs/f.txt", b"hello", true).await.unwrap();

        let entries = mfs.ls("/docs/f.txt").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "f.txt");
        assert_eq!(entries[0].kind, MfsKind::File { size: 5 });
        assert_eq!(entries[0].size, 5);
    }

    #[tokio::test]
    async fn exists_reports_presence() {
        let mfs = mfs().await;
        mfs.mkdir("/a/b", true).await.unwrap();
        mfs.write("/a/f.txt", b"x", true).await.unwrap();

        assert!(mfs.exists("/").await.unwrap());
        assert!(mfs.exists("/a").await.unwrap());
        assert!(mfs.exists("/a/b").await.unwrap());
        assert!(mfs.exists("/a/f.txt").await.unwrap());
        assert!(!mfs.exists("/a/nope").await.unwrap());
        assert!(!mfs.exists("/nope/deep").await.unwrap());
    }

    #[tokio::test]
    async fn stat_reports_cumulative_and_blocks() {
        let mfs = mfs().await;
        let content = fill(600_000, 51);
        mfs.write("/big.bin", &content, true).await.unwrap();

        let root = mfs.stat("/").await.unwrap();
        assert_eq!(root.kind, MfsKind::Directory);
        assert!(root.size > 0, "root size must reflect its contents, not 0");
        assert_eq!(root.size, root.cumulative_size);

        let st = mfs.stat("/big.bin").await.unwrap();
        assert_eq!(st.kind, MfsKind::File { size: 600_000 });
        assert!(st.cumulative_size > st.size, "dag carries link overhead");
        assert!(
            st.blocks > 1,
            "a multiblock file has multiple leaf links, got {}",
            st.blocks
        );

        mfs.write("/s", b"hi", true).await.unwrap();
        assert_eq!(
            mfs.stat("/s").await.unwrap().blocks,
            0,
            "a raw leaf has no links"
        );
    }

    #[tokio::test]
    async fn write_stream_error_mid_stream_leaves_path_untouched() {
        use futures::StreamExt as _;
        let mfs = mfs().await;

        let chunks: Vec<std::io::Result<bytes::Bytes>> = vec![
            Ok(bytes::Bytes::from_static(b"good")),
            Err(std::io::Error::other("boom")),
        ];
        let mut write = mfs.write_stream("/partial", futures::stream::iter(chunks), false);
        let mut failed = 0;
        let mut completed = false;
        while let Some(status) = write.next().await {
            match status {
                WriteStatus::Failed { .. } => failed += 1,
                WriteStatus::Completed { .. } => completed = true,
                WriteStatus::Progress { .. } => {}
            }
        }
        assert_eq!(failed, 1);
        assert!(!completed);
        assert!(
            !mfs.exists("/partial").await.unwrap(),
            "failed write must not create the path"
        );

        let chunks2: Vec<std::io::Result<bytes::Bytes>> = vec![
            Ok(bytes::Bytes::from_static(b"x")),
            Err(std::io::Error::other("boom")),
        ];
        assert!(
            mfs.write_stream("/partial2", futures::stream::iter(chunks2), false)
                .await
                .is_err()
        );
        assert!(!mfs.exists("/partial2").await.unwrap());
    }

    fn encode_file_bf(data: &[u8], bf: usize) -> (Cid, Vec<Block>) {
        use rust_unixfs::file::adder::BalancedCollector;
        let mut adder = FileAdder::builder()
            .with_cid_version(VERSION)
            .with_hasher(HASHER)
            .with_collector(BalancedCollector::with_branching_factor(bf))
            .build();
        let mut blocks = Vec::new();
        let mut root = None;
        let mut off = 0;
        while off < data.len() {
            let (ready, consumed) = adder.push(&data[off..]);
            for (cid, block) in ready {
                root = Some(cid);
                blocks.push(Block::new(cid, block).unwrap());
            }
            off += consumed;
        }
        for (cid, block) in adder.finish() {
            root = Some(cid);
            blocks.push(Block::new(cid, block).unwrap());
        }
        (root.unwrap(), blocks)
    }

    #[tokio::test]
    async fn deep_tree_editor_overwrites_and_grows() {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let mfs = Mfs::new(repo.clone());

        let bf = 4;
        let content = fill(20 * CHUNK as usize, 31);
        let (root, blocks) = encode_file_bf(&content, bf);
        repo.put_blocks(blocks).await.unwrap();
        mfs.cp(&format!("/ipfs/{root}"), "/deep", true)
            .await
            .unwrap();

        let block = mfs.get_block(&root).await.unwrap();
        let branch = parse_file_branch(block.data()).expect("root is a file branch");
        let child = mfs.get_block(&branch.links[0].cid).await.unwrap();
        assert!(
            parse_file_branch(child.data()).is_some(),
            "tree must be deeper than two levels to exercise the recursive editor"
        );

        let patch = b"DEEP-OVERWRITE-PATCH";
        let off = 7 * CHUNK as usize + 100;
        mfs.write_with(
            "/deep",
            patch,
            WriteOptions {
                offset: off as u64,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let mut expected = content.clone();
        expected[off..off + patch.len()].copy_from_slice(patch);
        let (exp_root, _) = encode_file_bf(&expected, bf);
        assert_eq!(
            mfs.stat("/deep").await.unwrap().cid,
            exp_root,
            "overwrite preserves the deep structure canonically"
        );
        assert_eq!(mfs.read("/deep").await.unwrap(), expected);

        let app = fill(50_000, 32);
        mfs.write_with(
            "/deep",
            &app,
            WriteOptions {
                offset: expected.len() as u64,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        expected.extend_from_slice(&app);
        assert_canonical(&mfs, "/deep", &expected).await;
    }

    #[tokio::test]
    async fn mv_into_dir_does_not_clobber_colliding_entry() {
        let mfs = mfs().await;
        mfs.write("/a/x/keep", b"src", true).await.unwrap();
        mfs.write("/b/x/old", b"victim", true).await.unwrap();

        let err = mfs.mv("/a/x", "/b", false).await.unwrap_err();
        assert!(matches!(
            err.downcast_ref::<MfsError>(),
            Some(MfsError::AlreadyExists(_))
        ));
        assert_eq!(
            mfs.read("/b/x/old").await.unwrap(),
            b"victim",
            "destination subtree survives"
        );
        assert_eq!(
            mfs.read("/a/x/keep").await.unwrap(),
            b"src",
            "source preserved on failure"
        );
    }

    #[tokio::test]
    async fn mv_missing_source_errors() {
        let mfs = mfs().await;
        mfs.mkdir("/d", false).await.unwrap();
        assert!(
            mfs.mv("/d/missing", "/d", false).await.is_err(),
            "no false success via no-op guard"
        );
        assert!(mfs.mv("/nope", "/nope", false).await.is_err());
    }

    #[tokio::test]
    async fn rm_force_propagates_structural_errors() {
        let mfs = mfs().await;
        mfs.write("/a", b"file", true).await.unwrap();
        // /a is a file, so /a/b treats a file as an intermediate directory, a real error, not absence
        assert!(mfs.rm_force("/a/b", false).await.is_err());
        assert_eq!(mfs.read("/a").await.unwrap(), b"file");
    }

    #[tokio::test]
    async fn root_cumulative_size_is_monotonic() {
        let mfs = mfs().await;
        let big = fill(600_000, 61);
        mfs.write("/dir/a", &big, true).await.unwrap();
        mfs.cp("/dir/a", "/dir/b", false).await.unwrap();

        let root = mfs.stat("/").await.unwrap().cumulative_size;
        let dir = mfs.stat("/dir").await.unwrap().cumulative_size;
        assert!(
            root >= dir,
            "root cumulative {root} must be >= its child dir {dir}"
        );
    }
}
