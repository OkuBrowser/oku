//! # File based blob store.
//!
//! A file based blob store needs a writeable directory to work with.
//!
//! General design:
//!
//! The file store consists of two actors.
//!
//! # The main actor
//!
//! The purpose of the main actor is to handle user commands and own a map of
//! handles for hashes that are currently being worked on.
//!
//! It also owns tasks for ongoing import and export operations, as well as the
//! database actor.
//!
//! Handling a command almost always involves either forwarding it to the
//! database actor or creating a hash context and spawning a task.
//!
//! # The database actor
//!
//! The database actor is responsible for storing metadata about each hash,
//! as well as inlined data and outboard data for small files.
//!
//! In addition to the metadata, the database actor also stores tags.
//!
//! # Tasks
//!
//! Tasks do not return a result. They are responsible for sending an error
//! to the requester if possible. Otherwise, just dropping the sender will
//! also fail the receiver, but without a descriptive error message.
//!
//! Tasks are usually implemented as an impl fn that does return a result,
//! and a wrapper (named `..._task`) that just forwards the error, if any.
//!
//! That way you can use `?` syntax in the task implementation. The impl fns
//! are also easier to test.
//!
//! # Context
//!
//! The main actor holds a TaskContext that is needed for almost all tasks,
//! such as the config and a way to interact with the database.
//!
//! For tasks that are specific to a hash, a HashContext combines the task
//! context with a slot from the table of the main actor that can be used
//! to obtain an unique handle for the hash.
//!
//! # Runtime
//!
//! The fs store owns and manages its own tokio runtime. Dropping the store
//! will clean up the database and shut down the runtime. However, some parts
//! of the persistent state won't make it to disk, so operations involving large
//! partial blobs will have a large initial delay on the next startup.
//!
//! It is also not guaranteed that all write operations will make it to disk.
//! The on-disk store will be in a consistent state, but might miss some writes
//! in the last seconds before shutdown.
//!
//! To avoid this, you can use the [`crate::api::Store::shutdown`] method to
//! cleanly shut down the store and save ephemeral state to disk.
//!
//! Note that if you use the store inside a [`iroh::protocol::Router`] and shut
//! down the router using [`iroh::protocol::Router::shutdown`], the store will be
//! safely shut down as well. Any store refs you are holding will be inoperable
//! after this.
use std::{
    fmt::{self, Debug},
    fs,
    future::Future,
    io::Write,
    num::NonZeroU64,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use bao_tree::{
    blake3,
    io::{
        mixed::{traverse_ranges_validated, EncodedItem, ReadBytesAt},
        outboard::PreOrderOutboard,
        sync::ReadAt,
        BaoContentItem, Leaf,
    },
    BaoTree, ChunkNum, ChunkRanges,
};
use bytes::Bytes;
use delete_set::{BaoFilePart, ProtectHandle};
use entity_manager::{EntityManagerState, SpawnArg};
use entry_state::{DataLocation, OutboardLocation};
use import::{ImportEntry, ImportSource};
use irpc::{channel::mpsc, RpcMessage};
use meta::list_blobs;
use n0_error::{Result, StdResultExt};
use n0_future::{future::yield_now, io};
use nested_enum_utils::enum_conversions;
use range_collections::range_set::RangeSetRange;
use tokio::task::{JoinError, JoinSet};
use tracing::{error, instrument, trace};

use crate::{
    api::{
        proto::{
            self, bitfield::is_validated, BatchMsg, BatchResponse, Bitfield, Command,
            CreateTempTagMsg, ExportBaoMsg, ExportBaoRequest, ExportPathMsg, ExportPathRequest,
            ExportRangesItem, ExportRangesMsg, ExportRangesRequest, HashSpecific, ImportBaoMsg,
            ImportBaoRequest, ObserveMsg, Scope,
        },
        ApiClient,
    },
    protocol::ChunkRangesExt,
    store::{
        fs::{
            bao_file::{
                BaoFileStorage, BaoFileStorageSubscriber, CompleteStorage, DataReader,
                OutboardReader,
            },
            util::entity_manager::{self, ActiveEntityState},
        },
        gc::run_gc,
        util::{BaoTreeSender, FixedSize, MemOrFile, ValueOrPoisioned},
        IROH_BLOCK_SIZE,
    },
    util::{
        channel::oneshot,
        temp_tag::{TagDrop, TempTag, TempTagScope, TempTags},
    },
    Hash,
};
mod bao_file;
use bao_file::BaoFileHandle;
mod delete_set;
mod entry_state;
mod import;
mod meta;
pub mod options;
pub(crate) mod util;
use entry_state::EntryState;
use import::{import_byte_stream, import_bytes, import_path, ImportEntryMsg};
use options::Options;
use tracing::Instrument;

use crate::{
    api::{
        self,
        blobs::{AddProgressItem, ExportMode, ExportProgressItem},
        Store,
    },
    HashAndFormat,
};

/// Maximum number of external paths we track per blob.
const MAX_EXTERNAL_PATHS: usize = 8;

/// Create a 16 byte unique ID.
fn new_uuid() -> [u8; 16] {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    bytes
}

/// Create temp file name based on a 16 byte UUID.
fn temp_name() -> String {
    format!("{}.temp", hex::encode(new_uuid()))
}

#[derive(Debug)]
#[enum_conversions()]
pub(crate) enum InternalCommand {
    Dump(meta::Dump),
    FinishImport(ImportEntryMsg),
    ClearScope(ClearScope),
}

#[derive(Debug)]
pub(crate) struct ClearScope {
    pub scope: Scope,
}

impl InternalCommand {
    pub fn parent_span(&self) -> tracing::Span {
        match self {
            Self::Dump(_) => tracing::Span::current(),
            Self::ClearScope(_) => tracing::Span::current(),
            Self::FinishImport(cmd) => cmd
                .parent_span_opt()
                .cloned()
                .unwrap_or_else(tracing::Span::current),
        }
    }
}

/// Context needed by most tasks
#[derive(Debug)]
struct TaskContext {
    // Store options such as paths and inline thresholds, in an Arc to cheaply share with tasks.
    pub options: Arc<Options>,
    // Metadata database, basically a mpsc sender with some extra functionality.
    pub db: meta::Db,
    // Handle to send internal commands
    pub internal_cmd_tx: tokio::sync::mpsc::Sender<InternalCommand>,
    /// Handle to protect files from deletion.
    pub protect: ProtectHandle,
}

impl TaskContext {
    pub async fn clear_scope(&self, scope: Scope) {
        self.internal_cmd_tx
            .send(ClearScope { scope }.into())
            .await
            .ok();
    }
}

#[derive(Debug)]
struct EmParams;

impl entity_manager::Params for EmParams {
    type EntityId = Hash;

    type GlobalState = Arc<TaskContext>;

    type EntityState = BaoFileHandle;

    async fn on_shutdown(
        state: entity_manager::ActiveEntityState<Self>,
        cause: entity_manager::ShutdownCause,
    ) {
        trace!("persist {:?} due to {cause:?}", state.id);
        state.persist().await;
    }
}

#[derive(Debug)]
struct Actor {
    // Context that can be cheaply shared with tasks.
    context: Arc<TaskContext>,
    // Receiver for incoming user commands.
    cmd_rx: tokio::sync::mpsc::Receiver<Command>,
    // Receiver for incoming file store specific commands.
    fs_cmd_rx: tokio::sync::mpsc::Receiver<InternalCommand>,
    // Tasks for import and export operations.
    tasks: JoinSet<()>,
    // Entity manager that handles concurrency for entities.
    handles: EntityManagerState<EmParams>,
    // temp tags
    temp_tags: TempTags,
    // waiters for idle state.
    idle_waiters: Vec<irpc::channel::oneshot::Sender<()>>,
    // our private tokio runtime. It has to live somewhere.
    _rt: RtWrapper,
}

type HashContext = ActiveEntityState<EmParams>;

impl SyncEntityApi for HashContext {
    /// Load the state from the database.
    ///
    /// If the state is Initial, this will start the load.
    /// If it is Loading, it will wait until loading is done.
    /// If it is any other state, it will be a noop.
    async fn load(&self) {
        enum Action {
            Load,
            Wait,
            None,
        }
        let mut action = Action::None;
        self.state.send_if_modified(|guard| match guard.deref() {
            BaoFileStorage::Initial => {
                *guard = BaoFileStorage::Loading;
                action = Action::Load;
                true
            }
            BaoFileStorage::Loading => {
                action = Action::Wait;
                false
            }
            _ => false,
        });
        match action {
            Action::Load => {
                let state = if self.id == Hash::EMPTY {
                    BaoFileStorage::Complete(CompleteStorage {
                        data: MemOrFile::Mem(Bytes::new()),
                        outboard: MemOrFile::empty(),
                    })
                } else {
                    // we must assign a new state even in the error case, otherwise
                    // tasks waiting for loading would stall!
                    match self.global.db.get(self.id).await {
                        Ok(state) => match BaoFileStorage::open(state, self).await {
                            Ok(handle) => handle,
                            Err(_) => BaoFileStorage::Poisoned,
                        },
                        Err(_) => BaoFileStorage::Poisoned,
                    }
                };
                self.state.send_replace(state);
            }
            Action::Wait => {
                // we are in state loading already, so we just need to wait for the
                // other task to complete loading.
                while matches!(self.state.borrow().deref(), BaoFileStorage::Loading) {
                    self.state.0.subscribe().changed().await.ok();
                }
            }
            Action::None => {}
        }
    }

    /// Write a batch and notify the db
    async fn write_batch(&self, batch: &[BaoContentItem], bitfield: &Bitfield) -> io::Result<()> {
        trace!("write_batch bitfield={:?} batch={}", bitfield, batch.len());
        let mut res = Ok(None);
        self.state.send_if_modified(|state| {
            let Ok((state1, update)) = state.take().write_batch(batch, bitfield, self) else {
                res = Err(io::Error::other("write batch failed"));
                return false;
            };
            res = Ok(update);
            *state = state1;
            true
        });
        if let Some(update) = res? {
            self.global.db.update(self.id, update).await?;
        }
        Ok(())
    }

    /// An AsyncSliceReader for the data file.
    ///
    /// Caution: this is a reader for the unvalidated data file. Reading this
    /// can produce data that does not match the hash.
    #[allow(refining_impl_trait_internal)]
    fn data_reader(&self) -> DataReader {
        DataReader(self.state.clone())
    }

    /// An AsyncSliceReader for the outboard file.
    ///
    /// The outboard file is used to validate the data file. It is not guaranteed
    /// to be complete.
    #[allow(refining_impl_trait_internal)]
    fn outboard_reader(&self) -> OutboardReader {
        OutboardReader(self.state.clone())
    }

    /// The most precise known total size of the data file.
    fn current_size(&self) -> io::Result<u64> {
        match self.state.borrow().deref() {
            BaoFileStorage::Complete(mem) => Ok(mem.size()),
            BaoFileStorage::PartialMem(mem) => Ok(mem.current_size()),
            BaoFileStorage::Partial(file) => file.current_size(),
            BaoFileStorage::Poisoned => Err(io::Error::other("poisoned storage")),
            BaoFileStorage::Initial => Err(io::Error::other("initial")),
            BaoFileStorage::Loading => Err(io::Error::other("loading")),
            BaoFileStorage::NonExisting => Err(io::ErrorKind::NotFound.into()),
        }
    }

    /// The most precise known total size of the data file.
    fn bitfield(&self) -> io::Result<Bitfield> {
        match self.state.borrow().deref() {
            BaoFileStorage::Complete(mem) => Ok(mem.bitfield()),
            BaoFileStorage::PartialMem(mem) => Ok(mem.bitfield().clone()),
            BaoFileStorage::Partial(file) => Ok(file.bitfield().clone()),
            BaoFileStorage::Poisoned => Err(io::Error::other("poisoned storage")),
            BaoFileStorage::Initial => Err(io::Error::other("initial")),
            BaoFileStorage::Loading => Err(io::Error::other("loading")),
            BaoFileStorage::NonExisting => Err(io::ErrorKind::NotFound.into()),
        }
    }
}

impl HashContext {
    /// The outboard for the file.
    pub fn outboard(&self) -> io::Result<PreOrderOutboard<OutboardReader>> {
        let tree = BaoTree::new(self.current_size()?, IROH_BLOCK_SIZE);
        let outboard = self.outboard_reader();
        Ok(PreOrderOutboard {
            root: blake3::Hash::from(self.id),
            tree,
            data: outboard,
        })
    }

    fn db(&self) -> &meta::Db {
        &self.global.db
    }

    pub fn options(&self) -> &Arc<Options> {
        &self.global.options
    }

    pub fn protect(&self, parts: impl IntoIterator<Item = BaoFilePart>) {
        self.global.protect.protect(self.id, parts);
    }

    /// Update the entry state in the database, and wait for completion.
    pub async fn update_await(&self, state: EntryState<Bytes>) -> io::Result<()> {
        self.db().update_await(self.id, state).await?;
        Ok(())
    }

    pub async fn get_entry_state(&self) -> io::Result<Option<EntryState<Bytes>>> {
        let hash = self.id;
        if hash == Hash::EMPTY {
            return Ok(Some(EntryState::Complete {
                data_location: DataLocation::Inline(Bytes::new()),
                outboard_location: OutboardLocation::NotNeeded,
            }));
        };
        self.db().get(hash).await
    }

    /// Update the entry state in the database, and wait for completion.
    pub async fn set(&self, state: EntryState<Bytes>) -> io::Result<()> {
        self.db().set(self.id, state).await
    }
}

impl Actor {
    fn db(&self) -> &meta::Db {
        &self.context.db
    }

    fn context(&self) -> Arc<TaskContext> {
        self.context.clone()
    }

    fn spawn(&mut self, fut: impl Future<Output = ()> + Send + 'static) {
        let span = tracing::Span::current();
        self.tasks.spawn(fut.instrument(span));
    }

    fn log_task_result(res: Result<(), JoinError>) {
        match res {
            Ok(_) => {}
            Err(e) => {
                error!("task failed: {e}");
            }
        }
    }

    async fn create_temp_tag(&mut self, cmd: CreateTempTagMsg) {
        let CreateTempTagMsg { tx, inner, .. } = cmd;
        let mut tt = self.temp_tags.create(inner.scope, inner.value);
        if tx.is_rpc() {
            tt.leak();
        }
        tx.send(tt).await.ok();
    }

    async fn handle_command(&mut self, cmd: Command) {
        let span = cmd.parent_span();
        let _entered = span.enter();
        match cmd {
            Command::SyncDb(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::WaitIdle(cmd) => {
                trace!("{cmd:?}");
                if self.tasks.is_empty() {
                    // we are currently idle
                    cmd.tx.send(()).await.ok();
                } else {
                    // wait for idle state
                    self.idle_waiters.push(cmd.tx);
                }
            }
            Command::Shutdown(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::CreateTag(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::SetTag(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::ListTags(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::DeleteTags(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::RenameTag(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::ClearProtected(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::BlobStatus(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::DeleteBlobs(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            Command::ListBlobs(cmd) => {
                trace!("{cmd:?}");
                if let Ok(snapshot) = self.db().snapshot(cmd.span.clone()).await {
                    self.spawn(list_blobs(snapshot, cmd));
                }
            }
            Command::Batch(cmd) => {
                trace!("{cmd:?}");
                let (id, scope) = self.temp_tags.create_scope();
                self.spawn(handle_batch(cmd, id, scope, self.context()));
            }
            Command::CreateTempTag(cmd) => {
                trace!("{cmd:?}");
                self.create_temp_tag(cmd).await;
            }
            Command::ListTempTags(cmd) => {
                trace!("{cmd:?}");
                let tts = self.temp_tags.list();
                cmd.tx.send(tts).await.ok();
            }
            Command::ImportBytes(cmd) => {
                trace!("{cmd:?}");
                self.spawn(import_bytes(cmd, self.context()));
            }
            Command::ImportByteStream(cmd) => {
                trace!("{cmd:?}");
                self.spawn(import_byte_stream(cmd, self.context()));
            }
            Command::ImportPath(cmd) => {
                trace!("{cmd:?}");
                self.spawn(import_path(cmd, self.context()));
            }
            Command::ExportPath(cmd) => {
                trace!("{cmd:?}");
                cmd.spawn(&mut self.handles, &mut self.tasks).await;
            }
            Command::ExportBao(cmd) => {
                trace!("{cmd:?}");
                cmd.spawn(&mut self.handles, &mut self.tasks).await;
            }
            Command::ExportRanges(cmd) => {
                trace!("{cmd:?}");
                cmd.spawn(&mut self.handles, &mut self.tasks).await;
            }
            Command::ImportBao(cmd) => {
                trace!("{cmd:?}");
                cmd.spawn(&mut self.handles, &mut self.tasks).await;
            }
            Command::Observe(cmd) => {
                trace!("{cmd:?}");
                cmd.spawn(&mut self.handles, &mut self.tasks).await;
            }
        }
    }

    async fn handle_fs_command(&mut self, cmd: InternalCommand) {
        let span = cmd.parent_span();
        let _entered = span.enter();
        match cmd {
            InternalCommand::Dump(cmd) => {
                trace!("{cmd:?}");
                self.db().send(cmd.into()).await.ok();
            }
            InternalCommand::ClearScope(cmd) => {
                trace!("{cmd:?}");
                self.temp_tags.end_scope(cmd.scope);
            }
            InternalCommand::FinishImport(cmd) => {
                trace!("{cmd:?}");
                if cmd.hash == Hash::EMPTY {
                    cmd.tx
                        .send(AddProgressItem::Done(TempTag::leaking_empty(cmd.format)))
                        .await
                        .ok();
                } else {
                    let tt = self.temp_tags.create(
                        cmd.scope,
                        HashAndFormat {
                            hash: cmd.hash,
                            format: cmd.format,
                        },
                    );
                    (tt, cmd).spawn(&mut self.handles, &mut self.tasks).await;
                }
            }
        }
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                task = self.handles.tick() => {
                    if let Some(task) = task {
                        self.spawn(task);
                    }
                }
                cmd = self.cmd_rx.recv() => {
                    let Some(cmd) = cmd else {
                        break;
                    };
                    self.handle_command(cmd).await;
                }
                Some(cmd) = self.fs_cmd_rx.recv() => {
                    self.handle_fs_command(cmd).await;
                }
                Some(res) = self.tasks.join_next(), if !self.tasks.is_empty() => {
                    Self::log_task_result(res);
                    if self.tasks.is_empty() {
                        for tx in self.idle_waiters.drain(..) {
                            tx.send(()).await.ok();
                        }
                    }
                }
            }
        }
        self.handles.shutdown().await;
        while let Some(res) = self.tasks.join_next().await {
            Self::log_task_result(res);
        }
    }

    async fn new(
        db_path: PathBuf,
        rt: RtWrapper,
        cmd_rx: tokio::sync::mpsc::Receiver<Command>,
        fs_commands_rx: tokio::sync::mpsc::Receiver<InternalCommand>,
        fs_commands_tx: tokio::sync::mpsc::Sender<InternalCommand>,
        options: Arc<Options>,
    ) -> Result<Self> {
        trace!(
            "creating data directory: {}",
            options.path.data_path.display()
        );
        fs::create_dir_all(&options.path.data_path)?;
        trace!(
            "creating temp directory: {}",
            options.path.temp_path.display()
        );
        fs::create_dir_all(&options.path.temp_path)?;
        trace!(
            "creating parent directory for db file{}",
            db_path.parent().unwrap().display()
        );
        fs::create_dir_all(db_path.parent().unwrap())?;
        let (db_send, db_recv) = tokio::sync::mpsc::channel(100);
        let (protect, ds) = delete_set::pair(Arc::new(options.path.clone()));
        let db_actor = meta::Actor::new(db_path, db_recv, ds, options.batch.clone())?;
        let slot_context = Arc::new(TaskContext {
            options: options.clone(),
            db: meta::Db::new(db_send),
            internal_cmd_tx: fs_commands_tx,
            protect,
        });
        rt.spawn(db_actor.run());
        Ok(Self {
            context: slot_context.clone(),
            cmd_rx,
            fs_cmd_rx: fs_commands_rx,
            tasks: JoinSet::new(),
            handles: EntityManagerState::new(slot_context, 1024, 32, 32, 2),
            temp_tags: Default::default(),
            idle_waiters: Vec::new(),
            _rt: rt,
        })
    }
}

trait HashSpecificCommand: HashSpecific + Send + 'static {
    /// Handle the command on success by spawning a task into the per-hash context.
    fn handle(self, ctx: HashContext) -> impl Future<Output = ()> + Send + 'static;

    /// Opportunity to send an error if spawning fails due to the task being busy (inbox full)
    /// or dead (e.g. panic in one of the running tasks).
    fn on_error(self, arg: SpawnArg<EmParams>) -> impl Future<Output = ()> + Send + 'static;

    async fn spawn(
        self,
        manager: &mut entity_manager::EntityManagerState<EmParams>,
        tasks: &mut JoinSet<()>,
    ) where
        Self: Sized,
    {
        let span = tracing::Span::current();
        let task = manager
            .spawn(self.hash(), |arg| {
                async move {
                    match arg {
                        SpawnArg::Active(state) => {
                            self.handle(state).await;
                        }
                        SpawnArg::Busy => {
                            self.on_error(arg).await;
                        }
                        SpawnArg::Dead => {
                            self.on_error(arg).await;
                        }
                    }
                }
                .instrument(span)
            })
            .await;
        if let Some(task) = task {
            tasks.spawn(task);
        }
    }
}

impl HashSpecificCommand for ObserveMsg {
    async fn handle(self, ctx: HashContext) {
        ctx.observe(self).await
    }
    async fn on_error(self, _arg: SpawnArg<EmParams>) {}
}
impl HashSpecificCommand for ExportPathMsg {
    async fn handle(self, ctx: HashContext) {
        ctx.export_path(self).await
    }
    async fn on_error(self, arg: SpawnArg<EmParams>) {
        let err = match arg {
            SpawnArg::Busy => io::ErrorKind::ResourceBusy.into(),
            SpawnArg::Dead => err_entity_dead(),
            _ => unreachable!(),
        };
        self.tx
            .send(ExportProgressItem::Error(api::Error::from(err)))
            .await
            .ok();
    }
}
impl HashSpecificCommand for ExportBaoMsg {
    async fn handle(self, ctx: HashContext) {
        ctx.export_bao(self).await
    }
    async fn on_error(self, arg: SpawnArg<EmParams>) {
        let err = match arg {
            SpawnArg::Busy => io::ErrorKind::ResourceBusy.into(),
            SpawnArg::Dead => err_entity_dead(),
            _ => unreachable!(),
        };
        self.tx
            .send(EncodedItem::Error(bao_tree::io::EncodeError::Io(err)))
            .await
            .ok();
    }
}
impl HashSpecificCommand for ExportRangesMsg {
    async fn handle(self, ctx: HashContext) {
        ctx.export_ranges(self).await
    }
    async fn on_error(self, arg: SpawnArg<EmParams>) {
        let err = match arg {
            SpawnArg::Busy => io::ErrorKind::ResourceBusy.into(),
            SpawnArg::Dead => err_entity_dead(),
            _ => unreachable!(),
        };
        self.tx
            .send(ExportRangesItem::Error(api::Error::from(err)))
            .await
            .ok();
    }
}
impl HashSpecificCommand for ImportBaoMsg {
    async fn handle(self, ctx: HashContext) {
        ctx.import_bao(self).await
    }
    async fn on_error(self, arg: SpawnArg<EmParams>) {
        let err = match arg {
            SpawnArg::Busy => io::ErrorKind::ResourceBusy.into(),
            SpawnArg::Dead => err_entity_dead(),
            _ => unreachable!(),
        };
        self.tx.send(Err(api::Error::from(err))).await.ok();
    }
}
impl HashSpecific for (TempTag, ImportEntryMsg) {
    fn hash(&self) -> Hash {
        self.1.hash()
    }
}
impl HashSpecificCommand for (TempTag, ImportEntryMsg) {
    async fn handle(self, ctx: HashContext) {
        let (tt, cmd) = self;
        ctx.finish_import(cmd, tt).await
    }
    async fn on_error(self, arg: SpawnArg<EmParams>) {
        let err = match arg {
            SpawnArg::Busy => io::ErrorKind::ResourceBusy.into(),
            SpawnArg::Dead => err_entity_dead(),
            _ => unreachable!(),
        };
        self.1.tx.send(AddProgressItem::Error(err)).await.ok();
    }
}

fn err_entity_dead() -> io::Error {
    io::Error::other("entity is dead")
}

struct RtWrapper(Option<tokio::runtime::Runtime>);

impl From<tokio::runtime::Runtime> for RtWrapper {
    fn from(rt: tokio::runtime::Runtime) -> Self {
        Self(Some(rt))
    }
}

impl fmt::Debug for RtWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ValueOrPoisioned(self.0.as_ref()).fmt(f)
    }
}

impl Deref for RtWrapper {
    type Target = tokio::runtime::Runtime;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl Drop for RtWrapper {
    fn drop(&mut self) {
        if let Some(rt) = self.0.take() {
            trace!("dropping tokio runtime");
            tokio::task::block_in_place(|| {
                drop(rt);
            });
            trace!("dropped tokio runtime");
        }
    }
}

async fn handle_batch(cmd: BatchMsg, id: Scope, scope: Arc<TempTagScope>, ctx: Arc<TaskContext>) {
    if let Err(cause) = handle_batch_impl(cmd, id, &scope).await {
        error!("batch failed: {cause}");
    }
    ctx.clear_scope(id).await;
}

async fn handle_batch_impl(cmd: BatchMsg, id: Scope, scope: &Arc<TempTagScope>) -> api::Result<()> {
    let BatchMsg { tx, mut rx, .. } = cmd;
    trace!("created scope {}", id);
    tx.send(id).await?;
    while let Some(msg) = rx.recv().await? {
        match msg {
            BatchResponse::Drop(msg) => scope.on_drop(&msg),
            BatchResponse::Ping => {}
        }
    }
    Ok(())
}

/// The minimal API you need to implement for an entity for a store to work.
trait EntityApi {
    /// Import from a stream of n0 bao encoded data.
    async fn import_bao(&self, cmd: ImportBaoMsg);
    /// Finish an import from a local file or memory.
    async fn finish_import(&self, cmd: ImportEntryMsg, tt: TempTag);
    /// Observe the bitfield of the entry.
    async fn observe(&self, cmd: ObserveMsg);
    /// Export byte ranges of the entry as data
    async fn export_ranges(&self, cmd: ExportRangesMsg);
    /// Export chunk ranges of the entry as a n0 bao encoded stream.
    async fn export_bao(&self, cmd: ExportBaoMsg);
    /// Export the entry to a local file.
    async fn export_path(&self, cmd: ExportPathMsg);
    /// Persist the entry at the end of its lifecycle.
    async fn persist(&self);
}

/// A more opinionated API that can be used as a helper to save implementation
/// effort when implementing the EntityApi trait.
trait SyncEntityApi: EntityApi {
    /// Load the entry state from the database. This must make sure that it is
    /// not run concurrently, so if load is called multiple times, all but one
    /// must wait. You can use a tokio::sync::OnceCell or similar to achieve this.
    async fn load(&self);

    /// Get a synchronous reader for the data file.
    fn data_reader(&self) -> impl ReadBytesAt;

    /// Get a synchronous reader for the outboard file.
    fn outboard_reader(&self) -> impl ReadAt;

    /// Get the best known size of the data file.
    fn current_size(&self) -> io::Result<u64>;

    /// Get the bitfield of the entry.
    fn bitfield(&self) -> io::Result<Bitfield>;

    /// Write a batch of content items to the entry.
    async fn write_batch(&self, batch: &[BaoContentItem], bitfield: &Bitfield) -> io::Result<()>;
}

/// The high level entry point per entry.
impl EntityApi for HashContext {
    #[instrument(skip_all, fields(hash = %cmd.hash_short()))]
    async fn import_bao(&self, cmd: ImportBaoMsg) {
        trace!("{cmd:?}");
        self.load().await;
        let ImportBaoMsg {
            inner: ImportBaoRequest { size, .. },
            rx,
            tx,
            ..
        } = cmd;
        let res = import_bao_impl(self, size, rx).await;
        trace!("{res:?}");
        tx.send(res).await.ok();
    }

    #[instrument(skip_all, fields(hash = %cmd.hash_short()))]
    async fn observe(&self, cmd: ObserveMsg) {
        trace!("{cmd:?}");
        self.load().await;
        BaoFileStorageSubscriber::new(self.state.subscribe())
            .forward(cmd.tx)
            .await
            .ok();
    }

    #[instrument(skip_all, fields(hash = %cmd.hash_short()))]
    async fn export_ranges(&self, mut cmd: ExportRangesMsg) {
        trace!("{cmd:?}");
        self.load().await;
        if let Err(cause) = export_ranges_impl(self, cmd.inner, &mut cmd.tx).await {
            cmd.tx
                .send(ExportRangesItem::Error(cause.into()))
                .await
                .ok();
        }
    }

    #[instrument(skip_all, fields(hash = %cmd.hash_short()))]
    async fn export_bao(&self, mut cmd: ExportBaoMsg) {
        trace!("{cmd:?}");
        self.load().await;
        if let Err(cause) = export_bao_impl(self, cmd.inner, &mut cmd.tx).await {
            // if the entry is in state NonExisting, this will be an io error with
            // kind NotFound. So we must not wrap this somehow but pass it on directly.
            cmd.tx
                .send(bao_tree::io::EncodeError::Io(cause).into())
                .await
                .ok();
        }
    }

    #[instrument(skip_all, fields(hash = %cmd.hash_short()))]
    async fn export_path(&self, cmd: ExportPathMsg) {
        trace!("{cmd:?}");
        self.load().await;
        let ExportPathMsg { inner, mut tx, .. } = cmd;
        if let Err(cause) = export_path_impl(self, inner, &mut tx).await {
            tx.send(cause.into()).await.ok();
        }
    }

    #[instrument(skip_all, fields(hash = %cmd.hash_short()))]
    async fn finish_import(&self, cmd: ImportEntryMsg, mut tt: TempTag) {
        trace!("{cmd:?}");
        self.load().await;
        let res = match finish_import_impl(self, cmd.inner).await {
            Ok(()) => {
                // for a remote call, we can't have the on_drop callback, so we have to leak the temp tag
                // it will be cleaned up when either the process exits or scope ends
                if cmd.tx.is_rpc() {
                    trace!("leaking temp tag {}", tt.hash_and_format());
                    tt.leak();
                }
                AddProgressItem::Done(tt)
            }
            Err(cause) => AddProgressItem::Error(cause),
        };
        cmd.tx.send(res).await.ok();
    }

    #[instrument(skip_all, fields(hash = %self.id.fmt_short()))]
    async fn persist(&self) {
        self.state.send_if_modified(|guard| {
            let hash = &self.id;
            let BaoFileStorage::Partial(fs) = guard.take() else {
                return false;
            };
            let path = self.global.options.path.bitfield_path(hash);
            trace!("writing bitfield for hash {} to {}", hash, path.display());
            if let Err(cause) = fs.sync_all(&path) {
                error!(
                    "failed to write bitfield for {} at {}: {:?}",
                    hash,
                    path.display(),
                    cause
                );
            }
            false
        });
    }
}

async fn finish_import_impl(ctx: &HashContext, import_data: ImportEntry) -> io::Result<()> {
    if ctx.id == Hash::EMPTY {
        return Ok(()); // nothing to do for the empty hash
    }
    let ImportEntry {
        source,
        hash,
        outboard,
        ..
    } = import_data;
    let options = ctx.options();
    match &source {
        ImportSource::Memory(data) => {
            debug_assert!(options.is_inlined_data(data.len() as u64));
        }
        ImportSource::External(_, _, size) => {
            debug_assert!(!options.is_inlined_data(*size));
        }
        ImportSource::TempFile(_, _, size) => {
            debug_assert!(!options.is_inlined_data(*size));
        }
    }
    ctx.load().await;
    let handle = &ctx.state;
    // if I do have an existing handle, I have to possibly deal with observers.
    // if I don't have an existing handle, there are 2 cases:
    //   the entry exists in the db, but we don't have a handle
    //   the entry does not exist at all.
    // convert the import source to a data location and drop the open files
    ctx.protect([BaoFilePart::Data, BaoFilePart::Outboard]);
    let data_location = match source {
        ImportSource::Memory(data) => DataLocation::Inline(data),
        ImportSource::External(path, _file, size) => DataLocation::External(vec![path], size),
        ImportSource::TempFile(path, _file, size) => {
            // this will always work on any unix, but on windows there might be an issue if the target file is open!
            // possibly open with FILE_SHARE_DELETE on windows?
            let target = ctx.options().path.data_path(&hash);
            trace!(
                "moving temp file to owned data location: {} -> {}",
                path.display(),
                target.display()
            );
            if let Err(cause) = fs::rename(&path, &target) {
                error!(
                    "failed to move temp file {} to owned data location {}: {cause}",
                    path.display(),
                    target.display()
                );
            }
            DataLocation::Owned(size)
        }
    };
    let outboard_location = match outboard {
        MemOrFile::Mem(bytes) if bytes.is_empty() => OutboardLocation::NotNeeded,
        MemOrFile::Mem(bytes) => OutboardLocation::Inline(bytes),
        MemOrFile::File(path) => {
            // the same caveat as above applies here
            let target = ctx.options().path.outboard_path(&hash);
            trace!(
                "moving temp file to owned outboard location: {} -> {}",
                path.display(),
                target.display()
            );
            if let Err(cause) = fs::rename(&path, &target) {
                error!(
                    "failed to move temp file {} to owned outboard location {}: {cause}",
                    path.display(),
                    target.display()
                );
            }
            OutboardLocation::Owned
        }
    };
    let data = match &data_location {
        DataLocation::Inline(data) => MemOrFile::Mem(data.clone()),
        DataLocation::Owned(size) => {
            let path = ctx.options().path.data_path(&hash);
            let file = fs::File::open(&path)?;
            MemOrFile::File(FixedSize::new(file, *size))
        }
        DataLocation::External(paths, size) => {
            let Some(path) = paths.iter().next() else {
                return Err(io::Error::other("no external data path"));
            };
            let file = fs::File::open(path)?;
            MemOrFile::File(FixedSize::new(file, *size))
        }
    };
    let outboard = match &outboard_location {
        OutboardLocation::NotNeeded => MemOrFile::empty(),
        OutboardLocation::Inline(data) => MemOrFile::Mem(data.clone()),
        OutboardLocation::Owned => {
            let path = ctx.options().path.outboard_path(&hash);
            let file = fs::File::open(&path)?;
            MemOrFile::File(file)
        }
    };
    handle.complete(data, outboard);
    let state = EntryState::Complete {
        data_location,
        outboard_location,
    };
    ctx.update_await(state).await?;
    Ok(())
}

fn chunk_range(leaf: &Leaf) -> ChunkRanges {
    let start = ChunkNum::chunks(leaf.offset);
    let end = ChunkNum::chunks(leaf.offset + leaf.data.len() as u64);
    (start..end).into()
}

async fn import_bao_impl(
    ctx: &HashContext,
    size: NonZeroU64,
    mut rx: mpsc::Receiver<BaoContentItem>,
) -> api::Result<()> {
    trace!("importing bao: {} {} bytes", ctx.id.fmt_short(), size);
    let mut batch = Vec::<BaoContentItem>::new();
    let mut ranges = ChunkRanges::empty();
    while let Some(item) = rx.recv().await? {
        // if the batch is not empty, the last item is a leaf and the current item is a parent, write the batch
        if !batch.is_empty() && batch[batch.len() - 1].is_leaf() && item.is_parent() {
            let bitfield = Bitfield::new_unchecked(ranges, size.into());
            ctx.write_batch(&batch, &bitfield).await?;
            batch.clear();
            ranges = ChunkRanges::empty();
        }
        if let BaoContentItem::Leaf(leaf) = &item {
            let leaf_range = chunk_range(leaf);
            if is_validated(size, &leaf_range) && size.get() != leaf.offset + leaf.data.len() as u64
            {
                return Err(api::Error::io(io::ErrorKind::InvalidData, "invalid size"));
            }
            ranges |= leaf_range;
        }
        batch.push(item);
    }
    if !batch.is_empty() {
        let bitfield = Bitfield::new_unchecked(ranges, size.into());
        ctx.write_batch(&batch, &bitfield).await?;
    }
    Ok(())
}

async fn export_ranges_impl(
    ctx: &HashContext,
    cmd: ExportRangesRequest,
    tx: &mut mpsc::Sender<ExportRangesItem>,
) -> io::Result<()> {
    let ExportRangesRequest { ranges, hash } = cmd;
    trace!(
        "exporting ranges: {hash} {ranges:?} size={}",
        ctx.current_size()?
    );
    let bitfield = ctx.bitfield()?;
    let data = ctx.data_reader();
    let size = bitfield.size();
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
            let res = data.read_bytes_at(offset, size);
            tx.send(ExportRangesItem::Data(Leaf { offset, data: res? }))
                .await?;
            offset = end;
            if offset >= range.end {
                break;
            }
        }
    }
    Ok(())
}

async fn export_bao_impl(
    ctx: &HashContext,
    cmd: ExportBaoRequest,
    tx: &mut mpsc::Sender<EncodedItem>,
) -> io::Result<()> {
    let ExportBaoRequest { ranges, hash, .. } = cmd;
    let outboard = ctx.outboard()?;
    let size = outboard.tree.size();
    if size == 0 && cmd.hash != Hash::EMPTY {
        // we have no data whatsoever, so we stop here
        return Ok(());
    }
    trace!("exporting bao: {hash} {ranges:?} size={size}",);
    let data = ctx.data_reader();
    let tx = BaoTreeSender::new(tx);
    traverse_ranges_validated(data, outboard, &ranges, tx).await?;
    Ok(())
}

async fn export_path_impl(
    ctx: &HashContext,
    cmd: ExportPathRequest,
    tx: &mut mpsc::Sender<ExportProgressItem>,
) -> api::Result<()> {
    let ExportPathRequest { mode, target, .. } = cmd;
    if !target.is_absolute() {
        return Err(api::Error::io(
            io::ErrorKind::InvalidInput,
            "path is not absolute",
        ));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let state = ctx.get_entry_state().await?;
    let (data_location, outboard_location) = match state {
        Some(EntryState::Complete {
            data_location,
            outboard_location,
        }) => (data_location, outboard_location),
        Some(EntryState::Partial { .. }) => {
            return Err(api::Error::io(
                io::ErrorKind::InvalidInput,
                "cannot export partial entry",
            ));
        }
        None => {
            return Err(api::Error::io(io::ErrorKind::NotFound, "no entry found"));
        }
    };
    trace!("exporting {} to {}", cmd.hash.to_hex(), target.display());
    let (data, mut external) = match data_location {
        DataLocation::Inline(data) => (MemOrFile::Mem(data), vec![]),
        DataLocation::Owned(size) => (
            MemOrFile::File((ctx.options().path.data_path(&cmd.hash), size)),
            vec![],
        ),
        DataLocation::External(paths, size) => (
            MemOrFile::File((
                paths.first().cloned().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "no external data path")
                })?,
                size,
            )),
            paths,
        ),
    };
    let size = match &data {
        MemOrFile::Mem(data) => data.len() as u64,
        MemOrFile::File((_, size)) => *size,
    };
    tx.send(ExportProgressItem::Size(size)).await?;
    match data {
        MemOrFile::Mem(data) => {
            let mut target = fs::File::create(&target)?;
            target.write_all(&data)?;
        }
        MemOrFile::File((source_path, size)) => match mode {
            ExportMode::Copy => {
                let res = reflink_or_copy_with_progress(&source_path, &target, size, tx).await?;
                trace!(
                    "exported {} to {}, {res:?}",
                    source_path.display(),
                    target.display()
                );
            }
            ExportMode::TryReference => {
                if !external.is_empty() {
                    // the file already exists externally, so we need to copy it.
                    // if the OS supports reflink, we might as well use that.
                    let res =
                        reflink_or_copy_with_progress(&source_path, &target, size, tx).await?;
                    trace!(
                        "exported {} also to {}, {res:?}",
                        source_path.display(),
                        target.display()
                    );
                    external.push(target);
                    external.sort();
                    external.dedup();
                    external.truncate(MAX_EXTERNAL_PATHS);
                } else {
                    // the file was previously owned, so we can just move it.
                    // if that fails with ERR_CROSS, we fall back to copy.
                    match std::fs::rename(&source_path, &target) {
                        Ok(()) => {}
                        Err(cause) => {
                            const ERR_CROSS: i32 = 18;
                            if cause.raw_os_error() == Some(ERR_CROSS) {
                                reflink_or_copy_with_progress(&source_path, &target, size, tx)
                                    .await?;
                            } else {
                                return Err(cause.into());
                            }
                        }
                    }
                    external.push(target);
                };
                // setting the new entry state will also take care of deleting the owned data file!
                ctx.set(EntryState::Complete {
                    data_location: DataLocation::External(external, size),
                    outboard_location,
                })
                .await?;
            }
        },
    }
    tx.send(ExportProgressItem::Done).await?;
    Ok(())
}

trait CopyProgress: RpcMessage {
    fn from_offset(offset: u64) -> Self;
}

impl CopyProgress for ExportProgressItem {
    fn from_offset(offset: u64) -> Self {
        ExportProgressItem::CopyProgress(offset)
    }
}

impl CopyProgress for AddProgressItem {
    fn from_offset(offset: u64) -> Self {
        AddProgressItem::CopyProgress(offset)
    }
}

#[derive(Debug)]
enum CopyResult {
    Reflinked,
    Copied,
}

async fn reflink_or_copy_with_progress(
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
    size: u64,
    tx: &mut mpsc::Sender<impl CopyProgress>,
) -> io::Result<CopyResult> {
    let from = from.as_ref();
    let to = to.as_ref();
    if reflink_copy::reflink(from, to).is_ok() {
        return Ok(CopyResult::Reflinked);
    }
    let source = fs::File::open(from)?;
    let mut target = fs::File::create(to)?;
    copy_with_progress(source, size, &mut target, tx).await?;
    Ok(CopyResult::Copied)
}

async fn copy_with_progress<T: CopyProgress>(
    file: impl ReadAt,
    size: u64,
    target: &mut impl Write,
    tx: &mut mpsc::Sender<T>,
) -> io::Result<()> {
    let mut offset = 0;
    let mut buf = vec![0u8; 1024 * 1024];
    while offset < size {
        let remaining = buf.len().min((size - offset) as usize);
        let buf: &mut [u8] = &mut buf[..remaining];
        file.read_exact_at(offset, buf)?;
        target.write_all(buf)?;
        tx.try_send(T::from_offset(offset)).await?;
        yield_now().await;
        offset += buf.len() as u64;
    }
    Ok(())
}

impl FsStore {
    /// Load or create a new store.
    pub async fn load(root: impl AsRef<Path>) -> Result<Self> {
        let path = root.as_ref();
        let db_path = path.join("blobs.db");
        let options = Options::new(path);
        Self::load_with_opts(db_path, options).await
    }

    /// Load or create a new store with custom options, returning an additional sender for file store specific commands.
    pub async fn load_with_opts(db_path: PathBuf, options: Options) -> Result<FsStore> {
        static THREAD_NR: AtomicU64 = AtomicU64::new(0);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_name_fn(|| {
                format!(
                    "iroh-blob-store-{}",
                    THREAD_NR.fetch_add(1, Ordering::Relaxed)
                )
            })
            .enable_time()
            .build()?;
        let handle = rt.handle().clone();
        let (commands_tx, commands_rx) = tokio::sync::mpsc::channel(100);
        let (fs_commands_tx, fs_commands_rx) = tokio::sync::mpsc::channel(100);
        let gc_config = options.gc.clone();
        let actor = handle
            .spawn(Actor::new(
                db_path,
                rt.into(),
                commands_rx,
                fs_commands_rx,
                fs_commands_tx.clone(),
                Arc::new(options),
            ))
            .await
            .anyerr()??;
        handle.spawn(actor.run());
        let store = FsStore::new(commands_tx.into(), fs_commands_tx);
        if let Some(config) = gc_config {
            handle.spawn(run_gc(store.deref().clone(), config));
        }
        Ok(store)
    }
}

/// A file based store.
///
/// A store can be created using [`load`](FsStore::load) or [`load_with_opts`](FsStore::load_with_opts).
/// Load will use the default options and create the required directories, while load_with_opts allows
/// you to customize the options and the location of the database. Both variants will create the database
/// if it does not exist, and load an existing database if one is found at the configured location.
///
/// In addition to implementing the [`Store`](`crate::api::Store`) API via [`Deref`](`std::ops::Deref`),
/// there are a few additional methods that are specific to file based stores, such as [`dump`](FsStore::dump).
#[derive(Debug, Clone)]
pub struct FsStore {
    sender: ApiClient,
    db: tokio::sync::mpsc::Sender<InternalCommand>,
}

impl From<FsStore> for Store {
    fn from(value: FsStore) -> Self {
        Store::from_sender(value.sender)
    }
}

impl Deref for FsStore {
    type Target = Store;

    fn deref(&self) -> &Self::Target {
        Store::ref_from_sender(&self.sender)
    }
}

impl AsRef<Store> for FsStore {
    fn as_ref(&self) -> &Store {
        self.deref()
    }
}

impl FsStore {
    fn new(
        sender: irpc::LocalSender<proto::Request>,
        db: tokio::sync::mpsc::Sender<InternalCommand>,
    ) -> Self {
        Self {
            sender: sender.into(),
            db,
        }
    }

    pub async fn dump(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.db
            .send(
                meta::Dump {
                    tx,
                    span: tracing::Span::current(),
                }
                .into(),
            )
            .await
            .anyerr()?;
        rx.await.anyerr()??;
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use core::panic;
    use std::collections::{HashMap, HashSet};

    use bao_tree::{io::round_up_to_chunks_groups, ChunkRanges};
    use n0_future::{stream, Stream, StreamExt};
    use testresult::TestResult;
    use walkdir::WalkDir;

    use super::*;
    use crate::{
        api::blobs::Bitfield,
        store::{
            util::{read_checksummed, tests::create_n0_bao, SliceInfoExt, Tag},
            IROH_BLOCK_SIZE,
        },
    };

    /// Interesting sizes for testing.
    pub const INTERESTING_SIZES: [usize; 8] = [
        0,               // annoying corner case - always present, handled by the api
        1,               // less than 1 chunk, data inline, outboard not needed
        1024,            // exactly 1 chunk, data inline, outboard not needed
        1024 * 16 - 1,   // less than 1 chunk group, data inline, outboard not needed
        1024 * 16,       // exactly 1 chunk group, data inline, outboard not needed
        1024 * 16 + 1,   // data file, outboard inline (just 1 hash pair)
        1024 * 1024,     // data file, outboard inline (many hash pairs)
        1024 * 1024 * 8, // data file, outboard file
    ];

    pub fn round_up_request(size: u64, ranges: &ChunkRanges) -> ChunkRanges {
        let last_chunk = ChunkNum::chunks(size);
        let data_range = ChunkRanges::from(..last_chunk);
        let ranges = if !data_range.intersects(ranges) && !ranges.is_empty() {
            if last_chunk == 0 {
                ChunkRanges::all()
            } else {
                ChunkRanges::from(last_chunk - 1..)
            }
        } else {
            ranges.clone()
        };
        round_up_to_chunks_groups(ranges, IROH_BLOCK_SIZE)
    }

    fn create_n0_bao_full(
        data: &[u8],
        ranges: &ChunkRanges,
    ) -> n0_error::Result<(Hash, ChunkRanges, Vec<u8>)> {
        let ranges = round_up_request(data.len() as u64, ranges);
        let (hash, encoded) = create_n0_bao(data, &ranges)?;
        Ok((hash, ranges, encoded))
    }

    #[tokio::test]
    // #[traced_test]
    async fn test_observe() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let options = Options::new(&db_dir);
        let store = FsStore::load_with_opts(db_dir.join("blobs.db"), options).await?;
        let sizes = INTERESTING_SIZES;
        for size in sizes {
            let data = test_data(size);
            let ranges = ChunkRanges::all();
            let (hash, bao) = create_n0_bao(&data, &ranges)?;
            let obs = store.observe(hash);
            let task = n0_future::task::spawn(async move {
                obs.await_completion().await?;
                api::Result::Ok(())
            });
            store.import_bao_bytes(hash, ranges, bao).await?;
            task.await??;
        }
        Ok(())
    }

    /// Generate test data for size n.
    ///
    /// We don't really care about the content, since we assume blake3 works.
    /// The only thing it should not be is all zeros, since that is what you
    /// will get for a gap.
    pub fn test_data(n: usize) -> Bytes {
        let mut res = Vec::with_capacity(n);
        // Using uppercase A-Z (65-90), 26 possible characters
        for i in 0..n {
            // Change character every 1024 bytes
            let block_num = i / 1024;
            // Map to uppercase A-Z range (65-90)
            let ascii_val = 65 + (block_num % 26) as u8;
            res.push(ascii_val);
        }
        Bytes::from(res)
    }

    // import data via import_bytes, check that we can observe it and that it is complete
    #[tokio::test]
    async fn test_import_byte_stream() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let store = FsStore::load(db_dir).await?;
        for size in INTERESTING_SIZES {
            let expected = test_data(size);
            let expected_hash = Hash::new(&expected);
            let stream = bytes_to_stream(expected.clone(), 1023);
            let obs = store.observe(expected_hash);
            let tt = store.add_stream(stream).await.temp_tag().await?;
            assert_eq!(expected_hash, tt.hash());
            // we must at some point see completion, otherwise the test will hang
            obs.await_completion().await?;
            let actual = store.get_bytes(expected_hash).await?;
            // check that the data is there
            assert_eq!(&expected, &actual);
        }
        Ok(())
    }

    // import data via import_bytes, check that we can observe it and that it is complete
    #[tokio::test]
    async fn test_import_bytes_simple() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let store = FsStore::load(&db_dir).await?;
        let sizes = INTERESTING_SIZES;
        trace!("{}", Options::new(&db_dir).is_inlined_data(16385));
        for size in sizes {
            let expected = test_data(size);
            let expected_hash = Hash::new(&expected);
            let obs = store.observe(expected_hash);
            let tt = store.add_bytes(expected.clone()).await?;
            assert_eq!(expected_hash, tt.hash);
            // we must at some point see completion, otherwise the test will hang
            obs.await_completion().await?;
            let actual = store.get_bytes(expected_hash).await?;
            // check that the data is there
            assert_eq!(&expected, &actual);
        }
        store.shutdown().await?;
        dump_dir_full(db_dir)?;
        Ok(())
    }

    // import data via import_bytes, check that we can observe it and that it is complete
    #[tokio::test]
    #[ignore = "flaky. I need a reliable way to keep the handle alive"]
    async fn test_roundtrip_bytes_small() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let store = FsStore::load(db_dir).await?;
        for size in INTERESTING_SIZES
            .into_iter()
            .filter(|x| *x != 0 && *x <= IROH_BLOCK_SIZE.bytes())
        {
            let expected = test_data(size);
            let expected_hash = Hash::new(&expected);
            let obs = store.observe(expected_hash);
            let tt = store.add_bytes(expected.clone()).await?;
            assert_eq!(expected_hash, tt.hash);
            let actual = store.get_bytes(expected_hash).await?;
            // check that the data is there
            assert_eq!(&expected, &actual);
            assert_eq!(
                &expected.addr(),
                &actual.addr(),
                "address mismatch for size {size}"
            );
            // we must at some point see completion, otherwise the test will hang
            // keep the handle alive by observing until the end, otherwise the handle
            // will change and the bytes won't be the same instance anymore
            obs.await_completion().await?;
        }
        store.shutdown().await?;
        Ok(())
    }

    // import data via import_bytes, check that we can observe it and that it is complete
    #[tokio::test]
    async fn test_import_path() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let store = FsStore::load(db_dir).await?;
        for size in INTERESTING_SIZES {
            let expected = test_data(size);
            let expected_hash = Hash::new(&expected);
            let path = testdir.path().join(format!("in-{size}"));
            fs::write(&path, &expected)?;
            let obs = store.observe(expected_hash);
            let tt = store.add_path(&path).await?;
            assert_eq!(expected_hash, tt.hash);
            // we must at some point see completion, otherwise the test will hang
            obs.await_completion().await?;
            let actual = store.get_bytes(expected_hash).await?;
            // check that the data is there
            assert_eq!(&expected, &actual, "size={size}");
        }
        dump_dir_full(testdir.path())?;
        Ok(())
    }

    // import data via import_bytes, check that we can observe it and that it is complete
    #[tokio::test]
    async fn test_export_path() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let store = FsStore::load(db_dir).await?;
        for size in INTERESTING_SIZES {
            let expected = test_data(size);
            let expected_hash = Hash::new(&expected);
            let tt = store.add_bytes(expected.clone()).await?;
            assert_eq!(expected_hash, tt.hash);
            let out_path = testdir.path().join(format!("out-{size}"));
            store.export(expected_hash, &out_path).await?;
            let actual = fs::read(&out_path)?;
            assert_eq!(expected, actual);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bao_ranges() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        {
            let store = FsStore::load(&db_dir).await?;
            let data = test_data(100000);
            let ranges = ChunkRanges::chunks(16..32);
            let (hash, bao) = create_n0_bao(&data, &ranges)?;
            store
                .import_bao_bytes(hash, ranges.clone(), bao.clone())
                .await?;
            let bitfield = store.observe(hash).await?;
            assert_eq!(bitfield.ranges, ranges);
            assert_eq!(bitfield.size(), data.len() as u64);
            let export = store.export_bao(hash, ranges).bao_to_vec().await?;
            assert_eq!(export, bao);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bao_minimal() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = [1];
        let db_dir = testdir.path().join("db");
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let data = vec![0u8; size];
                let (hash, encoded) = create_n0_bao(&data, &ChunkRanges::all())?;
                let data = Bytes::from(encoded);
                store
                    .import_bao_bytes(hash, ChunkRanges::all(), data)
                    .await?;
            }
            store.shutdown().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bao_simple() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = [1048576];
        let db_dir = testdir.path().join("db");
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let data = vec![0u8; size];
                let (hash, encoded) = create_n0_bao(&data, &ChunkRanges::all())?;
                let data = Bytes::from(encoded);
                trace!("importing size={}", size);
                store
                    .import_bao_bytes(hash, ChunkRanges::all(), data)
                    .await?;
            }
            store.shutdown().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bao_persistence_full() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = INTERESTING_SIZES;
        let db_dir = testdir.path().join("db");
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let data = vec![0u8; size];
                let (hash, encoded) = create_n0_bao(&data, &ChunkRanges::all())?;
                let data = Bytes::from(encoded);
                store
                    .import_bao_bytes(hash, ChunkRanges::all(), data)
                    .await?;
            }
            store.shutdown().await?;
        }
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let expected = vec![0u8; size];
                let hash = Hash::new(&expected);
                let actual = store
                    .export_bao(hash, ChunkRanges::all())
                    .data_to_vec()
                    .await?;
                assert_eq!(&expected, &actual);
            }
            store.shutdown().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bao_persistence_just_size() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = INTERESTING_SIZES;
        let db_dir = testdir.path().join("db");
        let just_size = ChunkRanges::last_chunk();
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let data = test_data(size);
                let (hash, ranges, encoded) = create_n0_bao_full(&data, &just_size)?;
                let data = Bytes::from(encoded);
                if let Err(cause) = store.import_bao_bytes(hash, ranges, data).await {
                    panic!("failed to import size={size}: {cause}");
                }
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        {
            let store = FsStore::load(&db_dir).await?;
            store.dump().await?;
            for size in sizes {
                let data = test_data(size);
                let (hash, ranges, expected) = create_n0_bao_full(&data, &just_size)?;
                let actual = match store.export_bao(hash, ranges).bao_to_vec().await {
                    Ok(actual) => actual,
                    Err(cause) => panic!("failed to export size={size}: {cause}"),
                };
                assert_eq!(&expected, &actual);
            }
            store.shutdown().await?;
        }
        dump_dir_full(testdir.path())?;
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bao_persistence_two_stages() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = INTERESTING_SIZES;
        let db_dir = testdir.path().join("db");
        let just_size = ChunkRanges::last_chunk();
        // stage 1, import just the last full chunk group to get a validated size
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let data = test_data(size);
                let (hash, ranges, encoded) = create_n0_bao_full(&data, &just_size)?;
                let data = Bytes::from(encoded);
                if let Err(cause) = store.import_bao_bytes(hash, ranges, data).await {
                    panic!("failed to import size={size}: {cause}");
                }
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        dump_dir_full(testdir.path())?;
        // stage 2, import the rest
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let remaining = ChunkRanges::all() - round_up_request(size as u64, &just_size);
                if remaining.is_empty() {
                    continue;
                }
                let data = test_data(size);
                let (hash, ranges, encoded) = create_n0_bao_full(&data, &remaining)?;
                let data = Bytes::from(encoded);
                if let Err(cause) = store.import_bao_bytes(hash, ranges, data).await {
                    panic!("failed to import size={size}: {cause}");
                }
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        // check if the data is complete
        {
            let store = FsStore::load(&db_dir).await?;
            store.dump().await?;
            for size in sizes {
                let data = test_data(size);
                let (hash, ranges, expected) = create_n0_bao_full(&data, &ChunkRanges::all())?;
                let actual = match store.export_bao(hash, ranges).bao_to_vec().await {
                    Ok(actual) => actual,
                    Err(cause) => panic!("failed to export size={size}: {cause}"),
                };
                assert_eq!(&expected, &actual);
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        dump_dir_full(testdir.path())?;
        Ok(())
    }

    fn just_size() -> ChunkRanges {
        ChunkRanges::last_chunk()
    }

    #[tokio::test]
    async fn test_import_bao_persistence_observe() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = INTERESTING_SIZES;
        let db_dir = testdir.path().join("db");
        let just_size = just_size();
        // stage 1, import just the last full chunk group to get a validated size
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let data = test_data(size);
                let (hash, ranges, encoded) = create_n0_bao_full(&data, &just_size)?;
                let data = Bytes::from(encoded);
                if let Err(cause) = store.import_bao_bytes(hash, ranges, data).await {
                    panic!("failed to import size={size}: {cause}");
                }
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        dump_dir_full(testdir.path())?;
        // stage 2, import the rest
        {
            let store = FsStore::load(&db_dir).await?;
            for size in sizes {
                let expected_ranges = round_up_request(size as u64, &just_size);
                let data = test_data(size);
                let hash = Hash::new(&data);
                let bitfield = store.observe(hash).await?;
                assert_eq!(bitfield.ranges, expected_ranges);
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bao_persistence_recover() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = INTERESTING_SIZES;
        let db_dir = testdir.path().join("db");
        let options = Options::new(&db_dir);
        let just_size = just_size();
        // stage 1, import just the last full chunk group to get a validated size
        {
            let store = FsStore::load_with_opts(db_dir.join("blobs.db"), options.clone()).await?;
            for size in sizes {
                let data = test_data(size);
                let (hash, ranges, encoded) = create_n0_bao_full(&data, &just_size)?;
                let data = Bytes::from(encoded);
                if let Err(cause) = store.import_bao_bytes(hash, ranges, data).await {
                    panic!("failed to import size={size}: {cause}");
                }
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        delete_rec(testdir.path(), "bitfield")?;
        dump_dir_full(testdir.path())?;
        // stage 2, import the rest
        {
            let store = FsStore::load_with_opts(db_dir.join("blobs.db"), options.clone()).await?;
            for size in sizes {
                let expected_ranges = round_up_request(size as u64, &just_size);
                let data = test_data(size);
                let hash = Hash::new(&data);
                let bitfield = store.observe(hash).await?;
                assert_eq!(bitfield.ranges, expected_ranges, "size={size}");
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_import_bytes_persistence_full() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let sizes = INTERESTING_SIZES;
        let db_dir = testdir.path().join("db");
        {
            let store = FsStore::load(&db_dir).await?;
            let mut tts = Vec::new();
            for size in sizes {
                let data = test_data(size);
                let data = data;
                tts.push(store.add_bytes(data.clone()).await?);
            }
            store.dump().await?;
            store.shutdown().await?;
        }
        {
            let store = FsStore::load(&db_dir).await?;
            store.dump().await?;
            for size in sizes {
                let expected = test_data(size);
                let hash = Hash::new(&expected);
                let Ok(actual) = store
                    .export_bao(hash, ChunkRanges::all())
                    .data_to_vec()
                    .await
                else {
                    panic!("failed to export size={size}");
                };
                assert_eq!(&expected, &actual, "size={size}");
            }
            store.shutdown().await?;
        }
        Ok(())
    }

    async fn test_batch(store: &Store) -> TestResult<()> {
        let batch = store.blobs().batch().await?;
        let tt1 = batch.temp_tag(Hash::new("foo")).await?;
        let tt2 = batch.add_slice("boo").await?;
        let tts = store
            .tags()
            .list_temp_tags()
            .await?
            .collect::<HashSet<_>>()
            .await;
        assert!(tts.contains(&tt1.hash_and_format()));
        assert!(tts.contains(&tt2.hash_and_format()));
        drop(batch);
        store.sync_db().await?;
        store.wait_idle().await?;
        let tts = store
            .tags()
            .list_temp_tags()
            .await?
            .collect::<HashSet<_>>()
            .await;
        // temp tag went out of scope, so it does not work anymore
        assert!(!tts.contains(&tt1.hash_and_format()));
        assert!(!tts.contains(&tt2.hash_and_format()));
        drop(tt1);
        drop(tt2);
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_fs() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let store = FsStore::load(db_dir).await?;
        test_batch(&store).await
    }

    #[tokio::test]
    async fn smoke() -> TestResult<()> {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_dir = testdir.path().join("db");
        let store = FsStore::load(db_dir).await?;
        let haf = HashAndFormat::raw(Hash::from([0u8; 32]));
        store.tags().set(Tag::from("test"), haf).await?;
        store.tags().set(Tag::from("boo"), haf).await?;
        store.tags().set(Tag::from("bar"), haf).await?;
        let sizes = INTERESTING_SIZES;
        let mut hashes = Vec::new();
        let mut data_by_hash = HashMap::new();
        let mut bao_by_hash = HashMap::new();
        for size in sizes {
            let data = vec![0u8; size];
            let data = Bytes::from(data);
            let tt = store.add_bytes(data.clone()).temp_tag().await?;
            data_by_hash.insert(tt.hash(), data);
            hashes.push(tt);
        }
        store.sync_db().await?;
        for tt in &hashes {
            let hash = tt.hash();
            let path = testdir.path().join(format!("{hash}.txt"));
            store.export(hash, path).await?;
        }
        for tt in &hashes {
            let hash = tt.hash();
            let data = store
                .export_bao(hash, ChunkRanges::all())
                .data_to_vec()
                .await
                .unwrap();
            assert_eq!(data, data_by_hash[&hash].to_vec());
            let bao = store
                .export_bao(hash, ChunkRanges::all())
                .bao_to_vec()
                .await
                .unwrap();
            bao_by_hash.insert(hash, bao);
        }
        store.dump().await?;

        for size in sizes {
            let data = test_data(size);
            let ranges = ChunkRanges::all();
            let (hash, bao) = create_n0_bao(&data, &ranges)?;
            store.import_bao_bytes(hash, ranges, bao).await?;
        }

        for (_hash, _bao_tree) in bao_by_hash {
            // let mut reader = Cursor::new(bao_tree);
            // let size = reader.read_u64_le().await?;
            // let tree = BaoTree::new(size, IROH_BLOCK_SIZE);
            // let ranges = ChunkRanges::all();
            // let mut decoder = DecodeResponseIter::new(hash, tree, reader, &ranges);
            // while let Some(item) = decoder.next() {
            //     let item = item?;
            // }
            // store.import_bao_bytes(hash, ChunkRanges::all(), bao_tree.into()).await?;
        }
        Ok(())
    }

    pub fn delete_rec(root_dir: impl AsRef<Path>, extension: &str) -> Result<(), std::io::Error> {
        // Remove leading dot if present, so we have just the extension
        let ext = extension.trim_start_matches('.').to_lowercase();

        for entry in WalkDir::new(root_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_file() {
                if let Some(file_ext) = path.extension() {
                    if file_ext.to_string_lossy().to_lowercase() == ext {
                        fs::remove_file(path)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn dump_dir(path: impl AsRef<Path>) -> io::Result<()> {
        let mut entries: Vec<_> = WalkDir::new(&path)
            .into_iter()
            .filter_map(Result::ok) // Skip errors
            .collect();

        // Sort by path (name at each depth)
        entries.sort_by(|a, b| a.path().cmp(b.path()));

        for entry in entries {
            let depth = entry.depth();
            let indent = "  ".repeat(depth); // Two spaces per level
            let name = entry.file_name().to_string_lossy();
            let size = entry.metadata()?.len(); // Size in bytes

            if entry.file_type().is_file() {
                println!("{indent}{name} ({size} bytes)");
            } else if entry.file_type().is_dir() {
                println!("{indent}{name}/");
            }
        }
        Ok(())
    }

    pub fn dump_dir_full(path: impl AsRef<Path>) -> io::Result<()> {
        let mut entries: Vec<_> = WalkDir::new(&path)
            .into_iter()
            .filter_map(Result::ok) // Skip errors
            .collect();

        // Sort by path (name at each depth)
        entries.sort_by(|a, b| a.path().cmp(b.path()));

        for entry in entries {
            let depth = entry.depth();
            let indent = "  ".repeat(depth);
            let name = entry.file_name().to_string_lossy();

            if entry.file_type().is_dir() {
                println!("{indent}{name}/");
            } else if entry.file_type().is_file() {
                let size = entry.metadata()?.len();
                println!("{indent}{name} ({size} bytes)");

                // Dump depending on file type
                let path = entry.path();
                if name.ends_with(".data") {
                    print!("{indent}  ");
                    dump_file(path, 1024 * 16)?;
                } else if name.ends_with(".obao4") {
                    print!("{indent}  ");
                    dump_file(path, 64)?;
                } else if name.ends_with(".sizes4") {
                    print!("{indent}  ");
                    dump_file(path, 8)?;
                } else if name.ends_with(".bitfield") {
                    match read_checksummed::<Bitfield>(path) {
                        Ok(bitfield) => {
                            println!("{indent}  bitfield: {bitfield:?}");
                        }
                        Err(cause) => {
                            println!("{indent}  bitfield: error: {cause}");
                        }
                    }
                } else {
                    continue; // Skip content dump for other files
                };
            }
        }
        Ok(())
    }

    pub fn dump_file<P: AsRef<Path>>(path: P, chunk_size: u64) -> io::Result<()> {
        let bits = file_bits(path, chunk_size)?;
        println!("{}", print_bitfield_ansi(bits));
        Ok(())
    }

    pub fn file_bits(path: impl AsRef<Path>, chunk_size: u64) -> io::Result<Vec<bool>> {
        let file = fs::File::open(&path)?;
        let file_size = file.metadata()?.len();
        let mut buffer = vec![0u8; chunk_size as usize];
        let mut bits = Vec::new();

        let mut offset = 0u64;
        while offset < file_size {
            let remaining = file_size - offset;
            let current_chunk_size = chunk_size.min(remaining);

            let chunk = &mut buffer[..current_chunk_size as usize];
            file.read_exact_at(offset, chunk)?;

            let has_non_zero = chunk.iter().any(|&byte| byte != 0);
            bits.push(has_non_zero);

            offset += current_chunk_size;
        }

        Ok(bits)
    }

    #[allow(dead_code)]
    fn print_bitfield(bits: impl IntoIterator<Item = bool>) -> String {
        bits.into_iter()
            .map(|bit| if bit { '#' } else { '_' })
            .collect()
    }

    fn print_bitfield_ansi(bits: impl IntoIterator<Item = bool>) -> String {
        let mut result = String::new();
        let mut iter = bits.into_iter();

        while let Some(b1) = iter.next() {
            let b2 = iter.next();

            // ANSI color codes
            let white_fg = "\x1b[97m"; // bright white foreground
            let reset = "\x1b[0m"; // reset all attributes
            let gray_bg = "\x1b[100m"; // bright black (gray) background
            let black_bg = "\x1b[40m"; // black background

            let colored_char = match (b1, b2) {
                (true, Some(true)) => format!("{}{}{}", white_fg, '█', reset), // 11 - solid white on default background
                (true, Some(false)) => format!("{}{}{}{}", gray_bg, white_fg, '▌', reset), // 10 - left half white on gray background
                (false, Some(true)) => format!("{}{}{}{}", gray_bg, white_fg, '▐', reset), // 01 - right half white on gray background
                (false, Some(false)) => format!("{}{}{}{}", gray_bg, white_fg, ' ', reset), // 00 - space with gray background
                (true, None) => format!("{}{}{}{}", black_bg, white_fg, '▌', reset), // 1 (pad 0) - left half white on black background
                (false, None) => format!("{}{}{}{}", black_bg, white_fg, ' ', reset), // 0 (pad 0) - space with black background
            };

            result.push_str(&colored_char);
        }

        // Ensure we end with a reset code to prevent color bleeding
        result.push_str("\x1b[0m");
        result
    }

    fn bytes_to_stream(
        bytes: Bytes,
        chunk_size: usize,
    ) -> impl Stream<Item = io::Result<Bytes>> + 'static {
        assert!(chunk_size > 0, "Chunk size must be greater than 0");
        stream::unfold((bytes, 0), move |(bytes, offset)| async move {
            if offset >= bytes.len() {
                None
            } else {
                let chunk_len = chunk_size.min(bytes.len() - offset);
                let chunk = bytes.slice(offset..offset + chunk_len);
                Some((Ok(chunk), (bytes, offset + chunk_len)))
            }
        })
    }
}
