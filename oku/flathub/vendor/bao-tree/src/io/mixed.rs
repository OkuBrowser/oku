//! Read from sync, send to tokio sender
use std::{future::Future, result};

use blake3;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::{sync::Outboard, EncodeError, Leaf, Parent};
use crate::{
    iter::BaoChunk, rec::truncate_ranges, split_inner, ChunkNum, ChunkRangesRef, HashMode, TreeNode,
};

/// A content item for the bao streaming protocol.
#[derive(Debug, Serialize, Deserialize)]
pub enum EncodedItem {
    /// total data size, will be the first item
    Size(u64),
    /// a parent node
    Parent(Parent),
    /// a leaf node
    Leaf(Leaf),
    /// an error, will be the last item
    Error(EncodeError),
    /// done, will be the last item
    Done,
}

impl From<Leaf> for EncodedItem {
    fn from(l: Leaf) -> Self {
        Self::Leaf(l)
    }
}

impl From<Parent> for EncodedItem {
    fn from(p: Parent) -> Self {
        Self::Parent(p)
    }
}

impl From<EncodeError> for EncodedItem {
    fn from(e: EncodeError) -> Self {
        Self::Error(e)
    }
}

/// Abstract sender trait for sending encoded items
pub trait Sender {
    /// Error type
    type Error;
    /// Send an item
    fn send(
        &mut self,
        item: EncodedItem,
    ) -> impl Future<Output = std::result::Result<(), Self::Error>> + '_;
}

impl Sender for tokio::sync::mpsc::Sender<EncodedItem> {
    type Error = tokio::sync::mpsc::error::SendError<EncodedItem>;
    fn send(
        &mut self,
        item: EncodedItem,
    ) -> impl Future<Output = std::result::Result<(), Self::Error>> + '_ {
        tokio::sync::mpsc::Sender::send(self, item)
    }
}

/// Traverse ranges relevant to a query from a reader and outboard to a stream
///
/// This function validates the data before writing.
///
/// It is possible to encode ranges from a partial file and outboard.
/// This will either succeed if the requested ranges are all present, or fail
/// as soon as a range is missing.
pub async fn traverse_ranges_validated<D, O, F>(
    data: D,
    outboard: O,
    ranges: &ChunkRangesRef,
    send: &mut F,
) -> std::result::Result<(), F::Error>
where
    D: ReadBytesAt,
    O: Outboard,
    F: Sender,
{
    traverse_ranges_validated_impl(data, outboard, ranges, send, HashMode::Standard).await
}

/// Traverse ranges relevant to a query from a reader and keyed outboard to a stream
///
/// This function validates the data before writing, using BLAKE3 keyed hashing.
///
/// It is possible to encode ranges from a partial file and outboard.
/// This will either succeed if the requested ranges are all present, or fail
/// as soon as a range is missing.
pub async fn keyed_traverse_ranges_validated<D, O, F>(
    data: D,
    outboard: O,
    ranges: &ChunkRangesRef,
    send: &mut F,
    key: &[u8; 32],
) -> std::result::Result<(), F::Error>
where
    D: ReadBytesAt,
    O: Outboard,
    F: Sender,
{
    traverse_ranges_validated_impl(data, outboard, ranges, send, HashMode::Keyed(*key)).await
}

async fn traverse_ranges_validated_impl<D, O, F>(
    data: D,
    outboard: O,
    ranges: &ChunkRangesRef,
    send: &mut F,
    mode: HashMode,
) -> std::result::Result<(), F::Error>
where
    D: ReadBytesAt,
    O: Outboard,
    F: Sender,
{
    send.send(EncodedItem::Size(outboard.tree().size())).await?;
    let res: result::Result<std::result::Result<(), F::Error>, EncodeError> = async {
        if ranges.is_empty() {
            return Ok(Ok(()));
        }
        let mut stack: SmallVec<[_; 10]> = SmallVec::<[blake3::Hash; 10]>::new();
        stack.push(outboard.root());
        let data = data;
        let tree = outboard.tree();
        // canonicalize ranges
        let ranges = truncate_ranges(ranges, tree.size());
        for item in tree.ranges_pre_order_chunks_iter_ref(ranges, 0) {
            match item {
                BaoChunk::Parent {
                    is_root,
                    left,
                    right,
                    node,
                    ..
                } => {
                    let (l_hash, r_hash) = outboard.load(node)?.unwrap();
                    let actual = mode.parent_cv(&l_hash, &r_hash, is_root);
                    let expected = stack.pop().unwrap();
                    if actual != expected {
                        return Err(EncodeError::ParentHashMismatch(node));
                    }
                    if right {
                        stack.push(r_hash);
                    }
                    if left {
                        stack.push(l_hash);
                    }
                    let item = Parent {
                        node,
                        pair: (l_hash, r_hash),
                    };
                    if let Err(e) = send.send(item.into()).await {
                        return Ok(Err(e));
                    }
                }
                BaoChunk::Leaf {
                    start_chunk,
                    size,
                    is_root,
                    ranges,
                    ..
                } => {
                    let expected = stack.pop().unwrap();
                    let start = start_chunk.to_bytes();
                    let buffer = data.read_bytes_at(start, size)?;
                    if !ranges.is_all() {
                        // we need to encode just a part of the data
                        //
                        // write into an out buffer to ensure we detect mismatches
                        // before writing to the output.
                        let mut out_buf = Vec::new();
                        let actual = traverse_selected_rec_impl(
                            start_chunk,
                            buffer,
                            is_root,
                            ranges,
                            tree.block_size.to_u32(),
                            true,
                            &mut out_buf,
                            mode,
                        );
                        if actual != expected {
                            return Err(EncodeError::LeafHashMismatch(start_chunk));
                        }
                        for item in out_buf.into_iter() {
                            if let Err(e) = send.send(item).await {
                                return Ok(Err(e));
                            }
                        }
                    } else {
                        let actual = mode.hash_subtree(start_chunk.0, &buffer, is_root);
                        #[allow(clippy::redundant_slicing)]
                        if actual != expected {
                            return Err(EncodeError::LeafHashMismatch(start_chunk));
                        }
                        let item = Leaf {
                            data: buffer,
                            offset: start_chunk.to_bytes(),
                        };
                        if let Err(e) = send.send(item.into()).await {
                            return Ok(Err(e));
                        }
                    };
                }
            }
        }
        Ok(Ok(()))
    }
    .await;
    let res = match res {
        Ok(Ok(())) => EncodedItem::Done,
        Err(cause) => EncodedItem::Error(cause),
        Ok(Err(err)) => return Err(err),
    };
    send.send(res).await
}

/// Encode ranges relevant to a query from a slice and outboard to a buffer.
///
/// This will compute the root hash, so it will have to traverse the entire tree.
/// The `ranges` parameter just controls which parts of the data are written.
///
/// Except for writing to a buffer, this is the same as computing the subtree hash.
/// The `min_level` parameter controls the minimum level that will be emitted as a leaf.
/// Set this to 0 to disable chunk groups entirely.
/// The `emit_data` parameter controls whether the data is written to the buffer.
/// When setting this to false and setting query to `RangeSet::all()`, this can be used
/// to write an outboard.
///
/// `res` will not contain the length prefix, so if you want a bao compatible format,
/// you need to prepend it yourself.
///
/// This is used as a reference implementation in tests, but also to compute hashes
/// below the chunk group size when creating responses for outboards with a chunk group
/// size of >0.
pub fn traverse_selected_rec(
    start_chunk: ChunkNum,
    data: Bytes,
    is_root: bool,
    query: &ChunkRangesRef,
    min_level: u32,
    emit_data: bool,
    res: &mut Vec<EncodedItem>,
) -> blake3::Hash {
    traverse_selected_rec_impl(
        start_chunk,
        data,
        is_root,
        query,
        min_level,
        emit_data,
        res,
        HashMode::Standard,
    )
}

/// Keyed version of [traverse_selected_rec], using BLAKE3 keyed hashing.
#[allow(clippy::too_many_arguments)]
pub fn keyed_traverse_selected_rec(
    start_chunk: ChunkNum,
    data: Bytes,
    is_root: bool,
    query: &ChunkRangesRef,
    min_level: u32,
    emit_data: bool,
    res: &mut Vec<EncodedItem>,
    key: &[u8; 32],
) -> blake3::Hash {
    traverse_selected_rec_impl(
        start_chunk,
        data,
        is_root,
        query,
        min_level,
        emit_data,
        res,
        HashMode::Keyed(*key),
    )
}

#[allow(clippy::too_many_arguments)]
fn traverse_selected_rec_impl(
    start_chunk: ChunkNum,
    data: Bytes,
    is_root: bool,
    query: &ChunkRangesRef,
    min_level: u32,
    emit_data: bool,
    res: &mut Vec<EncodedItem>,
    mode: HashMode,
) -> blake3::Hash {
    use blake3::CHUNK_LEN;
    if data.len() <= CHUNK_LEN {
        if emit_data && !query.is_empty() {
            res.push(
                Leaf {
                    data: data.clone(),
                    offset: start_chunk.to_bytes(),
                }
                .into(),
            );
        }
        mode.hash_subtree(start_chunk.0, &data, is_root)
    } else {
        let chunks = data.len() / CHUNK_LEN + (data.len() % CHUNK_LEN != 0) as usize;
        let chunks = chunks.next_power_of_two();
        let level = chunks.trailing_zeros() - 1;
        let mid = chunks / 2;
        let mid_bytes = mid * CHUNK_LEN;
        let mid_chunk = start_chunk + (mid as u64);
        let (l_ranges, r_ranges) = split_inner(query, start_chunk, mid_chunk);
        // for empty ranges, we don't want to emit anything.
        // for full ranges where the level is below min_level, we want to emit
        // just the data.
        //
        // todo: maybe call into blake3::hazmat::hash_subtree directly for this case? it would be faster.
        let full = query.is_all();
        let emit_parent = !query.is_empty() && (!full || level >= min_level);
        let hash_offset = if emit_parent {
            // make some room for the hash pair
            let pair = Parent {
                node: TreeNode(0),
                pair: ([0; 32].into(), [0; 32].into()),
            };
            res.push(pair.into());
            Some(res.len() - 1)
        } else {
            None
        };
        // recurse to the left and right to compute the hashes and emit data
        let left = traverse_selected_rec_impl(
            start_chunk,
            data.slice(..mid_bytes),
            false,
            l_ranges,
            min_level,
            emit_data,
            res,
            mode,
        );
        let right = traverse_selected_rec_impl(
            mid_chunk,
            data.slice(mid_bytes..),
            false,
            r_ranges,
            min_level,
            emit_data,
            res,
            mode,
        );
        // backfill the hashes if needed
        if let Some(o) = hash_offset {
            // todo: figure out how to get the tree node from the start chunk!
            let node = TreeNode(0);
            res[o] = Parent {
                node,
                pair: (left, right),
            }
            .into();
        }
        mode.parent_cv(&left, &right, is_root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        io::{
            outboard::PreOrderMemOutboard,
            sync::{encode_ranges_validated, keyed_encode_ranges_validated},
        },
        BlockSize, ChunkRanges,
    };

    fn flatten(items: Vec<EncodedItem>) -> Vec<u8> {
        let mut res = Vec::new();
        for item in items {
            match item {
                EncodedItem::Leaf(Leaf { data, .. }) => res.extend_from_slice(&data),
                EncodedItem::Parent(Parent { pair: (l, r), .. }) => {
                    res.extend_from_slice(l.as_bytes());
                    res.extend_from_slice(r.as_bytes());
                }
                _ => {}
            }
        }
        res
    }

    #[tokio::test]
    async fn smoke() {
        let data = [0u8; 100000];
        let outboard = PreOrderMemOutboard::create(data, BlockSize::from_chunk_log(4));
        let (mut tx, mut rx) = tokio::sync::mpsc::channel(10);
        let mut encoded = Vec::new();
        encode_ranges_validated(&data[..], &outboard, &ChunkRanges::empty(), &mut encoded).unwrap();
        tokio::spawn(async move {
            traverse_ranges_validated(&data[..], &outboard, &ChunkRanges::empty(), &mut tx)
                .await
                .unwrap();
        });
        let mut res = Vec::new();
        while let Some(item) = rx.recv().await {
            res.push(item);
        }
        println!("{res:?}");
        let encoded2 = flatten(res);
        assert_eq!(encoded, encoded2);
    }

    #[tokio::test]
    async fn keyed_smoke() {
        let data = [0u8; 100000];
        let key = blake3::derive_key("bao-tree.test", b"mixed");
        let outboard = PreOrderMemOutboard::create_keyed(data, BlockSize::from_chunk_log(4), &key);
        let (mut tx, mut rx) = tokio::sync::mpsc::channel(10);
        let mut encoded = Vec::new();
        keyed_encode_ranges_validated(
            &data[..],
            &outboard,
            &ChunkRanges::all(),
            &mut encoded,
            &key,
        )
        .unwrap();
        tokio::spawn(async move {
            keyed_traverse_ranges_validated(
                &data[..],
                &outboard,
                &ChunkRanges::all(),
                &mut tx,
                &key,
            )
            .await
            .unwrap();
        });
        let mut res = Vec::new();
        while let Some(item) = rx.recv().await {
            res.push(item);
        }
        let encoded2 = flatten(res);
        assert_eq!(encoded, encoded2);
    }
}

/// Trait identical to `ReadAt` but returning `Bytes` instead of reading into a buffer.
///
/// This forwards to the underlying `ReadAt` implementation except for `Bytes`, `&Bytes`, `&mut Bytes`.
pub trait ReadBytesAt {
    /// Version of `ReadAt::read_exact_at` that returns a `Bytes` instead of reading into a buffer.
    fn read_bytes_at(&self, offset: u64, size: usize) -> std::io::Result<Bytes>;
}

mod impls {
    use std::io;

    use bytes::Bytes;

    use super::ReadBytesAt;

    // Macro for generic implementations (allocating with copy_from_slice)
    macro_rules! impl_read_bytes_at_generic {
    ($($t:ty),*) => {
        $(
            impl ReadBytesAt for $t {
                fn read_bytes_at(&self, offset: u64, size: usize) -> io::Result<Bytes> {
                    let mut buf = vec![0; size];
                    ::positioned_io::ReadAt::read_exact_at(self, offset, &mut buf)?;
                    Ok(buf.into())
                }
            }
        )*
    };
}

    // Macro for special implementations (non-allocating with slice)
    macro_rules! impl_read_bytes_at_special {
    ($($t:ty),*) => {
        $(
            impl ReadBytesAt for $t {
                fn read_bytes_at(&self, offset: u64, size: usize) -> io::Result<Bytes> {
                    let offset = offset as usize;
                    if offset + size > self.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Read past end of buffer",
                        ));
                    }
                    Ok(self.slice(offset..offset + size))
                }
            }
        )*
    };
}

    // Apply the macros
    impl_read_bytes_at_generic!(&[u8], Vec<u8>);
    impl_read_bytes_at_special!(Bytes, &Bytes, &mut Bytes);
    #[cfg(feature = "fs")]
    impl_read_bytes_at_generic!(std::fs::File);
}
