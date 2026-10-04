//! The low level server side API
//!
//! Note that while using this API directly is fine, the standard way
//! to provide data is to just register a [`crate::BlobsProtocol`] protocol
//! handler with an [`iroh::Endpoint`](iroh::protocol::Router).
use std::{fmt::Debug, future::Future, io};

use bao_tree::ChunkRanges;
use iroh::endpoint::{self, ConnectionError, VarInt};
use iroh_io::{AsyncStreamReader, AsyncStreamWriter};
use n0_error::{e, stack_error, Result};
use n0_future::{
    time::{Duration, Instant},
    StreamExt,
};
use serde::{Deserialize, Serialize};
use tokio::select;
use tracing::{debug, debug_span, Instrument};

use crate::{
    api::{
        blobs::{Bitfield, WriteProgress},
        ExportBaoError, ExportBaoResult, RequestError, Store,
    },
    hashseq::HashSeq,
    protocol::{
        GetManyRequest, GetRequest, ObserveItem, ObserveRequest, PushRequest, Request, ERR_INTERNAL,
    },
    provider::events::{
        ClientConnected, ClientResult, ConnectionClosed, HasErrorCode, ProgressError,
        RequestTracker,
    },
    util::{RecvStream, RecvStreamExt, SendStream, SendStreamExt},
    Hash,
};
pub mod events;
use events::EventSender;

type DefaultReader = iroh::endpoint::RecvStream;
type DefaultWriter = iroh::endpoint::SendStream;

/// Statistics about a successful or failed transfer.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransferStats {
    /// The number of bytes sent that are part of the payload.
    pub payload_bytes_sent: u64,
    /// The number of bytes sent that are not part of the payload.
    ///
    /// Hash pairs and the initial size header.
    pub other_bytes_sent: u64,
    /// The number of bytes read from the stream.
    ///
    /// In most cases this is just the request, for push requests this is
    /// request, size header and hash pairs.
    pub other_bytes_read: u64,
    /// Total duration from reading the request to transfer completed.
    pub duration: Duration,
}

/// A pair of [`SendStream`] and [`RecvStream`] with additional context data.
#[derive(Debug)]
pub struct StreamPair<R: RecvStream = DefaultReader, W: SendStream = DefaultWriter> {
    t0: Instant,
    connection_id: u64,
    reader: R,
    writer: W,
    other_bytes_read: u64,
    events: EventSender,
}

impl StreamPair {
    pub async fn accept(
        conn: &endpoint::Connection,
        events: EventSender,
    ) -> Result<Self, ConnectionError> {
        let (writer, reader) = conn.accept_bi().await?;
        Ok(Self::new(conn.stable_id() as u64, reader, writer, events))
    }
}

impl<R: RecvStream, W: SendStream> StreamPair<R, W> {
    pub fn stream_id(&self) -> u64 {
        self.reader.id()
    }

    pub fn new(connection_id: u64, reader: R, writer: W, events: EventSender) -> Self {
        Self {
            t0: Instant::now(),
            connection_id,
            reader,
            writer,
            other_bytes_read: 0,
            events,
        }
    }

    /// Read the request.
    ///
    /// Will fail if there is an error while reading, or if no valid request is sent.
    ///
    /// This will read exactly the number of bytes needed for the request, and
    /// leave the rest of the stream for the caller to read.
    ///
    /// It is up to the caller do decide if there should be more data.
    pub async fn read_request(&mut self) -> Result<Request> {
        let (res, size) = Request::read_async(&mut self.reader).await?;
        self.other_bytes_read += size as u64;
        Ok(res)
    }

    /// We are done with reading. Return a ProgressWriter that contains the read stats and connection id
    pub async fn into_writer(
        mut self,
        tracker: RequestTracker,
    ) -> Result<ProgressWriter<W>, io::Error> {
        self.reader.expect_eof().await?;
        drop(self.reader);
        Ok(ProgressWriter::new(
            self.writer,
            WriterContext {
                t0: self.t0,
                other_bytes_read: self.other_bytes_read,
                payload_bytes_written: 0,
                other_bytes_written: 0,
                tracker,
            },
        ))
    }

    pub async fn into_reader(
        mut self,
        tracker: RequestTracker,
    ) -> Result<ProgressReader<R>, io::Error> {
        self.writer.sync().await?;
        drop(self.writer);
        Ok(ProgressReader {
            inner: self.reader,
            context: ReaderContext {
                t0: self.t0,
                other_bytes_read: self.other_bytes_read,
                tracker,
            },
        })
    }

    pub async fn get_request(
        &self,
        f: impl FnOnce() -> GetRequest,
    ) -> Result<RequestTracker, ProgressError> {
        self.events
            .request(f, self.connection_id, self.reader.id())
            .await
    }

    pub async fn get_many_request(
        &self,
        f: impl FnOnce() -> GetManyRequest,
    ) -> Result<RequestTracker, ProgressError> {
        self.events
            .request(f, self.connection_id, self.reader.id())
            .await
    }

    pub async fn push_request(
        &self,
        f: impl FnOnce() -> PushRequest,
    ) -> Result<RequestTracker, ProgressError> {
        self.events
            .request(f, self.connection_id, self.reader.id())
            .await
    }

    pub async fn observe_request(
        &self,
        f: impl FnOnce() -> ObserveRequest,
    ) -> Result<RequestTracker, ProgressError> {
        self.events
            .request(f, self.connection_id, self.reader.id())
            .await
    }

    pub fn stats(&self) -> TransferStats {
        TransferStats {
            payload_bytes_sent: 0,
            other_bytes_sent: 0,
            other_bytes_read: self.other_bytes_read,
            duration: self.t0.elapsed(),
        }
    }
}

#[derive(Debug)]
struct ReaderContext {
    /// The start time of the transfer
    t0: Instant,
    /// The number of bytes read from the stream
    other_bytes_read: u64,
    /// Progress tracking for the request
    tracker: RequestTracker,
}

impl ReaderContext {
    fn stats(&self) -> TransferStats {
        TransferStats {
            payload_bytes_sent: 0,
            other_bytes_sent: 0,
            other_bytes_read: self.other_bytes_read,
            duration: self.t0.elapsed(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct WriterContext {
    /// The start time of the transfer
    t0: Instant,
    /// The number of bytes read from the stream
    other_bytes_read: u64,
    /// The number of payload bytes written to the stream
    payload_bytes_written: u64,
    /// The number of bytes written that are not part of the payload
    other_bytes_written: u64,
    /// Way to report progress
    tracker: RequestTracker,
}

impl WriterContext {
    fn stats(&self) -> TransferStats {
        TransferStats {
            payload_bytes_sent: self.payload_bytes_written,
            other_bytes_sent: self.other_bytes_written,
            other_bytes_read: self.other_bytes_read,
            duration: self.t0.elapsed(),
        }
    }
}

impl WriteProgress for WriterContext {
    async fn notify_payload_write(&mut self, _index: u64, offset: u64, len: usize) -> ClientResult {
        let len = len as u64;
        let end_offset = offset + len;
        self.payload_bytes_written += len;
        self.tracker.transfer_progress(len, end_offset).await
    }

    fn log_other_write(&mut self, len: usize) {
        self.other_bytes_written += len as u64;
    }

    async fn send_transfer_started(&mut self, index: u64, hash: &Hash, size: u64) {
        self.tracker.transfer_started(index, hash, size).await.ok();
    }
}

/// Wrapper for a [`noq::SendStream`] with additional per request information.
#[derive(Debug)]
pub struct ProgressWriter<W: SendStream = DefaultWriter> {
    /// The noq::SendStream to write to
    pub inner: W,
    pub(crate) context: WriterContext,
}

impl<W: SendStream> ProgressWriter<W> {
    fn new(inner: W, context: WriterContext) -> Self {
        Self { inner, context }
    }

    async fn transfer_aborted(&self) {
        self.context
            .tracker
            .transfer_aborted(|| Box::new(self.context.stats()))
            .await
            .ok();
    }

    async fn transfer_completed(&self) {
        self.context
            .tracker
            .transfer_completed(|| Box::new(self.context.stats()))
            .await
            .ok();
    }
}

/// Handle a single connection.
pub async fn handle_connection(
    connection: endpoint::Connection,
    store: Store,
    progress: EventSender,
) {
    let connection_id = connection.stable_id() as u64;
    let span = debug_span!("connection", connection_id);
    async move {
        if let Err(cause) = progress
            .client_connected(|| ClientConnected {
                connection_id,
                endpoint_id: Some(connection.remote_id()),
            })
            .await
        {
            connection.close(cause.code(), cause.reason());
            debug!("closing connection: {cause}");
            return;
        }
        while let Ok(pair) = StreamPair::accept(&connection, progress.clone()).await {
            let span = debug_span!("stream", stream_id = %pair.stream_id());
            let store = store.clone();
            n0_future::task::spawn(handle_stream(pair, store).instrument(span));
        }
        progress
            .connection_closed(|| ConnectionClosed { connection_id })
            .await
            .ok();
    }
    .instrument(span)
    .await
}

/// Describes how to handle errors for a stream.
pub trait ErrorHandler {
    type W: AsyncStreamWriter;
    type R: AsyncStreamReader;
    fn stop(reader: &mut Self::R, code: VarInt) -> impl Future<Output = ()>;
    fn reset(writer: &mut Self::W, code: VarInt) -> impl Future<Output = ()>;
}

async fn handle_read_request_result<R: RecvStream, W: SendStream, T, E: HasErrorCode>(
    pair: &mut StreamPair<R, W>,
    r: Result<T, E>,
) -> Result<T, E> {
    match r {
        Ok(x) => Ok(x),
        Err(e) => {
            pair.writer.reset(e.code()).ok();
            Err(e)
        }
    }
}
async fn handle_write_result<W: SendStream, T, E: HasErrorCode>(
    writer: &mut ProgressWriter<W>,
    r: Result<T, E>,
) -> Result<T, E> {
    match r {
        Ok(x) => {
            writer.transfer_completed().await;
            Ok(x)
        }
        Err(e) => {
            writer.inner.reset(e.code()).ok();
            writer.transfer_aborted().await;
            Err(e)
        }
    }
}
async fn handle_read_result<R: RecvStream, T, E: HasErrorCode>(
    reader: &mut ProgressReader<R>,
    r: Result<T, E>,
) -> Result<T, E> {
    match r {
        Ok(x) => {
            reader.transfer_completed().await;
            Ok(x)
        }
        Err(e) => {
            reader.inner.stop(e.code()).ok();
            reader.transfer_aborted().await;
            Err(e)
        }
    }
}

pub async fn handle_stream<R: RecvStream, W: SendStream>(
    mut pair: StreamPair<R, W>,
    store: Store,
) -> n0_error::Result<()> {
    let request = pair.read_request().await?;
    match request {
        Request::Get(request) => handle_get(pair, store, request).await?,
        Request::GetMany(request) => handle_get_many(pair, store, request).await?,
        Request::Observe(request) => handle_observe(pair, store, request).await?,
        Request::Push(request) => handle_push(pair, store, request).await?,
        _ => {}
    }
    Ok(())
}

#[stack_error(derive, add_meta, from_sources)]
pub enum HandleGetError {
    #[error(transparent)]
    ExportBao {
        #[error(std_err)]
        source: ExportBaoError,
    },
    #[error("Invalid hash sequence")]
    InvalidHashSeq {},
    #[error("Invalid offset")]
    InvalidOffset {},
}

impl HasErrorCode for HandleGetError {
    fn code(&self) -> VarInt {
        match self {
            HandleGetError::ExportBao {
                source: ExportBaoError::ClientError { source, .. },
                ..
            } => source.code(),
            HandleGetError::InvalidHashSeq { .. } => ERR_INTERNAL,
            HandleGetError::InvalidOffset { .. } => ERR_INTERNAL,
            _ => ERR_INTERNAL,
        }
    }
}

/// Handle a single get request.
///
/// Requires a database, the request, and a writer.
async fn handle_get_impl<W: SendStream>(
    store: Store,
    request: GetRequest,
    writer: &mut ProgressWriter<W>,
) -> Result<(), HandleGetError> {
    let hash = request.hash;
    debug!(%hash, "get received request");
    let mut hash_seq = None;
    for (offset, ranges) in request.ranges.iter_non_empty_infinite() {
        if offset == 0 {
            send_blob(&store, offset, hash, ranges.clone(), writer).await?;
        } else {
            // todo: this assumes that 1. the hashseq is complete and 2. it is
            // small enough to fit in memory.
            //
            // This should really read the hashseq from the store in chunks,
            // only where needed, so we can deal with holes and large hashseqs.
            let hash_seq = match &hash_seq {
                Some(b) => b,
                None => {
                    let bytes = store.get_bytes(hash).await?;
                    let hs =
                        HashSeq::try_from(bytes).map_err(|_| e!(HandleGetError::InvalidHashSeq))?;
                    hash_seq = Some(hs);
                    hash_seq.as_ref().unwrap()
                }
            };
            let o = usize::try_from(offset - 1).map_err(|_| e!(HandleGetError::InvalidOffset))?;
            let Some(hash) = hash_seq.get(o) else {
                break;
            };
            send_blob(&store, offset, hash, ranges.clone(), writer).await?;
        }
    }
    writer
        .inner
        .sync()
        .await
        .map_err(|e| e!(HandleGetError::ExportBao, e.into()))?;

    Ok(())
}

pub async fn handle_get<R: RecvStream, W: SendStream>(
    mut pair: StreamPair<R, W>,
    store: Store,
    request: GetRequest,
) -> n0_error::Result<()> {
    let res = pair.get_request(|| request.clone()).await;
    let tracker = handle_read_request_result(&mut pair, res).await?;
    let mut writer = pair.into_writer(tracker).await?;
    let res = handle_get_impl(store, request, &mut writer).await;
    handle_write_result(&mut writer, res).await?;
    Ok(())
}

#[stack_error(derive, add_meta, from_sources)]
pub enum HandleGetManyError {
    #[error(transparent)]
    ExportBao { source: ExportBaoError },
}

impl HasErrorCode for HandleGetManyError {
    fn code(&self) -> VarInt {
        match self {
            Self::ExportBao {
                source: ExportBaoError::ClientError { source, .. },
                ..
            } => source.code(),
            _ => ERR_INTERNAL,
        }
    }
}

/// Handle a single get request.
///
/// Requires a database, the request, and a writer.
async fn handle_get_many_impl<W: SendStream>(
    store: Store,
    request: GetManyRequest,
    writer: &mut ProgressWriter<W>,
) -> Result<(), HandleGetManyError> {
    debug!("get_many received request");
    let request_ranges = request.ranges.iter_infinite();
    for (child, (hash, ranges)) in request.hashes.iter().zip(request_ranges).enumerate() {
        if !ranges.is_empty() {
            send_blob(&store, child as u64, *hash, ranges.clone(), writer).await?;
        }
    }
    Ok(())
}

pub async fn handle_get_many<R: RecvStream, W: SendStream>(
    mut pair: StreamPair<R, W>,
    store: Store,
    request: GetManyRequest,
) -> n0_error::Result<()> {
    let res = pair.get_many_request(|| request.clone()).await;
    let tracker = handle_read_request_result(&mut pair, res).await?;
    let mut writer = pair.into_writer(tracker).await?;
    let res = handle_get_many_impl(store, request, &mut writer).await;
    handle_write_result(&mut writer, res).await?;
    Ok(())
}

#[stack_error(derive, add_meta, from_sources)]
pub enum HandlePushError {
    #[error(transparent)]
    ExportBao { source: ExportBaoError },

    #[error("Invalid hash sequence")]
    InvalidHashSeq {},

    #[error(transparent)]
    Request { source: RequestError },
}

impl HasErrorCode for HandlePushError {
    fn code(&self) -> VarInt {
        match self {
            Self::ExportBao {
                source: ExportBaoError::ClientError { source, .. },
                ..
            } => source.code(),
            _ => ERR_INTERNAL,
        }
    }
}

/// Handle a single push request.
///
/// Requires a database, the request, and a reader.
async fn handle_push_impl<R: RecvStream>(
    store: Store,
    request: PushRequest,
    reader: &mut ProgressReader<R>,
) -> Result<(), HandlePushError> {
    let hash = request.hash;
    debug!(%hash, "push received request");
    let mut request_ranges = request.ranges.iter_infinite();
    let root_ranges = request_ranges.next().expect("infinite iterator");
    if !root_ranges.is_empty() {
        // todo: send progress from import_bao_noq or rename to import_bao_noq_with_progress
        store
            .import_bao_reader(hash, root_ranges.clone(), &mut reader.inner)
            .await?;
    }
    if request.ranges.is_blob() {
        debug!("push request complete");
        return Ok(());
    }
    // todo: we assume here that the hash sequence is complete. For some requests this might not be the case. We would need `LazyHashSeq` for that, but it is buggy as of now!
    let hash_seq = store.get_bytes(hash).await?;
    let hash_seq = HashSeq::try_from(hash_seq).map_err(|_| e!(HandlePushError::InvalidHashSeq))?;
    for (child_hash, child_ranges) in hash_seq.into_iter().zip(request_ranges) {
        if child_ranges.is_empty() {
            continue;
        }
        store
            .import_bao_reader(child_hash, child_ranges.clone(), &mut reader.inner)
            .await?;
    }
    Ok(())
}

pub async fn handle_push<R: RecvStream, W: SendStream>(
    mut pair: StreamPair<R, W>,
    store: Store,
    request: PushRequest,
) -> n0_error::Result<()> {
    let res = pair.push_request(|| request.clone()).await;
    let tracker = handle_read_request_result(&mut pair, res).await?;
    let mut reader = pair.into_reader(tracker).await?;
    let res = handle_push_impl(store, request, &mut reader).await;
    handle_read_result(&mut reader, res).await?;
    Ok(())
}

/// Send a blob to the client.
pub(crate) async fn send_blob<W: SendStream>(
    store: &Store,
    index: u64,
    hash: Hash,
    ranges: ChunkRanges,
    writer: &mut ProgressWriter<W>,
) -> ExportBaoResult<()> {
    store
        .export_bao(hash, ranges)
        .write_with_progress(&mut writer.inner, &mut writer.context, &hash, index)
        .await
}

#[stack_error(derive, add_meta, std_sources, from_sources)]
pub enum HandleObserveError {
    #[error("observe stream closed")]
    ObserveStreamClosed {},

    #[error(transparent)]
    RemoteClosed { source: io::Error },
}

impl HasErrorCode for HandleObserveError {
    fn code(&self) -> VarInt {
        ERR_INTERNAL
    }
}

/// Handle a single push request.
///
/// Requires a database, the request, and a reader.
async fn handle_observe_impl<W: SendStream>(
    store: Store,
    request: ObserveRequest,
    writer: &mut ProgressWriter<W>,
) -> std::result::Result<(), HandleObserveError> {
    let mut stream = store
        .observe(request.hash)
        .stream()
        .await
        .map_err(|_| e!(HandleObserveError::ObserveStreamClosed))?;
    let mut old = stream
        .next()
        .await
        .ok_or_else(|| e!(HandleObserveError::ObserveStreamClosed))?;
    // send the initial bitfield
    send_observe_item(writer, &old).await?;
    // send updates until the remote loses interest
    loop {
        select! {
            new = stream.next() => {
                let new = new.ok_or_else(|| e!(HandleObserveError::ObserveStreamClosed))?;
                let diff = old.diff(&new);
                if diff.is_empty() {
                    continue;
                }
                send_observe_item(writer, &diff).await?;
                old = new;
            }
            _ = writer.inner.stopped() => {
                debug!("observer closed");
                break;
            }
        }
    }
    Ok(())
}

async fn send_observe_item<W: SendStream>(
    writer: &mut ProgressWriter<W>,
    item: &Bitfield,
) -> io::Result<()> {
    let item = ObserveItem::from(item);
    let len = writer.inner.write_length_prefixed(item).await?;
    writer.context.log_other_write(len);
    Ok(())
}

pub async fn handle_observe<R: RecvStream, W: SendStream>(
    mut pair: StreamPair<R, W>,
    store: Store,
    request: ObserveRequest,
) -> n0_error::Result<()> {
    let res = pair.observe_request(|| request.clone()).await;
    let tracker = handle_read_request_result(&mut pair, res).await?;
    let mut writer = pair.into_writer(tracker).await?;
    let res = handle_observe_impl(store, request, &mut writer).await;
    handle_write_result(&mut writer, res).await?;
    Ok(())
}

pub struct ProgressReader<R: RecvStream = DefaultReader> {
    inner: R,
    context: ReaderContext,
}

impl<R: RecvStream> ProgressReader<R> {
    async fn transfer_aborted(&self) {
        self.context
            .tracker
            .transfer_aborted(|| Box::new(self.context.stats()))
            .await
            .ok();
    }

    async fn transfer_completed(&self) {
        self.context
            .tracker
            .transfer_completed(|| Box::new(self.context.stats()))
            .await
            .ok();
    }
}
