//! The entire logic for importing data from three different sources: bytes, byte stream, and file path.
//!
//! For bytes, the size is known in advance. But they might still have to be persisted to disk if they are too large.
//! E.g. you add a 1GB bytes to the store, you still want this to end up on disk.
//!
//! For byte streams, the size is not known in advance.
//!
//! For file paths, the size is known in advance. This is also the only case where we might reference the data instead
//! of copying it.
//!
//! The various ..._task fns return an `Option<ImportEntry>`. If import fails for whatever reason, the error goes
//! to the requester, and the task returns None.
use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Seek, Write},
    path::PathBuf,
    sync::Arc,
};

use bao_tree::{
    io::outboard::{PreOrderMemOutboard, PreOrderOutboard},
    BaoTree, ChunkNum,
};
use bytes::Bytes;
use genawaiter::sync::Gen;
use irpc::{
    channel::{mpsc, none::NoReceiver},
    Channels, WithChannels,
};
use n0_future::{stream, Stream, StreamExt};
use ref_cast::RefCast;
use smallvec::SmallVec;
use tracing::{instrument, trace};

use super::{meta::raw_outboard_size, options::Options, TaskContext};
use crate::{
    api::{
        blobs::{AddProgressItem, ImportMode},
        proto::{
            HashSpecific, ImportByteStreamMsg, ImportByteStreamRequest, ImportByteStreamUpdate,
            ImportBytesMsg, ImportBytesRequest, ImportPathMsg, ImportPathRequest, Request, Scope,
        },
    },
    store::{
        fs::reflink_or_copy_with_progress,
        util::{MemOrFile, DD},
        IROH_BLOCK_SIZE,
    },
    util::{outboard_with_progress::init_outboard, sink::Sink},
    BlobFormat, Hash,
};

/// An import source.
///
/// It must provide a way to read the data synchronously, as well as the size
/// and the file location.
///
/// This serves as an intermediate result between copying and outboard computation.
pub enum ImportSource {
    TempFile(PathBuf, File, u64),
    External(PathBuf, File, u64),
    Memory(Bytes),
}

impl std::fmt::Debug for ImportSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TempFile(path, _, size) => {
                f.debug_tuple("TempFile").field(path).field(size).finish()
            }
            Self::External(path, _, size) => {
                f.debug_tuple("External").field(path).field(size).finish()
            }
            Self::Memory(data) => f.debug_tuple("Memory").field(&data.len()).finish(),
        }
    }
}

impl ImportSource {
    pub fn fmt_short(&self) -> String {
        match self {
            Self::TempFile(path, _, _) => format!("TempFile({})", path.display()),
            Self::External(path, _, _) => format!("External({})", path.display()),
            Self::Memory(data) => format!("Memory({})", data.len()),
        }
    }

    fn is_mem(&self) -> bool {
        matches!(self, Self::Memory(_))
    }

    /// A reader for the import source.
    fn read(&self) -> MemOrFile<std::io::Cursor<&[u8]>, &File> {
        match self {
            Self::TempFile(_, file, _) => MemOrFile::File(file),
            Self::External(_, file, _) => MemOrFile::File(file),
            Self::Memory(data) => MemOrFile::Mem(std::io::Cursor::new(data.as_ref())),
        }
    }

    /// The size of the import source.
    fn size(&self) -> u64 {
        match self {
            Self::TempFile(_, _, size) => *size,
            Self::External(_, _, size) => *size,
            Self::Memory(data) => data.len() as u64,
        }
    }
}

/// An import entry.
///
/// This is the final result of an import operation. It gets passed to the store
/// for integration.
///
/// The store can assume that the outboard, if on disk, is in a location where
/// it can be moved to the final location (basically it needs to be on the same device).
pub struct ImportEntry {
    pub hash: Hash,
    pub format: BlobFormat,
    pub scope: Scope,
    pub source: ImportSource,
    pub outboard: MemOrFile<Bytes, PathBuf>,
}

impl std::fmt::Debug for ImportEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImportEntry")
            .field("hash", &self.hash)
            .field("format", &self.format)
            .field("scope", &self.scope)
            .field("source", &DD(self.source.fmt_short()))
            .field("outboard", &DD(self.outboard.fmt_short()))
            .finish()
    }
}

impl Channels<Request> for ImportEntry {
    type Tx = mpsc::Sender<AddProgressItem>;
    type Rx = NoReceiver;
}

pub type ImportEntryMsg = WithChannels<ImportEntry, Request>;

impl HashSpecific for ImportEntryMsg {
    fn hash(&self) -> Hash {
        self.hash
    }
}

impl ImportEntry {
    /// True if both data and outboard are in memory.
    pub fn is_mem(&self) -> bool {
        self.source.is_mem() && self.outboard.is_mem()
    }
}

/// Start a task to import from a [`Bytes`] in memory.
#[instrument(skip_all, fields(data = cmd.data.len()))]
pub async fn import_bytes(cmd: ImportBytesMsg, ctx: Arc<TaskContext>) {
    let size = cmd.data.len() as u64;
    if ctx.options.is_inlined_all(size) {
        import_bytes_tiny_outer(cmd, ctx).await;
    } else {
        let request = ImportByteStreamRequest {
            format: cmd.format,
            scope: cmd.scope,
        };
        let stream = stream::iter(Some(Ok(cmd.data.clone())));
        import_byte_stream_mid(request, cmd.tx, cmd.span, stream, ctx).await;
    }
}

async fn import_bytes_tiny_outer(mut cmd: ImportBytesMsg, ctx: Arc<TaskContext>) {
    match import_bytes_tiny_impl(cmd.inner, &mut cmd.tx).await {
        Ok(entry) => {
            let entry = ImportEntryMsg {
                inner: entry,
                tx: cmd.tx,
                rx: cmd.rx,
                span: cmd.span,
            };
            ctx.internal_cmd_tx.send(entry.into()).await.ok();
        }
        Err(cause) => {
            cmd.tx.send(cause.into()).await.ok();
        }
    }
}

async fn import_bytes_tiny_impl(
    cmd: ImportBytesRequest,
    tx: &mut mpsc::Sender<AddProgressItem>,
) -> io::Result<ImportEntry> {
    let size = cmd.data.len() as u64;
    // send the required progress events
    // AddProgressItem::Done will be sent when finishing the import!
    tx.send(AddProgressItem::Size(size)).await?;
    tx.send(AddProgressItem::CopyDone).await?;
    Ok(if raw_outboard_size(size) == 0 {
        // the thing is so small that it does not even need an outboard
        ImportEntry {
            hash: Hash::new(&cmd.data),
            format: cmd.format,
            scope: cmd.scope,
            source: ImportSource::Memory(cmd.data),
            outboard: MemOrFile::empty(),
        }
    } else {
        // we still know that computing the outboard will be super fast
        let outboard = PreOrderMemOutboard::create(&cmd.data, IROH_BLOCK_SIZE);
        ImportEntry {
            hash: outboard.root.into(),
            format: cmd.format,
            scope: cmd.scope,
            source: ImportSource::Memory(cmd.data),
            outboard: MemOrFile::Mem(Bytes::from(outboard.data)),
        }
    })
}

#[instrument(skip_all)]
pub async fn import_byte_stream(cmd: ImportByteStreamMsg, ctx: Arc<TaskContext>) {
    let stream = into_stream(cmd.rx);
    import_byte_stream_mid(cmd.inner, cmd.tx, cmd.span, stream, ctx).await
}

fn into_stream(
    mut rx: mpsc::Receiver<ImportByteStreamUpdate>,
) -> impl Stream<Item = io::Result<Bytes>> {
    Gen::new(|co| async move {
        loop {
            match rx.recv().await {
                Ok(Some(ImportByteStreamUpdate::Bytes(data))) => {
                    co.yield_(Ok(data)).await;
                }
                Ok(Some(ImportByteStreamUpdate::Done)) => {
                    break;
                }
                Ok(None) => {
                    co.yield_(Err(io::ErrorKind::UnexpectedEof.into())).await;
                    break;
                }
                Err(e) => {
                    co.yield_(Err(e.into())).await;
                    break;
                }
            }
        }
    })
}

async fn import_byte_stream_mid(
    request: ImportByteStreamRequest,
    mut tx: mpsc::Sender<AddProgressItem>,
    span: tracing::Span,
    stream: impl Stream<Item = io::Result<Bytes>> + Unpin,
    ctx: Arc<TaskContext>,
) {
    match import_byte_stream_impl(request, &mut tx, stream, ctx.options.clone()).await {
        Ok(entry) => {
            let entry = ImportEntryMsg {
                inner: entry,
                tx,
                rx: NoReceiver,
                span,
            };
            ctx.internal_cmd_tx.send(entry.into()).await.ok();
        }
        Err(cause) => {
            tx.send(cause.into()).await.ok();
        }
    }
}

async fn import_byte_stream_impl(
    cmd: ImportByteStreamRequest,
    tx: &mut mpsc::Sender<AddProgressItem>,
    stream: impl Stream<Item = io::Result<Bytes>> + Unpin,
    options: Arc<Options>,
) -> io::Result<ImportEntry> {
    let ImportByteStreamRequest { format, scope } = cmd;
    let import_source = get_import_source(stream, tx, &options).await?;
    tx.send(AddProgressItem::Size(import_source.size())).await?;
    tx.send(AddProgressItem::CopyDone).await?;
    compute_outboard(import_source, format, scope, options, tx).await
}

async fn get_import_source(
    stream: impl Stream<Item = io::Result<Bytes>> + Unpin,
    tx: &mut mpsc::Sender<AddProgressItem>,
    options: &Options,
) -> io::Result<ImportSource> {
    let mut stream = stream.fuse();
    let mut peek = SmallVec::<[_; 2]>::new();
    let Some(first) = stream.next().await.transpose()? else {
        return Ok(ImportSource::Memory(Bytes::new()));
    };
    match stream.next().await.transpose()? {
        Some(second) => {
            peek.push(Ok(first));
            peek.push(Ok(second));
        }
        None => {
            let size = first.len() as u64;
            if options.is_inlined_data(size) {
                return Ok(ImportSource::Memory(first));
            }
            peek.push(Ok(first));
        }
    };
    // todo: if both first and second are giant, we might want to write them to disk immediately
    let mut stream = stream::iter(peek).chain(stream);
    let mut size = 0;
    let mut data = Vec::new();
    let mut disk = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        size += chunk.len() as u64;
        if size > options.inline.max_data_inlined {
            let temp_path = options.path.temp_file_name();
            trace!("writing to temp file: {:?}", temp_path);
            let mut file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temp_path)?;
            file.write_all(&data)?;
            file.write_all(&chunk)?;
            data.clear();
            disk = Some((file, temp_path));
            break;
        } else {
            data.extend_from_slice(&chunk);
        }
        // todo: don't send progress for every chunk if the chunks are small?
        tx.try_send(AddProgressItem::CopyProgress(size)).await?;
    }
    Ok(if let Some((mut file, temp_path)) = disk {
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            size += chunk.len() as u64;
            tx.send(AddProgressItem::CopyProgress(size)).await?;
        }
        ImportSource::TempFile(temp_path, file, size)
    } else {
        ImportSource::Memory(data.into())
    })
}

#[derive(ref_cast::RefCast)]
#[repr(transparent)]
struct OutboardProgress(mpsc::Sender<AddProgressItem>);

impl Sink<ChunkNum> for OutboardProgress {
    type Error = irpc::channel::SendError;

    async fn send(&mut self, offset: ChunkNum) -> std::result::Result<(), Self::Error> {
        // if offset.0 % 1024 != 0 {
        //     return Ok(());
        // }
        self.0
            .try_send(AddProgressItem::OutboardProgress(offset.to_bytes()))
            .await?;
        Ok(())
    }
}

async fn compute_outboard(
    source: ImportSource,
    format: BlobFormat,
    scope: Scope,
    options: Arc<Options>,
    tx: &mut mpsc::Sender<AddProgressItem>,
) -> io::Result<ImportEntry> {
    let size = source.size();
    let tree = BaoTree::new(size, IROH_BLOCK_SIZE);
    let root = bao_tree::blake3::Hash::from_bytes([0; 32]);
    let outboard_size = raw_outboard_size(size);
    let send_progress = OutboardProgress::ref_cast_mut(tx);
    let mut data = source.read();
    data.rewind()?;
    let (hash, outboard) = if outboard_size > options.inline.max_outboard_inlined {
        // outboard will eventually be stored as a file, so compute it directly to a file
        // we don't know the hash yet, so we need to create a temp file
        let outboard_path = options.path.temp_file_name();
        trace!("Creating outboard file in {}", outboard_path.display());
        // we don't need to read from this file!
        let mut outboard_file = File::create(&outboard_path)?;
        let mut outboard = PreOrderOutboard {
            tree,
            root,
            data: &mut outboard_file,
        };
        init_outboard(data, &mut outboard, send_progress).await??;
        (outboard.root, MemOrFile::File(outboard_path))
    } else {
        // outboard will be stored in memory, so compute it to a memory buffer
        trace!("Creating outboard in memory");
        let mut outboard_file: Vec<u8> = Vec::new();
        let mut outboard = PreOrderOutboard {
            tree,
            root,
            data: &mut outboard_file,
        };
        init_outboard(data, &mut outboard, send_progress).await??;
        (outboard.root, MemOrFile::Mem(Bytes::from(outboard_file)))
    };
    Ok(ImportEntry {
        hash: hash.into(),
        format,
        scope,
        source,
        outboard,
    })
}

#[instrument(skip_all, fields(path = %cmd.path.display()))]
pub async fn import_path(mut cmd: ImportPathMsg, context: Arc<TaskContext>) {
    match import_path_impl(cmd.inner, &mut cmd.tx, context.options.clone()).await {
        Ok(inner) => {
            let res = ImportEntryMsg {
                inner,
                tx: cmd.tx,
                rx: cmd.rx,
                span: cmd.span,
            };
            context.internal_cmd_tx.send(res.into()).await.ok();
        }
        Err(cause) => {
            cmd.tx.send(cause.into()).await.ok();
        }
    }
}

async fn import_path_impl(
    cmd: ImportPathRequest,
    tx: &mut mpsc::Sender<AddProgressItem>,
    options: Arc<Options>,
) -> io::Result<ImportEntry> {
    let ImportPathRequest {
        path,
        mode,
        format,
        scope: batch,
    } = cmd;
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path must be absolute",
        ));
    }
    if !path.is_file() && !path.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a file or symlink",
        ));
    }

    let size = path.metadata()?.len();
    tx.send(AddProgressItem::Size(size)).await?;
    let import_source = if size <= options.inline.max_data_inlined {
        let data = std::fs::read(path)?;
        tx.send(AddProgressItem::CopyDone).await?;
        ImportSource::Memory(data.into())
    } else if mode == ImportMode::TryReference {
        // reference where it is. We are going to need the file handle to
        // compute the outboard, so open it here. If this fails, the import
        // can't proceed.
        let file = OpenOptions::new().read(true).open(&path)?;
        ImportSource::External(path, file, size)
    } else {
        let temp_path = options.path.temp_file_name();
        // todo: if reflink works, we don't need progress.
        // But if it does not, it might take a while and we won't get progress.
        let res = reflink_or_copy_with_progress(&path, &temp_path, size, tx).await?;
        trace!(
            "imported {} to {}, {res:?}",
            path.display(),
            temp_path.display()
        );
        // copy from path to temp_path
        let file = OpenOptions::new().read(true).open(&temp_path)?;
        tx.send(AddProgressItem::CopyDone).await?;
        ImportSource::TempFile(temp_path, file, size)
    };
    compute_outboard(import_source, format, batch, options, tx).await
}

#[cfg(test)]
mod tests {

    use bao_tree::io::outboard::PreOrderMemOutboard;
    use irpc::RpcMessage;
    use n0_future::stream;
    use testresult::TestResult;

    use super::*;
    use crate::{
        api::proto::BoxedByteStream,
        store::fs::options::{InlineOptions, PathOptions},
    };

    async fn drain<T: RpcMessage>(mut recv: mpsc::Receiver<T>) -> TestResult<Vec<T>> {
        let mut res = Vec::new();
        while let Some(item) = recv.recv().await? {
            res.push(item);
        }
        Ok(res)
    }

    fn assert_expected_progress(progress: &[AddProgressItem]) {
        assert!(progress
            .iter()
            .any(|x| matches!(&x, AddProgressItem::Size { .. })));
        assert!(progress
            .iter()
            .any(|x| matches!(&x, AddProgressItem::CopyDone)));
    }

    fn chunk_bytes(data: Bytes, chunk_size: usize) -> impl Iterator<Item = Bytes> {
        assert!(chunk_size > 0, "Chunk size must be positive");
        (0..data.len())
            .step_by(chunk_size)
            .map(move |i| data.slice(i..std::cmp::min(i + chunk_size, data.len())))
    }

    async fn test_import_byte_stream_task(data: Bytes, options: Arc<Options>) -> TestResult<()> {
        let stream: BoxedByteStream =
            Box::pin(stream::iter(chunk_bytes(data.clone(), 999).map(Ok)));
        let expected_outboard = PreOrderMemOutboard::create(data.as_ref(), IROH_BLOCK_SIZE);
        // make the channel absurdly large, so we don't have to drain it
        let (mut tx, rx) = mpsc::channel(1024 * 1024);
        let data = stream.collect::<Vec<_>>().await;
        let data = data.into_iter().collect::<io::Result<Vec<_>>>()?;
        let cmd = ImportByteStreamRequest {
            format: BlobFormat::Raw,
            scope: Default::default(),
        };
        let stream = stream::iter(data.into_iter().map(Ok));
        let res = import_byte_stream_impl(cmd, &mut tx, stream, options).await;
        let Ok(res) = res else {
            panic!("import failed");
        };
        let ImportEntry { outboard, .. } = res;
        drop(tx);
        let actual_outboard = match &outboard {
            MemOrFile::Mem(data) => data.clone(),
            MemOrFile::File(path) => std::fs::read(path)?.into(),
        };
        assert_eq!(expected_outboard.data.as_slice(), actual_outboard.as_ref());
        let progress = drain(rx).await?;
        assert_expected_progress(&progress);
        Ok(())
    }

    async fn test_import_file_task(data: Bytes, options: Arc<Options>) -> TestResult<()> {
        let path = options.path.temp_file_name();
        std::fs::write(&path, &data)?;
        let expected_outboard = PreOrderMemOutboard::create(data.as_ref(), IROH_BLOCK_SIZE);
        // make the channel absurdly large, so we don't have to drain it
        let (mut tx, rx) = mpsc::channel(1024 * 1024);
        let cmd = ImportPathRequest {
            path,
            mode: ImportMode::Copy,
            format: BlobFormat::Raw,
            scope: Scope::default(),
        };
        let res = import_path_impl(cmd, &mut tx, options).await;
        let Ok(res) = res else {
            panic!("import failed");
        };
        let ImportEntry { outboard, .. } = res;
        drop(tx);
        let actual_outboard = match &outboard {
            MemOrFile::Mem(data) => data.clone(),
            MemOrFile::File(path) => std::fs::read(path)?.into(),
        };
        assert_eq!(expected_outboard.data.as_slice(), actual_outboard.as_ref());
        let progress = drain(rx).await?;
        assert_expected_progress(&progress);
        Ok(())
    }

    #[tokio::test]
    async fn smoke() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir_all(dir.path().join("data"))?;
        std::fs::create_dir_all(dir.path().join("temp"))?;
        let options = Arc::new(Options {
            inline: InlineOptions {
                max_data_inlined: 1024 * 16,
                max_outboard_inlined: 1024 * 16,
            },
            batch: Default::default(),
            path: PathOptions::new(dir.path()),
            gc: None,
        });
        // test different sizes, below, at, and above the inline threshold
        let sizes = [
            0,               // empty, no outboard
            1024,            // data in mem, no outboard
            1024 * 16 - 1,   // data in mem, no outboard
            1024 * 16,       // data in mem, no outboard
            1024 * 16 + 1,   // data in file, outboard in mem
            1024 * 1024,     // data in file, outboard in mem
            1024 * 1024 * 8, // data in file, outboard in file
        ];
        for size in sizes {
            let data = Bytes::from(vec![0; size]);
            test_import_byte_stream_task(data.clone(), options.clone()).await?;
            test_import_file_task(data, options.clone()).await?;
        }
        Ok(())
    }
}
