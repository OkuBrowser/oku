//! Readonly in-memory store.
//!
//! This can only serve data that is provided at creation time. It is much simpler
//! than the mutable in-memory store and the file system store, and can serve as a
//! good starting point for custom implementations.
//!
//! It can also be useful as a lightweight store for tests.
use std::{
    collections::HashMap,
    io::{self, Write},
    ops::Deref,
    path::PathBuf,
};

use bao_tree::{
    io::{
        mixed::{traverse_ranges_validated, EncodedItem, ReadBytesAt},
        outboard::PreOrderMemOutboard,
        sync::ReadAt,
        Leaf,
    },
    BaoTree, ChunkRanges,
};
use bytes::Bytes;
use irpc::channel::mpsc;
use n0_future::{
    future::{self, yield_now},
    task::{JoinError, JoinSet},
};
use range_collections::range_set::RangeSetRange;
use ref_cast::RefCast;

use super::util::BaoTreeSender;
use crate::{
    api::{
        self,
        blobs::{Bitfield, ExportProgressItem},
        proto::{
            self, BlobStatus, Command, ExportBaoMsg, ExportBaoRequest, ExportPathMsg,
            ExportPathRequest, ExportRangesItem, ExportRangesMsg, ExportRangesRequest,
            ImportBaoMsg, ImportByteStreamMsg, ImportBytesMsg, ImportPathMsg, ObserveMsg,
            ObserveRequest, WaitIdleMsg,
        },
        ApiClient, TempTag,
    },
    protocol::ChunkRangesExt,
    store::{mem::CompleteStorage, IROH_BLOCK_SIZE},
    Hash,
};

#[derive(Debug, Clone)]
pub struct ReadonlyMemStore {
    client: ApiClient,
}

impl Deref for ReadonlyMemStore {
    type Target = crate::api::Store;

    fn deref(&self) -> &Self::Target {
        crate::api::Store::ref_from_sender(&self.client)
    }
}

impl From<ReadonlyMemStore> for crate::api::Store {
    fn from(value: ReadonlyMemStore) -> Self {
        crate::api::Store::from_sender(value.client)
    }
}

impl AsRef<crate::api::Store> for ReadonlyMemStore {
    fn as_ref(&self) -> &crate::api::Store {
        crate::api::Store::ref_from_sender(&self.client)
    }
}

struct Actor {
    commands: tokio::sync::mpsc::Receiver<proto::Command>,
    tasks: JoinSet<()>,
    idle_waiters: Vec<irpc::channel::oneshot::Sender<()>>,
    data: HashMap<Hash, CompleteStorage>,
}

impl Actor {
    fn new(
        commands: tokio::sync::mpsc::Receiver<proto::Command>,
        data: HashMap<Hash, CompleteStorage>,
    ) -> Self {
        Self {
            data,
            commands,
            tasks: JoinSet::new(),
            idle_waiters: Vec::new(),
        }
    }

    async fn handle_command(&mut self, cmd: Command) -> Option<irpc::channel::oneshot::Sender<()>> {
        match cmd {
            Command::ImportBao(ImportBaoMsg { tx, .. }) => {
                tx.send(Err(unsupported("import not supported").into()))
                    .await
                    .ok();
            }
            Command::WaitIdle(WaitIdleMsg { tx, .. }) => {
                if self.tasks.is_empty() {
                    // we are currently idle
                    tx.send(()).await.ok();
                } else {
                    // wait for idle state
                    self.idle_waiters.push(tx);
                }
            }
            Command::ImportBytes(ImportBytesMsg { tx, .. }) => {
                tx.send(unsupported("import not supported").into())
                    .await
                    .ok();
            }
            Command::ImportByteStream(ImportByteStreamMsg { tx, .. }) => {
                tx.send(unsupported("import not supported").into())
                    .await
                    .ok();
            }
            Command::ImportPath(ImportPathMsg { tx, .. }) => {
                tx.send(unsupported("import not supported").into())
                    .await
                    .ok();
            }
            Command::Observe(ObserveMsg {
                inner: ObserveRequest { hash },
                tx,
                ..
            }) => {
                let size = self.data.get_mut(&hash).map(|x| x.data.len() as u64);
                self.tasks.spawn(async move {
                    if let Some(size) = size {
                        tx.send(Bitfield::complete(size)).await.ok();
                    } else {
                        tx.send(Bitfield::empty()).await.ok();
                        future::pending::<()>().await;
                    };
                });
            }
            Command::ExportBao(ExportBaoMsg {
                inner: ExportBaoRequest { hash, ranges, .. },
                tx,
                ..
            }) => {
                let entry = self.data.get(&hash).cloned();
                self.tasks.spawn(export_bao(hash, entry, ranges, tx));
            }
            Command::ExportPath(ExportPathMsg {
                inner: ExportPathRequest { hash, target, .. },
                tx,
                ..
            }) => {
                let entry = self.data.get(&hash).cloned();
                self.tasks.spawn(export_path(entry, target, tx));
            }
            Command::Batch(_cmd) => {}
            Command::ClearProtected(cmd) => {
                cmd.tx.send(Ok(())).await.ok();
            }
            Command::CreateTag(cmd) => {
                cmd.tx
                    .send(Err(unsupported("create tag not supported").into()))
                    .await
                    .ok();
            }
            Command::CreateTempTag(cmd) => {
                cmd.tx.send(TempTag::new(cmd.inner.value, None)).await.ok();
            }
            Command::RenameTag(cmd) => {
                cmd.tx
                    .send(Err(unsupported("rename tag not supported").into()))
                    .await
                    .ok();
            }
            Command::DeleteTags(cmd) => {
                cmd.tx
                    .send(Err(unsupported("delete tags not supported").into()))
                    .await
                    .ok();
            }
            Command::DeleteBlobs(cmd) => {
                cmd.tx
                    .send(Err(unsupported("delete blobs not supported").into()))
                    .await
                    .ok();
            }
            Command::ListBlobs(cmd) => {
                let hashes: Vec<Hash> = self.data.keys().cloned().collect();
                self.tasks.spawn(async move {
                    for hash in hashes {
                        cmd.tx.send(Ok(hash)).await.ok();
                    }
                });
            }
            Command::BlobStatus(cmd) => {
                let hash = cmd.inner.hash;
                let entry = self.data.get(&hash);
                let status = if let Some(entry) = entry {
                    BlobStatus::Complete {
                        size: entry.data.len() as u64,
                    }
                } else {
                    BlobStatus::NotFound
                };
                cmd.tx.send(status).await.ok();
            }
            Command::ListTags(cmd) => {
                cmd.tx.send(Vec::new()).await.ok();
            }
            Command::SetTag(cmd) => {
                cmd.tx
                    .send(Err(unsupported("set tag not supported").into()))
                    .await
                    .ok();
            }
            Command::ListTempTags(cmd) => {
                cmd.tx.send(Vec::new()).await.ok();
            }
            Command::SyncDb(cmd) => {
                cmd.tx.send(Ok(())).await.ok();
            }
            Command::Shutdown(cmd) => {
                return Some(cmd.tx);
            }
            Command::ExportRanges(cmd) => {
                let entry = self.data.get(&cmd.inner.hash).cloned();
                self.tasks.spawn(export_ranges(cmd, entry));
            }
        }
        None
    }

    fn log_unit_task(&self, res: Result<(), JoinError>) {
        if let Err(e) = res {
            tracing::error!("task failed: {e}");
        }
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(cmd) = self.commands.recv() => {
                    if let Some(shutdown) = self.handle_command(cmd).await {
                        shutdown.send(()).await.ok();
                        break;
                    }
                },
                Some(res) = self.tasks.join_next(), if !self.tasks.is_empty() => {
                    self.log_unit_task(res);
                    if self.tasks.is_empty() {
                        // we are idle now
                        for tx in self.idle_waiters.drain(..) {
                            tx.send(()).await.ok();
                        }
                    }
                },
                else => break,
            }
        }
    }
}

fn unsupported(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, text)
}

async fn export_bao(
    hash: Hash,
    entry: Option<CompleteStorage>,
    ranges: ChunkRanges,
    mut sender: mpsc::Sender<EncodedItem>,
) {
    let entry = match entry {
        Some(entry) => entry,
        None => {
            sender
                .send(EncodedItem::Error(bao_tree::io::EncodeError::Io(
                    io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "export task ended unexpectedly",
                    ),
                )))
                .await
                .ok();
            return;
        }
    };
    let data = entry.data;
    let outboard = entry.outboard;
    let size = data.as_ref().len() as u64;
    let tree = BaoTree::new(size, IROH_BLOCK_SIZE);
    let outboard = PreOrderMemOutboard {
        root: hash.into(),
        tree,
        data: outboard,
    };
    let sender = BaoTreeSender::ref_cast_mut(&mut sender);
    traverse_ranges_validated(data.as_ref(), outboard, &ranges, sender)
        .await
        .ok();
}

async fn export_ranges(mut cmd: ExportRangesMsg, entry: Option<CompleteStorage>) {
    let Some(entry) = entry else {
        cmd.tx
            .send(ExportRangesItem::Error(api::Error::io(
                io::ErrorKind::NotFound,
                "hash not found",
            )))
            .await
            .ok();
        return;
    };
    if let Err(cause) = export_ranges_impl(cmd.inner, &mut cmd.tx, entry).await {
        cmd.tx
            .send(ExportRangesItem::Error(cause.into()))
            .await
            .ok();
    }
}

async fn export_ranges_impl(
    cmd: ExportRangesRequest,
    tx: &mut mpsc::Sender<ExportRangesItem>,
    entry: CompleteStorage,
) -> io::Result<()> {
    let ExportRangesRequest { ranges, .. } = cmd;
    let data = entry.data;
    let size = data.len() as u64;
    let bitfield = Bitfield::complete(size);
    for range in ranges.iter() {
        let range = match range {
            RangeSetRange::Range(range) => size.min(*range.start)..size.min(*range.end),
            RangeSetRange::RangeFrom(range) => size.min(*range.start)..size,
        };
        let requested = ChunkRanges::bytes(range.start..range.end);
        if !bitfield.ranges.is_superset(&requested) {
            return Err(io::Error::other(format!(
                "missing range: {requested:?}, present: {bitfield:?}",
            )));
        }
        let bs = 1024;
        let mut offset = range.start;
        loop {
            let end: u64 = (offset + bs).min(range.end);
            let size = (end - offset) as usize;
            tx.send(
                Leaf {
                    offset,
                    data: data.read_bytes_at(offset, size)?,
                }
                .into(),
            )
            .await?;
            offset = end;
            if offset >= range.end {
                break;
            }
        }
    }
    Ok(())
}

impl ReadonlyMemStore {
    pub fn new(items: impl IntoIterator<Item = impl AsRef<[u8]>>) -> Self {
        let mut entries = HashMap::new();
        for item in items {
            let data = Bytes::copy_from_slice(item.as_ref());
            let (hash, entry) = CompleteStorage::create(data);
            entries.insert(hash, entry);
        }
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let actor = Actor::new(receiver, entries);
        n0_future::task::spawn(actor.run());
        let local = irpc::LocalSender::from(sender);
        Self {
            client: local.into(),
        }
    }
}

async fn export_path(
    entry: Option<CompleteStorage>,
    target: PathBuf,
    mut tx: mpsc::Sender<ExportProgressItem>,
) {
    let Some(entry) = entry else {
        tx.send(api::Error::io(io::ErrorKind::NotFound, "hash not found").into())
            .await
            .ok();
        return;
    };
    match export_path_impl(entry, target, &mut tx).await {
        Ok(()) => tx.send(ExportProgressItem::Done).await.ok(),
        Err(cause) => tx.send(api::Error::from(cause).into()).await.ok(),
    };
}

async fn export_path_impl(
    entry: CompleteStorage,
    target: PathBuf,
    tx: &mut mpsc::Sender<ExportProgressItem>,
) -> io::Result<()> {
    let data = entry.data;
    // todo: for partial entries make sure to only write the part that is actually present
    let mut file = std::fs::File::create(&target)?;
    let size = data.len() as u64;
    tx.send(ExportProgressItem::Size(size)).await?;
    let mut buf = [0u8; 1024 * 64];
    for offset in (0..size).step_by(1024 * 64) {
        let len = std::cmp::min(size - offset, 1024 * 64) as usize;
        let buf = &mut buf[..len];
        data.as_ref().read_exact_at(offset, buf)?;
        file.write_all(buf)?;
        tx.try_send(ExportProgressItem::CopyProgress(offset))
            .await?;
        yield_now().await;
    }
    Ok(())
}
