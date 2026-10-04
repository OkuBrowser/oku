//! API for downloading blobs from a single remote node.
//!
//! The entry point is the [`Remote`] struct.
use std::{
    collections::BTreeMap,
    future::{Future, IntoFuture},
    num::NonZeroU64,
    sync::Arc,
};

use bao_tree::{
    io::{BaoContentItem, Leaf},
    ChunkNum, ChunkRanges,
};
use genawaiter::sync::{Co, Gen};
use iroh::endpoint::Connection;
use irpc::util::{AsyncReadVarintExt, WriteVarintExt};
use n0_error::{e, stack_error, AnyError, Result, StdResultExt};
use n0_future::{io, Stream, StreamExt};
use ref_cast::RefCast;
use tracing::{debug, trace};

use super::blobs::{Bitfield, ExportBaoOptions};
use crate::{
    api::{
        self,
        blobs::{Blobs, WriteProgress},
        ApiClient, Store,
    },
    get::{
        fsm::{
            AtBlobHeader, AtConnected, AtEndBlob, BlobContentNext, ConnectedNext, DecodeError,
            EndBlobNext,
        },
        GetError, GetResult, Stats, StreamPair,
    },
    hashseq::{HashSeq, HashSeqIter},
    protocol::{
        ChunkRangesSeq, GetManyRequest, GetRequest, ObserveItem, ObserveRequest, PushRequest,
        Request, RequestType, MAX_MESSAGE_SIZE,
    },
    provider::events::{ClientResult, ProgressError},
    store::IROH_BLOCK_SIZE,
    util::{
        sink::{Sink, TokioMpscSenderSink},
        RecvStream, SendStream,
    },
    Hash, HashAndFormat,
};

/// API to compute request and to download from remote nodes.
///
/// Usually you want to first find out what, if any, data you have locally.
/// This can be done using [`Remote::local`], which inspects the local store
/// and returns a [`LocalInfo`].
///
/// From this you can compute various values such as the number of locally present
/// bytes. You can also compute a request to get the missing data using [`LocalInfo::missing`].
///
/// Once you have a request, you can execute it using [`Remote::execute_get`].
/// Executing a request will store to the local store, but otherwise does not take
/// the available data into account.
///
/// If you are not interested in the details and just want your data, you can use
/// [`Remote::fetch`]. This will internally do the dance described above.
#[derive(Debug, Clone, RefCast)]
#[repr(transparent)]
pub struct Remote {
    client: ApiClient,
}

#[derive(Debug)]
pub enum GetProgressItem {
    /// Progress on the payload bytes read.
    Progress(u64),
    /// The request was completed.
    Done(Stats),
    /// The request was closed, but not completed.
    Error(GetError),
}

impl From<GetResult<Stats>> for GetProgressItem {
    fn from(res: GetResult<Stats>) -> Self {
        match res {
            Ok(stats) => GetProgressItem::Done(stats),
            Err(e) => GetProgressItem::Error(e),
        }
    }
}

impl TryFrom<GetProgressItem> for GetResult<Stats> {
    type Error = &'static str;

    fn try_from(item: GetProgressItem) -> Result<Self, Self::Error> {
        match item {
            GetProgressItem::Done(stats) => Ok(Ok(stats)),
            GetProgressItem::Error(e) => Ok(Err(e)),
            GetProgressItem::Progress(_) => Err("not a final item"),
        }
    }
}

pub struct GetProgress {
    rx: tokio::sync::mpsc::Receiver<GetProgressItem>,
    fut: n0_future::boxed::BoxFuture<()>,
}

impl IntoFuture for GetProgress {
    type Output = GetResult<Stats>;
    type IntoFuture = n0_future::boxed::BoxFuture<Self::Output>;

    fn into_future(self) -> n0_future::boxed::BoxFuture<Self::Output> {
        Box::pin(self.complete())
    }
}

impl GetProgress {
    pub fn stream(self) -> impl Stream<Item = GetProgressItem> {
        into_stream(self.rx, self.fut)
    }

    pub async fn complete(self) -> GetResult<Stats> {
        just_result(self.stream()).await.unwrap_or_else(|| {
            Err(e!(
                GetError::LocalFailure,
                n0_error::anyerr!("stream closed without result")
            ))
        })
    }
}

#[derive(Debug)]
pub enum PushProgressItem {
    /// Progress on the payload bytes read.
    Progress(u64),
    /// The request was completed.
    Done(Stats),
    /// The request was closed, but not completed.
    Error(AnyError),
}

impl From<Result<Stats>> for PushProgressItem {
    fn from(res: Result<Stats>) -> Self {
        match res {
            Ok(stats) => Self::Done(stats),
            Err(e) => Self::Error(e),
        }
    }
}

impl TryFrom<PushProgressItem> for Result<Stats> {
    type Error = &'static str;

    fn try_from(item: PushProgressItem) -> Result<Self, Self::Error> {
        match item {
            PushProgressItem::Done(stats) => Ok(Ok(stats)),
            PushProgressItem::Error(e) => Ok(Err(e)),
            PushProgressItem::Progress(_) => Err("not a final item"),
        }
    }
}

pub struct PushProgress {
    rx: tokio::sync::mpsc::Receiver<PushProgressItem>,
    fut: n0_future::boxed::BoxFuture<()>,
}

impl IntoFuture for PushProgress {
    type Output = Result<Stats>;
    type IntoFuture = n0_future::boxed::BoxFuture<Self::Output>;

    fn into_future(self) -> n0_future::boxed::BoxFuture<Self::Output> {
        Box::pin(self.complete())
    }
}

impl PushProgress {
    pub fn stream(self) -> impl Stream<Item = PushProgressItem> {
        into_stream(self.rx, self.fut)
    }

    pub async fn complete(self) -> Result<Stats> {
        just_result(self.stream())
            .await
            .unwrap_or_else(|| Err(n0_error::anyerr!("stream closed without result")))
    }
}

async fn just_result<S, R>(stream: S) -> Option<R>
where
    S: Stream<Item: std::fmt::Debug>,
    R: TryFrom<S::Item>,
{
    tokio::pin!(stream);
    while let Some(item) = stream.next().await {
        if let Ok(res) = R::try_from(item) {
            return Some(res);
        }
    }
    None
}

fn into_stream<T, F>(mut rx: tokio::sync::mpsc::Receiver<T>, fut: F) -> impl Stream<Item = T>
where
    F: Future,
{
    Gen::new(move |co| async move {
        tokio::pin!(fut);
        loop {
            tokio::select! {
                biased;
                item = rx.recv() => {
                    if let Some(item) = item {
                        co.yield_(item).await;
                    } else {
                        break;
                    }
                }
                _ = &mut fut => {
                    break;
                }
            }
        }
        while let Some(item) = rx.recv().await {
            co.yield_(item).await;
        }
    })
}

/// Local info for a blob or hash sequence.
///
/// This can be used to get the amount of missing data, and to construct a
/// request to get the missing data.
#[derive(Debug)]
pub struct LocalInfo {
    /// The hash for which this is the local info
    request: Arc<GetRequest>,
    /// The bitfield for the root hash
    bitfield: Bitfield,
    /// Optional - the hash sequence info if this was a request for a hash sequence
    children: Option<NonRawLocalInfo>,
}

impl LocalInfo {
    /// The number of bytes we have locally
    pub fn local_bytes(&self) -> u64 {
        let Some(root_requested) = self.requested_root_ranges() else {
            // empty request requests 0 bytes
            return 0;
        };
        let mut local = self.bitfield.clone();
        local.ranges.intersection_with(root_requested);
        let mut res = local.total_bytes();
        if let Some(children) = &self.children {
            let Some(max_local_index) = children.hash_seq.keys().next_back() else {
                // no children
                return res;
            };
            for (offset, ranges) in self.request.ranges.iter_non_empty_infinite() {
                if offset == 0 {
                    // skip the root hash
                    continue;
                }
                let child = offset - 1;
                if child > *max_local_index {
                    // we are done
                    break;
                }
                let Some(hash) = children.hash_seq.get(&child) else {
                    continue;
                };
                let bitfield = &children.bitfields[hash];
                let mut local = bitfield.clone();
                local.ranges.intersection_with(ranges);
                res += local.total_bytes();
            }
        }
        res
    }

    /// Number of children in this hash sequence
    pub fn children(&self) -> Option<u64> {
        if self.children.is_some() {
            self.bitfield.validated_size().map(|x| x / 32)
        } else {
            Some(0)
        }
    }

    /// The requested root ranges.
    ///
    /// This will return None if the request is empty, and an empty CHunkRanges
    /// if no ranges were requested for the root hash.
    fn requested_root_ranges(&self) -> Option<&ChunkRanges> {
        self.request.ranges.iter().next()
    }

    /// True if the data is complete.
    ///
    /// For a blob, this is true if the blob is complete.
    /// For a hash sequence, this is true if the hash sequence is complete and
    /// all its children are complete.
    pub fn is_complete(&self) -> bool {
        let Some(root_requested) = self.requested_root_ranges() else {
            // empty request is complete
            return true;
        };
        if !self.bitfield.ranges.is_superset(root_requested) {
            return false;
        }
        if let Some(children) = self.children.as_ref() {
            let iter = self.request.ranges.iter_non_empty_infinite();
            let max_child = self.bitfield.validated_size().map(|x| x / 32);
            for (offset, range) in iter {
                if offset == 0 {
                    // skip the root hash
                    continue;
                }
                let child = offset - 1;
                if let Some(hash) = children.hash_seq.get(&child) {
                    let bitfield = &children.bitfields[hash];
                    if !bitfield.ranges.is_superset(range) {
                        // we don't have the requested ranges
                        return false;
                    }
                } else {
                    if let Some(max_child) = max_child {
                        if child >= max_child {
                            // reading after the end of the request
                            return true;
                        }
                    }
                    return false;
                }
            }
        }
        true
    }

    /// A request to get the missing data to complete this request
    pub fn missing(&self) -> GetRequest {
        let Some(root_requested) = self.requested_root_ranges() else {
            // empty request is complete
            return GetRequest::new(self.request.hash, ChunkRangesSeq::empty());
        };
        let mut builder = GetRequest::builder().root(root_requested - &self.bitfield.ranges);

        let Some(children) = self.children.as_ref() else {
            return builder.build(self.request.hash);
        };
        let mut iter = self.request.ranges.iter_non_empty_infinite();
        let max_local = children
            .hash_seq
            .keys()
            .next_back()
            .map(|x| *x + 1)
            .unwrap_or_default();
        let max_offset = self.bitfield.validated_size().map(|x| x / 32);
        for (offset, requested) in iter.by_ref() {
            if offset == 0 {
                // skip the root hash
                continue;
            }
            let child = offset - 1;
            let missing = match children.hash_seq.get(&child) {
                Some(hash) => requested.difference(&children.bitfields[hash].ranges),
                None => requested.clone(),
            };
            builder = builder.child(child, missing);
            if offset >= max_local {
                // we can't do anything clever anymore
                break;
            }
        }
        loop {
            let Some((offset, requested)) = iter.next() else {
                return builder.build(self.request.hash);
            };
            if offset == 0 {
                // skip the root hash
                continue;
            }
            let child = offset - 1;
            if let Some(max_offset) = &max_offset {
                if child >= *max_offset {
                    return builder.build(self.request.hash);
                }
                builder = builder.child(child, requested.clone());
            } else {
                builder = builder.child(child, requested.clone());
                if iter.is_at_end() {
                    if iter.next().is_none() {
                        return builder.build(self.request.hash);
                    } else {
                        return builder.build_open(self.request.hash);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct NonRawLocalInfo {
    /// the available and relevant part of the hash sequence
    hash_seq: BTreeMap<u64, Hash>,
    /// For each hash relevant to the request, the local bitfield and the ranges
    /// that were requested.
    bitfields: BTreeMap<Hash, Bitfield>,
}

// fn iter_without_gaps<'a, T: Copy + 'a>(
//     iter: impl IntoIterator<Item = &'a (u64, T)> + 'a,
// ) -> impl Iterator<Item = (u64, Option<T>)> + 'a {
//     let mut prev = 0;
//     iter.into_iter().flat_map(move |(i, hash)| {
//         let start = prev + 1;
//         let curr = *i;
//         prev = *i;
//         (start..curr)
//             .map(|i| (i, None))
//             .chain(std::iter::once((curr, Some(*hash))))
//     })
// }

impl Remote {
    pub(crate) fn ref_from_sender(sender: &ApiClient) -> &Self {
        Self::ref_cast(sender)
    }

    fn store(&self) -> &Store {
        Store::ref_from_sender(&self.client)
    }

    pub async fn local_for_request(
        &self,
        request: impl Into<Arc<GetRequest>>,
    ) -> Result<LocalInfo> {
        let request = request.into();
        let root = request.hash;
        let bitfield = self.store().observe(root).await?;
        let children = if !request.ranges.is_blob() {
            let opts = ExportBaoOptions {
                hash: root,
                ranges: bitfield.ranges.clone(),
            };
            let bao = self.store().export_bao_with_opts(opts, 32);
            let mut by_index = BTreeMap::new();
            let mut stream = bao.hashes_with_index();
            while let Some(item) = stream.next().await {
                if let Ok((index, hash)) = item {
                    by_index.insert(index, hash);
                }
            }
            let mut bitfields = BTreeMap::new();
            let mut hash_seq = BTreeMap::new();
            let max = by_index.last_key_value().map(|(k, _)| *k + 1).unwrap_or(0);
            for (index, _) in request.ranges.iter_non_empty_infinite() {
                if index == 0 {
                    // skip the root hash
                    continue;
                }
                let child = index - 1;
                if child > max {
                    // we are done
                    break;
                }
                let Some(hash) = by_index.get(&child) else {
                    // we don't have the hash, so we can't store the bitfield
                    continue;
                };
                let bitfield = self.store().observe(*hash).await?;
                bitfields.insert(*hash, bitfield);
                hash_seq.insert(child, *hash);
            }
            Some(NonRawLocalInfo {
                hash_seq,
                bitfields,
            })
        } else {
            None
        };
        Ok(LocalInfo {
            request: request.clone(),
            bitfield,
            children,
        })
    }

    /// Get the local info for a given blob or hash sequence, at the present time.
    pub async fn local(&self, content: impl Into<HashAndFormat>) -> Result<LocalInfo> {
        let request = GetRequest::from(content.into());
        self.local_for_request(request).await
    }

    pub fn fetch(
        &self,
        sp: impl GetStreamPair + 'static,
        content: impl Into<HashAndFormat>,
    ) -> GetProgress {
        let content = content.into();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx2 = tx.clone();
        let sink = TokioMpscSenderSink(tx).with_map(GetProgressItem::Progress);
        let this = self.clone();
        let fut = async move {
            let res = this.fetch_sink(sp, content, sink).await.into();
            tx2.send(res).await.ok();
        };
        GetProgress {
            rx,
            fut: Box::pin(fut),
        }
    }

    /// Get a blob or hash sequence from the given connection, taking the locally available
    /// ranges into account.
    ///
    /// You can provide a progress channel to get updates on the download progress. This progress
    /// is the aggregated number of downloaded payload bytes in the request.
    ///
    /// This will return the stats of the download.
    pub(crate) async fn fetch_sink(
        &self,
        sp: impl GetStreamPair,
        content: impl Into<HashAndFormat>,
        progress: impl Sink<u64, Error = irpc::channel::SendError>,
    ) -> GetResult<Stats> {
        let content = content.into();
        let local = self
            .local(content)
            .await
            .map_err(|e| e!(GetError::LocalFailure, e))?;
        if local.is_complete() {
            return Ok(Default::default());
        }
        let request = local.missing();
        let stats = self.execute_get_sink(sp, request, progress).await?;
        Ok(stats)
    }

    pub fn observe(
        &self,
        conn: Connection,
        request: ObserveRequest,
    ) -> impl Stream<Item = io::Result<Bitfield>> + 'static {
        Gen::new(|co| async move {
            if let Err(cause) = Self::observe_impl(conn, request, &co).await {
                co.yield_(Err(cause)).await
            }
        })
    }

    async fn observe_impl(
        conn: Connection,
        request: ObserveRequest,
        co: &Co<io::Result<Bitfield>>,
    ) -> io::Result<()> {
        let hash = request.hash;
        debug!(%hash, "observing");
        let (mut send, mut recv) = conn.open_bi().await?;
        // write the request. Unlike for reading, we can just serialize it sync using postcard.
        write_observe_request(request, &mut send).await?;
        send.finish()?;
        loop {
            let msg = recv
                .read_length_prefixed::<ObserveItem>(MAX_MESSAGE_SIZE)
                .await?;
            co.yield_(Ok(Bitfield::from(&msg))).await;
        }
    }

    pub fn execute_push(&self, conn: Connection, request: PushRequest) -> PushProgress {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx2 = tx.clone();
        let sink = TokioMpscSenderSink(tx).with_map(PushProgressItem::Progress);
        let this = self.clone();
        let fut = async move {
            let res = this.execute_push_sink(conn, request, sink).await.into();
            tx2.send(res).await.ok();
        };
        PushProgress {
            rx,
            fut: Box::pin(fut),
        }
    }

    /// Push the given blob or hash sequence to a remote node.
    ///
    /// Note that many nodes will reject push requests. Also, this is an experimental feature for now.
    pub(crate) async fn execute_push_sink(
        &self,
        conn: Connection,
        request: PushRequest,
        progress: impl Sink<u64, Error = irpc::channel::SendError>,
    ) -> Result<Stats> {
        let hash = request.hash;
        debug!(%hash, "pushing");
        let (mut send, mut recv) = conn.open_bi().await.anyerr()?;
        let mut context = StreamContext {
            payload_bytes_sent: 0,
            sender: progress,
        };
        // we are not going to need this!
        recv.stop(0u32.into()).anyerr()?;
        // write the request. Unlike for reading, we can just serialize it sync using postcard.
        let request = write_push_request(request, &mut send).await?;
        let mut request_ranges = request.ranges.iter_infinite();
        let root = request.hash;
        let root_ranges = request_ranges.next().expect("infinite iterator");
        if !root_ranges.is_empty() {
            self.store()
                .export_bao(root, root_ranges.clone())
                .write_with_progress(&mut send, &mut context, &root, 0)
                .await?;
        }
        if request.ranges.is_blob() {
            // we are done
            send.finish().anyerr()?;
            return Ok(Default::default());
        }
        let hash_seq = self.store().get_bytes(root).await?;
        let hash_seq = HashSeq::try_from(hash_seq)?;
        for (child, (child_hash, child_ranges)) in
            hash_seq.into_iter().zip(request_ranges).enumerate()
        {
            if !child_ranges.is_empty() {
                self.store()
                    .export_bao(child_hash, child_ranges.clone())
                    .write_with_progress(&mut send, &mut context, &child_hash, (child + 1) as u64)
                    .await?;
            }
        }
        send.finish().anyerr()?;
        Ok(Default::default())
    }

    pub fn execute_get(&self, conn: impl GetStreamPair, request: GetRequest) -> GetProgress {
        self.execute_get_with_opts(conn, request)
    }

    pub fn execute_get_with_opts(
        &self,
        conn: impl GetStreamPair,
        request: GetRequest,
    ) -> GetProgress {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx2 = tx.clone();
        let sink = TokioMpscSenderSink(tx).with_map(GetProgressItem::Progress);
        let this = self.clone();
        let fut = async move {
            let res = this.execute_get_sink(conn, request, sink).await.into();
            tx2.send(res).await.ok();
        };
        GetProgress {
            rx,
            fut: Box::pin(fut),
        }
    }

    /// Execute a get request *without* taking the locally available ranges into account.
    ///
    /// You can provide a progress channel to get updates on the download progress. This progress
    /// is the aggregated number of downloaded payload bytes in the request.
    ///
    /// This will download the data again even if the data is locally present.
    ///
    /// This will return the stats of the download.
    pub(crate) async fn execute_get_sink(
        &self,
        conn: impl GetStreamPair,
        request: GetRequest,
        mut progress: impl Sink<u64, Error = irpc::channel::SendError>,
    ) -> GetResult<Stats> {
        let store = self.store();
        let root = request.hash;
        let conn = conn.open_stream_pair().await.map_err(|e| {
            e!(
                GetError::LocalFailure,
                n0_error::anyerr!("failed to open stream pair: {e}")
            )
        })?;
        // I am cloning the connection, but it's fine because the original connection or ConnectionRef stays alive
        // for the duration of the operation.
        let connected =
            AtConnected::new(conn.t0, conn.recv, conn.send, request, Default::default());
        trace!("Getting header");
        // read the header
        let next_child = match connected
            .next()
            .await
            .map_err(|e| e!(GetError::ConnectedNext, e))?
        {
            ConnectedNext::StartRoot(at_start_root) => {
                let header = at_start_root.next();
                let end = get_blob_ranges_impl(header, root, store, &mut progress).await?;
                match end.next() {
                    EndBlobNext::MoreChildren(at_start_child) => Ok(at_start_child),
                    EndBlobNext::Closing(at_closing) => Err(at_closing),
                }
            }
            ConnectedNext::StartChild(at_start_child) => Ok(at_start_child),
            ConnectedNext::Closing(at_closing) => Err(at_closing),
        };
        // read the rest, if any
        let at_closing = match next_child {
            Ok(at_start_child) => {
                let mut next_child = Ok(at_start_child);
                let hash_seq = HashSeq::try_from(
                    store
                        .get_bytes(root)
                        .await
                        .map_err(|e| e!(GetError::LocalFailure, e.into()))?,
                )
                .map_err(|e| e!(GetError::BadRequest, e))?;
                // let mut hash_seq = LazyHashSeq::new(store.blobs().clone(), root);
                loop {
                    let at_start_child = match next_child {
                        Ok(at_start_child) => at_start_child,
                        Err(at_closing) => break at_closing,
                    };
                    let offset = at_start_child.offset() - 1;
                    let Some(hash) = hash_seq.get(offset as usize) else {
                        break at_start_child.finish();
                    };
                    trace!("getting child {offset} {}", hash.fmt_short());
                    let header = at_start_child.next(hash);
                    let end = get_blob_ranges_impl(header, hash, store, &mut progress).await?;
                    next_child = match end.next() {
                        EndBlobNext::MoreChildren(at_start_child) => Ok(at_start_child),
                        EndBlobNext::Closing(at_closing) => Err(at_closing),
                    }
                }
            }
            Err(at_closing) => at_closing,
        };
        // read the rest, if any
        let stats = at_closing
            .next()
            .await
            .map_err(|e| e!(GetError::AtClosingNext, e))?;
        trace!(?stats, "get hash seq done");
        Ok(stats)
    }

    pub fn execute_get_many(&self, conn: Connection, request: GetManyRequest) -> GetProgress {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let tx2 = tx.clone();
        let sink = TokioMpscSenderSink(tx).with_map(GetProgressItem::Progress);
        let this = self.clone();
        let fut = async move {
            let res = this.execute_get_many_sink(conn, request, sink).await.into();
            tx2.send(res).await.ok();
        };
        GetProgress {
            rx,
            fut: Box::pin(fut),
        }
    }

    /// Execute a get request *without* taking the locally available ranges into account.
    ///
    /// You can provide a progress channel to get updates on the download progress. This progress
    /// is the aggregated number of downloaded payload bytes in the request.
    ///
    /// This will download the data again even if the data is locally present.
    ///
    /// This will return the stats of the download.
    pub async fn execute_get_many_sink(
        &self,
        conn: Connection,
        request: GetManyRequest,
        mut progress: impl Sink<u64, Error = irpc::channel::SendError>,
    ) -> GetResult<Stats> {
        let store = self.store();
        let hash_seq = request.hashes.iter().copied().collect::<HashSeq>();
        let next_child = crate::get::fsm::start_get_many(conn, request, Default::default()).await?;
        // read all children.
        let at_closing = match next_child {
            Ok(at_start_child) => {
                let mut next_child = Ok(at_start_child);
                loop {
                    let at_start_child = match next_child {
                        Ok(at_start_child) => at_start_child,
                        Err(at_closing) => break at_closing,
                    };
                    let offset = at_start_child.offset();
                    let Some(hash) = hash_seq.get(offset as usize) else {
                        break at_start_child.finish();
                    };
                    trace!("getting child {offset} {}", hash.fmt_short());
                    let header = at_start_child.next(hash);
                    let end = get_blob_ranges_impl(header, hash, store, &mut progress).await?;
                    next_child = match end.next() {
                        EndBlobNext::MoreChildren(at_start_child) => Ok(at_start_child),
                        EndBlobNext::Closing(at_closing) => Err(at_closing),
                    }
                }
            }
            Err(at_closing) => at_closing,
        };
        // read the rest, if any
        let stats = at_closing
            .next()
            .await
            .map_err(|e| e!(GetError::AtClosingNext, e))?;
        trace!(?stats, "get hash seq done");
        Ok(stats)
    }
}

/// Failures for a get operation
#[allow(missing_docs)]
#[non_exhaustive]
#[stack_error(derive, add_meta)]
pub enum ExecuteError {
    /// Network or IO operation failed.
    #[error("Unable to open bidi stream")]
    Connection {
        #[error(std_err)]
        source: iroh::endpoint::ConnectionError,
    },
    #[error("Unable to read from the remote")]
    Read {
        #[error(std_err)]
        source: iroh::endpoint::ReadError,
    },
    #[error("Error sending the request")]
    Send {
        #[error(std_err)]
        source: crate::get::fsm::ConnectedNextError,
    },
    #[error("Unable to read size")]
    Size {
        #[error(std_err)]
        source: crate::get::fsm::AtBlobHeaderNextError,
    },
    #[error("Error while decoding the data")]
    Decode {
        #[error(std_err)]
        source: crate::get::fsm::DecodeError,
    },
    #[error("Internal error while reading the hash sequence")]
    ExportBao { source: api::ExportBaoError },
    #[error("Hash sequence has an invalid length")]
    InvalidHashSeq { source: AnyError },
    #[error("Internal error importing the data")]
    ImportBao { source: crate::api::RequestError },
    #[error("Error sending download progress - receiver closed")]
    SendDownloadProgress { source: irpc::channel::SendError },
    #[error("Internal error importing the data")]
    MpscSend {
        #[error(std_err)]
        source: tokio::sync::mpsc::error::SendError<BaoContentItem>,
    },
}

pub trait GetStreamPair: Send + 'static {
    fn open_stream_pair(
        self,
    ) -> impl Future<Output = io::Result<StreamPair<impl RecvStream, impl SendStream>>> + Send + 'static;
}

impl<R: RecvStream + 'static, W: SendStream + 'static> GetStreamPair for StreamPair<R, W> {
    async fn open_stream_pair(self) -> io::Result<StreamPair<impl RecvStream, impl SendStream>> {
        Ok(self)
    }
}

impl GetStreamPair for Connection {
    async fn open_stream_pair(
        self,
    ) -> io::Result<StreamPair<impl crate::util::RecvStream, impl crate::util::SendStream>> {
        let connection_id = self.stable_id() as u64;
        let (send, recv) = self.open_bi().await?;
        Ok(StreamPair::new(connection_id, recv, send))
    }
}

fn get_buffer_size(size: NonZeroU64) -> usize {
    (size.get() / (IROH_BLOCK_SIZE.bytes() as u64) + 2).min(64) as usize
}

async fn get_blob_ranges_impl<R: RecvStream>(
    header: AtBlobHeader<R>,
    hash: Hash,
    store: &Store,
    mut progress: impl Sink<u64, Error = irpc::channel::SendError>,
) -> GetResult<AtEndBlob<R>> {
    let (mut content, size) = header
        .next()
        .await
        .map_err(|e| e!(GetError::AtBlobHeaderNext, e))?;
    let Some(size) = NonZeroU64::new(size) else {
        return if hash == Hash::EMPTY {
            let end = content.drain().await.map_err(|e| e!(GetError::Decode, e))?;
            Ok(end)
        } else {
            Err(e!(
                GetError::Decode,
                DecodeError::leaf_hash_mismatch(ChunkNum(0))
            ))
        };
    };
    let buffer_size = get_buffer_size(size);
    trace!(%size, %buffer_size, "get blob");
    let handle = store
        .import_bao(hash, size, buffer_size)
        .await
        .map_err(|e| e!(GetError::LocalFailure, e.into()))?;
    let write = async move {
        GetResult::Ok(loop {
            match content.next().await {
                BlobContentNext::More((next, res)) => {
                    let item = res.map_err(|e| e!(GetError::Decode, e))?;
                    progress
                        .send(next.stats().payload_bytes_read)
                        .await
                        .map_err(|e| e!(GetError::LocalFailure, e.into()))?;
                    handle
                        .tx
                        .send(item)
                        .await
                        .map_err(|e| e!(GetError::IrpcSend, e))?;
                    content = next;
                }
                BlobContentNext::Done(end) => {
                    drop(handle.tx);
                    break end;
                }
            }
        })
    };
    let complete = async move {
        handle.rx.await.map_err(|e| {
            e!(
                GetError::LocalFailure,
                n0_error::anyerr!("error reading from import stream: {e}")
            )
        })
    };
    let (_, end) = tokio::try_join!(complete, write)?;
    Ok(end)
}

#[derive(Debug)]
pub(crate) struct LazyHashSeq {
    blobs: Blobs,
    hash: Hash,
    current_chunk: Option<HashSeqChunk>,
}

#[derive(Debug)]
pub(crate) struct HashSeqChunk {
    /// the offset of the first hash in this chunk, in bytes
    offset: u64,
    /// the hashes in this chunk
    chunk: HashSeq,
}

impl TryFrom<Leaf> for HashSeqChunk {
    type Error = AnyError;

    fn try_from(leaf: Leaf) -> Result<Self, Self::Error> {
        let offset = leaf.offset;
        let chunk = HashSeq::try_from(leaf.data)?;
        Ok(Self { offset, chunk })
    }
}

impl IntoIterator for HashSeqChunk {
    type Item = Hash;
    type IntoIter = HashSeqIter;

    fn into_iter(self) -> Self::IntoIter {
        self.chunk.into_iter()
    }
}

impl HashSeqChunk {
    pub fn base(&self) -> u64 {
        self.offset / 32
    }

    #[allow(dead_code)]
    fn get(&self, offset: u64) -> Option<Hash> {
        let start = self.offset;
        let end = start + self.chunk.len() as u64;
        if offset >= start && offset < end {
            let o = (offset - start) as usize;
            self.chunk.get(o)
        } else {
            None
        }
    }
}

impl LazyHashSeq {
    #[allow(dead_code)]
    pub fn new(blobs: Blobs, hash: Hash) -> Self {
        Self {
            blobs,
            hash,
            current_chunk: None,
        }
    }

    #[allow(dead_code)]
    pub async fn get_from_offset(&mut self, offset: u64) -> Result<Option<Hash>> {
        if offset == 0 {
            Ok(Some(self.hash))
        } else {
            self.get(offset - 1).await
        }
    }

    #[allow(dead_code)]
    pub async fn get(&mut self, child_offset: u64) -> Result<Option<Hash>> {
        // check if we have the hash in the current chunk
        if let Some(chunk) = &self.current_chunk {
            if let Some(hash) = chunk.get(child_offset) {
                return Ok(Some(hash));
            }
        }
        // load the chunk covering the offset
        let leaf = self
            .blobs
            .export_chunk(self.hash, child_offset * 32)
            .await?;
        // return the hash if it is in the chunk, otherwise we are behind the end
        let hs = HashSeqChunk::try_from(leaf)?;
        Ok(hs.get(child_offset).inspect(|_hash| {
            self.current_chunk = Some(hs);
        }))
    }
}

async fn write_push_request(
    request: PushRequest,
    stream: &mut impl SendStream,
) -> Result<PushRequest> {
    let mut request_bytes = Vec::new();
    request_bytes.push(RequestType::Push as u8);
    request_bytes.write_length_prefixed(&request).unwrap();
    stream.send_bytes(request_bytes.into()).await?;
    Ok(request)
}

async fn write_observe_request(
    request: ObserveRequest,
    stream: &mut impl SendStream,
) -> io::Result<()> {
    let request = Request::Observe(request);
    let request_bytes = postcard::to_allocvec(&request)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    stream.send_bytes(request_bytes.into()).await?;
    Ok(())
}

struct StreamContext<S> {
    payload_bytes_sent: u64,
    sender: S,
}

impl<S> WriteProgress for StreamContext<S>
where
    S: Sink<u64, Error = irpc::channel::SendError>,
{
    async fn notify_payload_write(
        &mut self,
        _index: u64,
        _offset: u64,
        len: usize,
    ) -> ClientResult {
        self.payload_bytes_sent += len as u64;
        self.sender
            .send(self.payload_bytes_sent)
            .await
            .map_err(|e| n0_error::e!(ProgressError::Internal, e.into()))?;
        Ok(())
    }

    fn log_other_write(&mut self, _len: usize) {}

    async fn send_transfer_started(&mut self, _index: u64, _hash: &Hash, _size: u64) {}
}

#[cfg(test)]
#[cfg(feature = "fs-store")]
mod tests {
    use bao_tree::{ChunkNum, ChunkRanges};
    use testresult::TestResult;

    use crate::{
        api::blobs::Blobs,
        protocol::{ChunkRangesExt, ChunkRangesSeq, GetRequest},
        store::{
            fs::{
                tests::{test_data, INTERESTING_SIZES},
                FsStore,
            },
            mem::MemStore,
            util::tests::create_n0_bao,
        },
        tests::{add_test_hash_seq, add_test_hash_seq_incomplete},
    };

    #[tokio::test]
    async fn test_local_info_raw() -> TestResult<()> {
        let td = tempfile::tempdir()?;
        let store = FsStore::load(td.path().join("blobs.db")).await?;
        let blobs = store.blobs();
        let tt = blobs.add_slice(b"test").temp_tag().await?;
        let hash = tt.hash();
        let info = store.remote().local(hash).await?;
        assert_eq!(info.bitfield.ranges, ChunkRanges::all());
        assert_eq!(info.local_bytes(), 4);
        assert!(info.is_complete());
        assert_eq!(
            info.missing(),
            GetRequest::new(hash, ChunkRangesSeq::empty())
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_local_info_hash_seq_large() -> TestResult<()> {
        let sizes = (0..1024 + 5).collect::<Vec<_>>();
        let relevant_sizes = sizes[32 * 16..32 * 32]
            .iter()
            .map(|x| *x as u64)
            .sum::<u64>();
        let td = tempfile::tempdir()?;
        let hash_seq_ranges = ChunkRanges::chunks(16..32);
        let store = FsStore::load(td.path().join("blobs.db")).await?;
        {
            // only add the hash seq itself, and only the first chunk of the children
            let present = |i| {
                if i == 0 {
                    hash_seq_ranges.clone()
                } else {
                    ChunkRanges::from(..ChunkNum(1))
                }
            };
            let content = add_test_hash_seq_incomplete(&store, sizes, present).await?;
            let info = store.remote().local(content).await?;
            assert_eq!(info.bitfield.ranges, hash_seq_ranges);
            assert!(!info.is_complete());
            assert_eq!(info.local_bytes(), relevant_sizes + 16 * 1024);
        }

        Ok(())
    }

    async fn test_observe_partial(blobs: &Blobs) -> TestResult<()> {
        let sizes = INTERESTING_SIZES;
        for size in sizes {
            let data = test_data(size);
            let ranges = ChunkRanges::chunk(0);
            let (hash, bao) = create_n0_bao(&data, &ranges)?;
            blobs.import_bao_bytes(hash, ranges.clone(), bao).await?;
            let bitfield = blobs.observe(hash).await?;
            if size > 1024 {
                assert_eq!(bitfield.ranges, ranges);
            } else {
                assert_eq!(bitfield.ranges, ChunkRanges::all());
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_observe_partial_mem() -> TestResult<()> {
        let store = MemStore::new();
        test_observe_partial(store.blobs()).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_observe_partial_fs() -> TestResult<()> {
        let td = tempfile::tempdir()?;
        let store = FsStore::load(td.path()).await?;
        test_observe_partial(store.blobs()).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_local_info_hash_seq() -> TestResult<()> {
        let sizes = INTERESTING_SIZES;
        let total_size = sizes.iter().map(|x| *x as u64).sum::<u64>();
        let hash_seq_size = (sizes.len() as u64) * 32;
        let td = tempfile::tempdir()?;
        let store = FsStore::load(td.path().join("blobs.db")).await?;
        {
            // only add the hash seq itself, none of the children
            let present = |i| {
                if i == 0 {
                    ChunkRanges::all()
                } else {
                    ChunkRanges::empty()
                }
            };
            let content = add_test_hash_seq_incomplete(&store, sizes, present).await?;
            let info = store.remote().local(content).await?;
            assert_eq!(info.bitfield.ranges, ChunkRanges::all());
            assert_eq!(info.local_bytes(), hash_seq_size);
            assert!(!info.is_complete());
            assert_eq!(
                info.missing(),
                GetRequest::new(
                    content.hash,
                    ChunkRangesSeq::from_ranges([
                        ChunkRanges::empty(), // we have the hash seq itself
                        ChunkRanges::empty(), // we always have the empty blob
                        ChunkRanges::all(),   // we miss all the remaining blobs (sizes.len() - 1)
                        ChunkRanges::all(),
                        ChunkRanges::all(),
                        ChunkRanges::all(),
                        ChunkRanges::all(),
                        ChunkRanges::all(),
                        ChunkRanges::all(),
                    ])
                )
            );
            store.tags().delete_all().await?;
        }
        {
            // only add the hash seq itself, and only the first chunk of the children
            let present = |i| {
                if i == 0 {
                    ChunkRanges::all()
                } else {
                    ChunkRanges::from(..ChunkNum(1))
                }
            };
            let content = add_test_hash_seq_incomplete(&store, sizes, present).await?;
            let info = store.remote().local(content).await?;
            let first_chunk_size = sizes.into_iter().map(|x| x.min(1024) as u64).sum::<u64>();
            assert_eq!(info.bitfield.ranges, ChunkRanges::all());
            assert_eq!(info.local_bytes(), hash_seq_size + first_chunk_size);
            assert!(!info.is_complete());
            assert_eq!(
                info.missing(),
                GetRequest::new(
                    content.hash,
                    ChunkRangesSeq::from_ranges([
                        ChunkRanges::empty(), // we have the hash seq itself
                        ChunkRanges::empty(), // we always have the empty blob
                        ChunkRanges::empty(), // size=1
                        ChunkRanges::empty(), // size=1024
                        ChunkRanges::chunks(1..),
                        ChunkRanges::chunks(1..),
                        ChunkRanges::chunks(1..),
                        ChunkRanges::chunks(1..),
                        ChunkRanges::chunks(1..),
                    ])
                )
            );
        }
        {
            let content = add_test_hash_seq(&store, sizes).await?;
            let info = store.remote().local(content).await?;
            assert_eq!(info.bitfield.ranges, ChunkRanges::all());
            assert_eq!(info.local_bytes(), total_size + hash_seq_size);
            assert!(info.is_complete());
            assert_eq!(
                info.missing(),
                GetRequest::new(content.hash, ChunkRangesSeq::empty())
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_local_info_complex_request() -> TestResult<()> {
        let sizes = INTERESTING_SIZES;
        let hash_seq_size = (sizes.len() as u64) * 32;
        let td = tempfile::tempdir()?;
        let store = FsStore::load(td.path().join("blobs.db")).await?;
        // only add the hash seq itself, and only the first chunk of the children
        let present = |i| {
            if i == 0 {
                ChunkRanges::all()
            } else {
                ChunkRanges::chunks(..2)
            }
        };
        let content = add_test_hash_seq_incomplete(&store, sizes, present).await?;
        {
            let request: GetRequest = GetRequest::builder()
                .root(ChunkRanges::all())
                .build(content.hash);
            let info = store.remote().local_for_request(request).await?;
            assert_eq!(info.bitfield.ranges, ChunkRanges::all());
            assert_eq!(info.local_bytes(), hash_seq_size);
            assert!(info.is_complete());
        }
        {
            let request: GetRequest = GetRequest::builder()
                .root(ChunkRanges::all())
                .next(ChunkRanges::all())
                .build(content.hash);
            let info = store.remote().local_for_request(request).await?;
            let expected_child_sizes = sizes
                .into_iter()
                .take(1)
                .map(|x| 1024.min(x as u64))
                .sum::<u64>();
            assert_eq!(info.bitfield.ranges, ChunkRanges::all());
            assert_eq!(info.local_bytes(), hash_seq_size + expected_child_sizes);
            assert!(info.is_complete());
        }
        {
            let request: GetRequest = GetRequest::builder()
                .root(ChunkRanges::all())
                .next(ChunkRanges::all())
                .next(ChunkRanges::all())
                .build(content.hash);
            let info = store.remote().local_for_request(request).await?;
            let expected_child_sizes = sizes
                .into_iter()
                .take(2)
                .map(|x| 1024.min(x as u64))
                .sum::<u64>();
            assert_eq!(info.bitfield.ranges, ChunkRanges::all());
            assert_eq!(info.local_bytes(), hash_seq_size + expected_child_sizes);
            assert!(info.is_complete());
        }
        {
            let request: GetRequest = GetRequest::builder()
                .root(ChunkRanges::all())
                .next(ChunkRanges::chunk(0))
                .build_open(content.hash);
            let info = store.remote().local_for_request(request).await?;
            let expected_child_sizes = sizes.into_iter().map(|x| 1024.min(x as u64)).sum::<u64>();
            assert_eq!(info.bitfield.ranges, ChunkRanges::all());
            assert_eq!(info.local_bytes(), hash_seq_size + expected_child_sizes);
            assert!(info.is_complete());
        }
        Ok(())
    }
}
