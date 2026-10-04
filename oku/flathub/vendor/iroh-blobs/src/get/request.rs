//! Utilities to generate or execute complex get requests without persisting to a store.
//!
//! Any complex request can be executed with downloading to a store, using the
//! [`crate::api::remote::Remote::execute_get`] method. But for some requests it
//! is useful to just get the data without persisting it to a store.
//!
//! In addition to these utilities, there are also constructors in [`crate::protocol::ChunkRangesSeq`]
//! to construct complex requests.
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bao_tree::{io::BaoContentItem, ChunkNum, ChunkRanges};
use bytes::Bytes;
use genawaiter::sync::{Co, Gen};
use iroh::endpoint::Connection;
use n0_error::e;
use n0_future::{Stream, StreamExt};
use nested_enum_utils::enum_conversions;
use rand::{Rng, RngExt};
use tokio::sync::mpsc;

use super::{fsm, GetError, GetResult, Stats};
use crate::{
    hashseq::HashSeq,
    protocol::{ChunkRangesExt, ChunkRangesSeq, GetRequest},
    Hash, HashAndFormat,
};

/// Result of a [`get_blob`] request.
///
/// This is a stream of [`GetBlobItem`]s. You can also await it to get just
/// the bytes of the blob.
pub struct GetBlobResult {
    rx: n0_future::stream::Boxed<GetBlobItem>,
}

impl IntoFuture for GetBlobResult {
    type Output = GetResult<Bytes>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.bytes())
    }
}

impl GetBlobResult {
    pub async fn bytes(self) -> GetResult<Bytes> {
        let (bytes, _) = self.bytes_and_stats().await?;
        Ok(bytes)
    }

    pub async fn bytes_and_stats(mut self) -> GetResult<(Bytes, Stats)> {
        let mut parts = Vec::new();
        let stats = loop {
            let Some(item) = self.next().await else {
                return Err(e!(
                    GetError::LocalFailure,
                    n0_error::anyerr!("unexpected end")
                ));
            };
            match item {
                GetBlobItem::Item(item) => {
                    if let BaoContentItem::Leaf(leaf) = item {
                        parts.push(leaf.data);
                    }
                }
                GetBlobItem::Done(stats) => {
                    break stats;
                }
                GetBlobItem::Error(cause) => {
                    return Err(cause);
                }
            }
        };
        let bytes = if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            let mut bytes = Vec::new();
            for part in parts {
                bytes.extend_from_slice(&part);
            }
            bytes.into()
        };
        Ok((bytes, stats))
    }
}

impl Stream for GetBlobResult {
    type Item = GetBlobItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.rx.poll_next(cx)
    }
}

/// A single item in a [`GetBlobResult`].
#[derive(Debug)]
#[enum_conversions()]
pub enum GetBlobItem {
    /// Content
    Item(BaoContentItem),
    /// Request completed successfully
    Done(Stats),
    /// Request failed
    Error(GetError),
}

pub fn get_blob(connection: Connection, hash: Hash) -> GetBlobResult {
    let generator = Gen::new(|co| async move {
        if let Err(cause) = get_blob_impl(&connection, &hash, &co).await {
            co.yield_(GetBlobItem::Error(cause)).await;
        }
    });
    GetBlobResult {
        rx: Box::pin(generator),
    }
}

async fn get_blob_impl(
    connection: &Connection,
    hash: &Hash,
    co: &Co<GetBlobItem>,
) -> GetResult<()> {
    let request = GetRequest::blob(*hash);
    let request = fsm::start(connection.clone(), request, Default::default());
    let connected = request.next().await?;
    let fsm::ConnectedNext::StartRoot(start) = connected.next().await? else {
        unreachable!("expected start root");
    };
    let header = start.next();
    let (mut curr, _size) = header.next().await?;
    let end = loop {
        match curr.next().await {
            fsm::BlobContentNext::More((next, res)) => {
                co.yield_(res?.into()).await;
                curr = next;
            }
            fsm::BlobContentNext::Done(end) => {
                break end;
            }
        }
    };
    let fsm::EndBlobNext::Closing(closing) = end.next() else {
        unreachable!("expected closing");
    };
    let stats = closing.next().await?;
    co.yield_(stats.into()).await;
    Ok(())
}

/// Get the claimed size of a blob from a peer.
///
/// This is just reading the size header and then immediately closing the connection.
/// It can be used to check if a peer has any data at all.
pub async fn get_unverified_size(connection: &Connection, hash: &Hash) -> GetResult<(u64, Stats)> {
    let request = GetRequest::new(
        *hash,
        ChunkRangesSeq::from_ranges(vec![ChunkRanges::last_chunk()]),
    );
    let request = fsm::start(connection.clone(), request, Default::default());
    let connected = request.next().await?;
    let fsm::ConnectedNext::StartRoot(start) = connected.next().await? else {
        unreachable!("expected start root");
    };
    let at_blob_header = start.next();
    let (curr, size) = at_blob_header.next().await?;
    let stats = curr.finish().next().await?;
    Ok((size, stats))
}

/// Get the verified size of a blob from a peer.
///
/// This asks for the last chunk of the blob and validates the response.
/// Note that this does not validate that the peer has all the data.
pub async fn get_verified_size(connection: &Connection, hash: &Hash) -> GetResult<(u64, Stats)> {
    tracing::trace!("Getting verified size of {}", hash.to_hex());
    let request = GetRequest::new(
        *hash,
        ChunkRangesSeq::from_ranges(vec![ChunkRanges::last_chunk()]),
    );
    let request = fsm::start(connection.clone(), request, Default::default());
    let connected = request.next().await?;
    let fsm::ConnectedNext::StartRoot(start) = connected.next().await? else {
        unreachable!("expected start root");
    };
    let header = start.next();
    let (mut curr, size) = header.next().await?;
    let end = loop {
        match curr.next().await {
            fsm::BlobContentNext::More((next, res)) => {
                let _ = res?;
                curr = next;
            }
            fsm::BlobContentNext::Done(end) => {
                break end;
            }
        }
    };
    let fsm::EndBlobNext::Closing(closing) = end.next() else {
        unreachable!("expected closing");
    };
    let stats = closing.next().await?;
    tracing::trace!(
        "Got verified size of {}, {:.6}s",
        hash.to_hex(),
        stats.elapsed.as_secs_f64()
    );
    Ok((size, stats))
}

/// Given a hash of a hash seq, get the hash seq and the verified sizes of its
/// children.
///
/// If this operation succeeds we have a strong indication that the peer has
/// the hash seq and the last chunk of each child.
///
/// This can be used to compute the total size when requesting a hash seq.
pub async fn get_hash_seq_and_sizes(
    connection: &Connection,
    hash: &Hash,
    max_size: u64,
    _progress: Option<mpsc::Sender<u64>>,
) -> GetResult<(HashSeq, Arc<[u64]>)> {
    let content = HashAndFormat::hash_seq(*hash);
    tracing::debug!("Getting hash seq and children sizes of {}", content);
    let request = GetRequest::new(
        *hash,
        ChunkRangesSeq::from_ranges_infinite([ChunkRanges::all(), ChunkRanges::last_chunk()]),
    );
    let at_start = fsm::start(connection.clone(), request, Default::default());
    let at_connected = at_start.next().await?;
    let fsm::ConnectedNext::StartRoot(start) = at_connected.next().await? else {
        unreachable!("query includes root");
    };
    let at_start_root = start.next();
    let (at_blob_content, size) = at_start_root.next().await?;
    // check the size to avoid parsing a maliciously large hash seq
    if size > max_size {
        return Err(e!(
            GetError::BadRequest,
            n0_error::anyerr!("size too large")
        ));
    }
    let (mut curr, hash_seq) = at_blob_content.concatenate_into_vec().await?;
    let hash_seq =
        HashSeq::try_from(Bytes::from(hash_seq)).map_err(|e| e!(GetError::BadRequest, e))?;
    let mut sizes = Vec::with_capacity(hash_seq.len());
    let closing = loop {
        match curr.next() {
            fsm::EndBlobNext::MoreChildren(more) => {
                let hash = match hash_seq.get(sizes.len()) {
                    Some(hash) => hash,
                    None => break more.finish(),
                };
                let at_header = more.next(hash);
                let (at_content, size) = at_header.next().await?;
                let next = at_content.drain().await?;
                sizes.push(size);
                curr = next;
            }
            fsm::EndBlobNext::Closing(closing) => break closing,
        }
    };
    let _stats = closing.next().await?;
    tracing::debug!(
        "Got hash seq and children sizes of {}: {:?}",
        content,
        sizes
    );
    Ok((hash_seq, sizes.into()))
}

/// Probe for a single chunk of a blob.
///
/// This is used to check if a peer has a specific chunk. If the operation
/// is successful, we have a strong indication that the peer had the chunk at
/// the time of the request.
///
/// If the operation fails, either the connection failed or the peer did not
/// have the chunk.
///
/// It is usually not very helpful to try to distinguish between these two
/// cases.
pub async fn get_chunk_probe(
    connection: &Connection,
    hash: &Hash,
    chunk: ChunkNum,
) -> GetResult<Stats> {
    let ranges = ChunkRanges::from(chunk..chunk + 1);
    let ranges = ChunkRangesSeq::from_ranges([ranges]);
    let request = GetRequest::new(*hash, ranges);
    let request = fsm::start(connection.clone(), request, Default::default());
    let connected = request.next().await?;
    let fsm::ConnectedNext::StartRoot(start) = connected.next().await? else {
        unreachable!("query includes root");
    };
    let header = start.next();
    let (mut curr, _size) = header.next().await?;
    let end = loop {
        match curr.next().await {
            fsm::BlobContentNext::More((next, res)) => {
                res?;
                curr = next;
            }
            fsm::BlobContentNext::Done(end) => {
                break end;
            }
        }
    };
    let fsm::EndBlobNext::Closing(closing) = end.next() else {
        unreachable!("query contains only one blob");
    };
    let stats = closing.next().await?;
    Ok(stats)
}

/// Given a sequence of sizes of children, generate a range spec that selects a
/// random chunk of a random child.
///
/// The random chunk is chosen uniformly from the chunks of the children, so
/// larger children are more likely to be selected.
pub fn random_hash_seq_ranges(sizes: &[u64], mut rng: impl Rng) -> ChunkRangesSeq {
    let total_chunks = sizes
        .iter()
        .map(|size| ChunkNum::full_chunks(*size).0)
        .sum::<u64>();
    let random_chunk = rng.random_range(0..total_chunks);
    let mut remaining = random_chunk;
    let mut ranges = vec![];
    ranges.push(ChunkRanges::empty());
    for size in sizes.iter() {
        let chunks = ChunkNum::full_chunks(*size).0;
        if remaining < chunks {
            ranges.push(ChunkRanges::from(
                ChunkNum(remaining)..ChunkNum(remaining + 1),
            ));
            break;
        } else {
            remaining -= chunks;
            ranges.push(ChunkRanges::empty());
        }
    }
    ChunkRangesSeq::from_ranges(ranges)
}
