//! recursive reference implementation of bao encoding and decoding
//!
//! Encocding is used to compute hashes, decoding is only used in tests as a
//! reference implementation.
use crate::{blake3, split_inner, ChunkNum, ChunkRangesRef};

/// Given a set of chunk ranges, adapt them for a tree of the given size.
///
/// This will consider anything behind the tree size to be a request for the last chunk,
/// which makes things a bit more complex. This is useful to get a size proof for a
/// blob of an unknown size.
///
/// If you don't need this, you can just split the ranges on the tree size and then
/// keep the first part.
///
/// Examples:
///
/// assuming a size of 7 chunks:
///
/// 0..6 will remain 0..6
/// 0..7 will become 0.. (the entire blob)
/// 0..10, 11.12 will become 0.. (the entire blob)
/// 0..6, 7..10 will become 0.. (the entire blob). the last chunk will be included and the hole filled
/// 3..6, 7..10 will become 3.. the last chunk will be included and the hole filled
/// 0..5, 7..10 will become 0..5, 7.. (the last chunk will be included, but chunkk 5 will not be sent)
pub fn truncate_ranges(ranges: &ChunkRangesRef, size: u64) -> &ChunkRangesRef {
    let bs = ranges.boundaries();
    ChunkRangesRef::new_unchecked(&bs[..truncated_len(ranges, size)])
}

/// A version of [canonicalize_ranges] that takes and returns an owned [ChunkRanges].
///
/// This is needed for the state machines that own their ranges.
#[cfg(feature = "tokio_fsm")]
pub fn truncate_ranges_owned(ranges: crate::ChunkRanges, size: u64) -> crate::ChunkRanges {
    let n = truncated_len(&ranges, size);
    let mut boundaries = ranges.into_inner();
    boundaries.truncate(n);
    crate::ChunkRanges::new_unchecked(boundaries)
}

fn truncated_len(ranges: &ChunkRangesRef, size: u64) -> usize {
    let end = ChunkNum::chunks(size);
    let lc = ChunkNum(end.0.saturating_sub(1));
    let bs = ranges.boundaries();
    match bs.binary_search(&lc) {
        Ok(i) if (i & 1) == 0 => {
            // last chunk is included and is a start boundary.
            // keep it and remove the rest.
            i + 1
        }
        Ok(i) => {
            if bs.len() == i + 1 {
                // last chunk is included and is an end boundary, and there is nothing behind it.
                // nothing to do.
                i + 1
            } else {
                // last chunk is included and is an end boundary, and there is something behind it.
                // remove it to turn the range into an open range.
                i
            }
        }
        Err(ip) if (ip & 1) == 0 => {
            // insertion point would be a start boundary.
            if bs.len() == ip {
                // the insertion point is at the end, nothing to do.
                ip
            } else {
                // include one start boundary > lc to turn the range into an open range.
                ip + 1
            }
        }
        Err(ip) => {
            // insertion point is an end boundary
            // the value at ip must be > lc, so we can just omit it
            ip
        }
    }
}

/// Encode ranges relevant to a query from a slice and outboard to a buffer.
///
/// This will compute the root hash, so it will have to traverse the entire tree.
/// The `ranges` parameter just controls which parts of the data are written.
///
/// Except for writing to a buffer, this is the same as [hash_subtree].
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
///
/// `mode` is the hashing mode for subtree and parent hashes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_selected_rec(
    start_chunk: ChunkNum,
    data: &[u8],
    is_root: bool,
    query: &ChunkRangesRef,
    min_level: u32,
    emit_data: bool,
    res: &mut Vec<u8>,
    mode: crate::HashMode,
) -> blake3::Hash {
    use blake3::CHUNK_LEN;
    if data.len() <= CHUNK_LEN {
        if emit_data && !query.is_empty() {
            res.extend_from_slice(data);
        }
        mode.hash_subtree(start_chunk.0, data, is_root)
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
            // make some room for the hashes
            res.extend_from_slice(&[0xFFu8; 64]);
            Some(res.len() - 64)
        } else {
            None
        };
        // recurse to the left and right to compute the hashes and emit data
        let left = encode_selected_rec(
            start_chunk,
            &data[..mid_bytes],
            false,
            l_ranges,
            min_level,
            emit_data,
            res,
            mode,
        );
        let right = encode_selected_rec(
            mid_chunk,
            &data[mid_bytes..],
            false,
            r_ranges,
            min_level,
            emit_data,
            res,
            mode,
        );
        // backfill the hashes if needed
        if let Some(o) = hash_offset {
            res[o..o + 32].copy_from_slice(left.as_bytes());
            res[o + 32..o + 64].copy_from_slice(right.as_bytes());
        }
        mode.parent_cv(&left, &right, is_root)
    }
}

#[cfg(test)]
mod test_support {
    #[cfg(feature = "tokio_fsm")]
    use {
        crate::{split_inner, TreeNode},
        range_collections::{range_set::RangeSetEntry, RangeSet2},
        std::ops::Range,
    };

    use super::{encode_selected_rec, truncate_ranges};
    use crate::{blake3, BaoChunk, BaoTree, BlockSize, ChunkNum, ChunkRanges, ChunkRangesRef};

    /// Select nodes relevant to a query
    ///
    /// This is the receive side equivalent of [bao_encode_selected_recursive].
    /// It does not do any hashing, but just emits the nodes that are relevant to a query.
    ///
    /// The tree is given as `start_chunk`, `size` and `is_root`.
    ///
    /// To traverse an entire tree, use `0` for `start_chunk`, `tree.size` for `size` and
    /// `true` for `is_root`.
    ///
    /// `tree_level` is the smallest level the iterator will emit. Set this to `tree.block_size`
    /// `min_full_level` is the smallest level that will be emitted as a leaf if it is fully
    /// within the query range.
    ///
    /// To disable chunk groups entirely, just set both `tree_level` and `min_full_level` to 0.
    #[cfg(feature = "tokio_fsm")]
    pub(crate) fn select_nodes_rec<'a>(
        start_chunk: ChunkNum,
        size: usize,
        is_root: bool,
        ranges: &'a ChunkRangesRef,
        tree_level: u32,
        min_full_level: u32,
        emit: &mut impl FnMut(BaoChunk<&'a ChunkRangesRef>),
    ) {
        if ranges.is_empty() {
            return;
        }
        use blake3::CHUNK_LEN;

        if size <= CHUNK_LEN {
            emit(BaoChunk::Leaf {
                start_chunk,
                size,
                is_root,
                ranges,
            });
        } else {
            let chunks: usize = size / CHUNK_LEN + (size % CHUNK_LEN != 0) as usize;
            let chunks = chunks.next_power_of_two();
            // chunks is always a power of two, 2 for level 0
            // so we must subtract 1 to get the level, and this is also safe
            let level = chunks.trailing_zeros() - 1;
            let full = ranges.is_all();
            if (level < tree_level) || (full && level < min_full_level) {
                // we are allowed to just emit the entire data as a leaf
                emit(BaoChunk::Leaf {
                    start_chunk,
                    size,
                    is_root,
                    ranges,
                });
            } else {
                // split in half and recurse
                assert!(start_chunk.0 % 2 == 0);
                let mid = chunks / 2;
                let mid_bytes = mid * CHUNK_LEN;
                let mid_chunk = start_chunk + (mid as u64);
                let (l_ranges, r_ranges) = split_inner(ranges, start_chunk, mid_chunk);
                let node =
                    TreeNode::from_start_chunk_and_level(start_chunk, BlockSize(level as u8));
                emit(BaoChunk::Parent {
                    node,
                    is_root,
                    left: !l_ranges.is_empty(),
                    right: !r_ranges.is_empty(),
                    ranges,
                });
                // recurse to the left and right to compute the hashes and emit data
                select_nodes_rec(
                    start_chunk,
                    mid_bytes,
                    false,
                    l_ranges,
                    tree_level,
                    min_full_level,
                    emit,
                );
                select_nodes_rec(
                    mid_chunk,
                    size - mid_bytes,
                    false,
                    r_ranges,
                    tree_level,
                    min_full_level,
                    emit,
                );
            }
        }
    }

    pub(crate) fn bao_outboard_reference(data: &[u8]) -> (Vec<u8>, blake3::Hash) {
        let mut res = Vec::new();
        res.extend_from_slice(&(data.len() as u64).to_le_bytes());
        let hash = encode_selected_rec(
            ChunkNum(0),
            data,
            true,
            &ChunkRanges::all(),
            0,
            false,
            &mut res,
            crate::HashMode::Standard,
        );
        (res, hash)
    }

    pub(crate) fn bao_encode_reference(data: &[u8]) -> (Vec<u8>, blake3::Hash) {
        let mut res = Vec::new();
        res.extend_from_slice(&(data.len() as u64).to_le_bytes());
        let hash = encode_selected_rec(
            ChunkNum(0),
            data,
            true,
            &ChunkRanges::all(),
            0,
            true,
            &mut res,
            crate::HashMode::Standard,
        );
        (res, hash)
    }

    /// Reference implementation of the response iterator, using just the simple recursive
    /// implementation [select_nodes_rec].
    #[cfg(feature = "tokio_fsm")]
    pub(crate) fn partial_chunk_iter_reference(
        tree: BaoTree,
        ranges: &ChunkRangesRef,
        min_full_level: u8,
    ) -> Vec<BaoChunk<&ChunkRangesRef>> {
        let mut res = Vec::new();
        select_nodes_rec(
            ChunkNum(0),
            tree.size.try_into().unwrap(),
            true,
            ranges,
            tree.block_size.to_u32(),
            min_full_level as u32,
            &mut |x| res.push(x),
        );
        res
    }

    /// Reference implementation of the response iterator, using just the simple recursive
    /// implementation [select_nodes_rec].
    #[cfg(feature = "tokio_fsm")]
    pub(crate) fn response_iter_reference(tree: BaoTree, ranges: &ChunkRangesRef) -> Vec<BaoChunk> {
        let mut res = Vec::new();
        select_nodes_rec(
            ChunkNum(0),
            tree.size.try_into().unwrap(),
            true,
            ranges,
            0,
            tree.block_size.to_u32(),
            &mut |x| res.push(x.without_ranges()),
        );
        res
    }

    /// A reference implementation of [PreOrderPartialChunkIterRef] that just uses the recursive
    /// implementation [crate::iter::select_nodes_rec].
    ///
    /// This is not a proper iterator since it computes all elements at once, but it is useful for
    /// testing.
    #[derive(Debug)]
    pub struct ReferencePreOrderPartialChunkIterRef<'a> {
        iter: std::vec::IntoIter<BaoChunk<&'a ChunkRangesRef>>,
        tree: BaoTree,
    }

    impl<'a> ReferencePreOrderPartialChunkIterRef<'a> {
        /// Create a new iterator over the tree.
        #[cfg(feature = "tokio_fsm")]
        pub fn new(tree: BaoTree, ranges: &'a ChunkRangesRef, min_full_level: u8) -> Self {
            let iter = partial_chunk_iter_reference(tree, ranges, min_full_level).into_iter();
            Self { iter, tree }
        }

        /// Return a reference to the underlying tree.
        #[allow(dead_code)]
        pub fn tree(&self) -> &BaoTree {
            &self.tree
        }
    }

    impl<'a> Iterator for ReferencePreOrderPartialChunkIterRef<'a> {
        type Item = BaoChunk<&'a ChunkRangesRef>;

        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
        }
    }

    /// Make test data that has each chunk filled with the cunk number as an u8
    ///
    /// This makes sure that every chunk has a different hash, and makes it easy
    /// to see pages in hex dumps.
    pub(crate) fn make_test_data(n: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(n);
        for i in 0..n {
            data.push((i / 1024) as u8);
        }
        data
    }

    /// Compute the union of an iterator of ranges. The ranges should be non-overlapping, otherwise
    /// the result is None
    #[cfg(feature = "tokio_fsm")]
    pub fn range_union<K: RangeSetEntry>(
        ranges: impl IntoIterator<Item = Range<K>>,
    ) -> Option<RangeSet2<K>> {
        let mut res = RangeSet2::empty();
        for r in ranges.into_iter() {
            let part = RangeSet2::from(r);
            if part.intersects(&res) {
                return None;
            }
            res |= part;
        }
        Some(res)
    }

    #[cfg(feature = "tokio_fsm")]
    pub fn get_leaf_ranges<R>(
        iter: impl IntoIterator<Item = BaoChunk<R>>,
    ) -> impl Iterator<Item = Range<u64>> {
        iter.into_iter().filter_map(|e| {
            if let BaoChunk::Leaf {
                start_chunk, size, ..
            } = e
            {
                let start = start_chunk.to_bytes();
                let end = start + (size as u64);
                Some(start..end)
            } else {
                None
            }
        })
    }

    pub fn encode_ranges_reference(
        data: &[u8],
        ranges: &ChunkRangesRef,
        block_size: BlockSize,
    ) -> (Vec<u8>, blake3::Hash) {
        let mut res = Vec::new();
        let size = data.len() as u64;
        // canonicalize the ranges
        let ranges = truncate_ranges(ranges, size);
        let hash = encode_selected_rec(
            ChunkNum(0),
            data,
            true,
            ranges,
            block_size.to_u32(),
            true,
            &mut res,
            crate::HashMode::Standard,
        );
        (res, hash)
    }

    use std::io::Cursor;

    use crate::io::outboard::{PostOrderMemOutboard, PreOrderMemOutboard, PreOrderOutboard};
    use crate::io::sync;

    pub(crate) fn assert_pre_order_outboard_matches_mem(
        outboard: &PreOrderOutboard<Vec<u8>>,
        data: &[u8],
        block_size: BlockSize,
        key: &[u8; 32],
    ) {
        let reference = PreOrderMemOutboard::create_keyed(data, block_size, key);
        assert_eq!(outboard.root, reference.root);
        let tree = outboard.tree;
        let mut copied = PreOrderMemOutboard {
            root: outboard.root,
            tree,
            data: vec![0; tree.outboard_size().try_into().unwrap()],
        };
        sync::copy(outboard, &mut copied).unwrap();
        assert_eq!(copied.data, reference.data);
    }

    pub(crate) fn keyed_outboard_functions_checks(
        data: &[u8],
        block_size: BlockSize,
        key: &[u8; 32],
    ) {
        let tree = BaoTree::new(data.len() as u64, block_size);
        let expected = blake3::keyed_hash(key, data);

        let mut pre = PreOrderOutboard {
            root: blake3::Hash::from([0; 32]),
            tree,
            data: vec![0; tree.outboard_size().try_into().unwrap()],
        };
        let root = sync::keyed_outboard(Cursor::new(data), tree, &mut pre, key).unwrap();
        pre.root = root;
        assert_eq!(root, expected);
        assert_pre_order_outboard_matches_mem(&pre, data, block_size, key);

        let mut post_buf = Vec::new();
        let root =
            sync::keyed_outboard_post_order(Cursor::new(data), tree, &mut post_buf, key).unwrap();
        assert_eq!(root, expected);
        assert_eq!(post_buf.len(), tree.outboard_size().try_into().unwrap());

        let reference_post = PostOrderMemOutboard::create_keyed(data, block_size, key);
        assert_eq!(post_buf, reference_post.data);

        let post_mem = PostOrderMemOutboard {
            root,
            tree,
            data: post_buf,
        };
        let pre_from_post = post_mem.flip();
        assert_eq!(pre_from_post.data, pre.data);

        let truncated_size = 1024u64.min(data.len() as u64);
        if truncated_size < data.len() as u64 {
            let truncated_tree = BaoTree::new(truncated_size, BlockSize(0));
            let mut truncated_pre = PreOrderOutboard {
                root: blake3::Hash::from([0; 32]),
                tree: truncated_tree,
                data: vec![0; truncated_tree.outboard_size().try_into().unwrap()],
            };
            let root =
                sync::keyed_outboard(Cursor::new(data), truncated_tree, &mut truncated_pre, key)
                    .unwrap();
            truncated_pre.root = root;
            assert_eq!(
                root,
                blake3::keyed_hash(key, &data[..truncated_size as usize])
            );
            assert_pre_order_outboard_matches_mem(
                &truncated_pre,
                &data[..truncated_size as usize],
                BlockSize(0),
                key,
            );
        }

        let oversize_tree = BaoTree::new(data.len() as u64 + 100, BlockSize(0));
        let mut sink = Vec::new();
        assert!(
            sync::keyed_outboard_post_order(Cursor::new(data), oversize_tree, &mut sink, key)
                .is_err()
        );
    }

    #[cfg(feature = "tokio_fsm")]
    pub(crate) async fn keyed_outboard_functions_checks_fsm(
        data: &[u8],
        block_size: BlockSize,
        key: &[u8; 32],
    ) {
        use crate::io::fsm::{keyed_outboard, keyed_outboard_post_order};
        use bytes::Bytes;

        let tree = BaoTree::new(data.len() as u64, block_size);
        let expected = blake3::keyed_hash(key, data);

        let mut pre = PreOrderOutboard {
            root: blake3::Hash::from([0; 32]),
            tree,
            data: vec![0; tree.outboard_size().try_into().unwrap()],
        };
        let root = keyed_outboard(Cursor::new(Bytes::from(data.to_vec())), tree, &mut pre, key)
            .await
            .unwrap();
        pre.root = root;
        assert_eq!(root, expected);
        assert_pre_order_outboard_matches_mem(&pre, data, block_size, key);

        let mut post_buf = Vec::new();
        let root = keyed_outboard_post_order(
            Cursor::new(Bytes::from(data.to_vec())),
            tree,
            &mut post_buf,
            key,
        )
        .await
        .unwrap();
        assert_eq!(root, expected);
        assert_eq!(post_buf.len(), tree.outboard_size().try_into().unwrap());

        let reference_post = PostOrderMemOutboard::create_keyed(data, block_size, key);
        assert_eq!(post_buf, reference_post.data);

        let post_mem = PostOrderMemOutboard {
            root,
            tree,
            data: post_buf,
        };
        let pre_from_post = post_mem.flip();
        assert_eq!(pre_from_post.data, pre.data);

        let truncated_size = 1024u64.min(data.len() as u64);
        if truncated_size < data.len() as u64 {
            let truncated_tree = BaoTree::new(truncated_size, BlockSize(0));
            let mut truncated_pre = PreOrderOutboard {
                root: blake3::Hash::from([0; 32]),
                tree: truncated_tree,
                data: vec![0; truncated_tree.outboard_size().try_into().unwrap()],
            };
            let root = keyed_outboard(
                Cursor::new(Bytes::from(data.to_vec())),
                truncated_tree,
                &mut truncated_pre,
                key,
            )
            .await
            .unwrap();
            truncated_pre.root = root;
            assert_eq!(
                root,
                blake3::keyed_hash(key, &data[..truncated_size as usize])
            );
            assert_pre_order_outboard_matches_mem(
                &truncated_pre,
                &data[..truncated_size as usize],
                BlockSize(0),
                key,
            );
        }

        let oversize_tree = BaoTree::new(data.len() as u64 + 100, BlockSize(0));
        let mut sink = Vec::new();
        assert!(keyed_outboard_post_order(
            Cursor::new(Bytes::from(data.to_vec())),
            oversize_tree,
            &mut sink,
            key
        )
        .await
        .is_err());
    }

    /// Check that l and r of a 2-tuple are equal
    #[macro_export]
    macro_rules! assert_tuple_eq {
        ($tuple:expr) => {
            assert_eq!($tuple.0, $tuple.1);
        };
    }

    /// Check that l and r of a 2-tuple are equal
    #[macro_export]
    macro_rules! prop_assert_tuple_eq {
        ($tuple:expr) => {
            let (a, b) = $tuple;
            ::proptest::prop_assert_eq!(a, b);
        };
    }
}
#[cfg(test)]
pub use test_support::*;

/// These tests are not really tests for the crate itself, but tests for the reference
/// implementation that compare it to the bao crate.
#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Read},
        ops::Range,
    };

    use proptest::prelude::*;

    use crate::{
        rec::{
            bao_encode_reference, bao_outboard_reference, encode_ranges_reference, make_test_data,
        },
        BlockSize, ChunkNum, ChunkRanges,
    };

    fn size_and_slice() -> impl Strategy<Value = (usize, Range<usize>)> {
        (1..100000usize)
            .prop_flat_map(|len| (Just(len), (0..len), (0..len)))
            .prop_map(|(len, a, b)| {
                let start = a.min(b);
                let end = a.max(b);
                (len, start..end)
            })
    }

    fn size_and_start() -> impl Strategy<Value = (usize, usize)> {
        (1..100000usize).prop_flat_map(|len| (Just(len), (0..len)))
    }

    /// Compare the outboard from the bao crate to the outboard from the reference implementation
    ///
    /// Since bao does not support chunk groups, this can only be done for chunk group size 0.
    #[test_strategy::proptest]
    fn bao_outboard_comparison(#[strategy(0usize..100000)] size: usize) {
        let data = make_test_data(size);
        let (expected_outboard, expected_hash) = bao::encode::outboard(&data);
        let (actual_outboard, actual_hash) = bao_outboard_reference(&data);
        prop_assert_eq!(expected_outboard, actual_outboard);
        prop_assert_eq!(expected_hash.as_bytes(), actual_hash.as_bytes());
    }

    /// Compare the encoded from the bao crate to the encoded from the reference implementation
    ///
    /// Since bao does not support chunk groups, this can only be done for chunk group size 0.
    #[test_strategy::proptest]
    fn bao_encode_comparison(#[strategy(0usize..100000)] size: usize) {
        let data = make_test_data(size);
        let (expected_encoded, expected_hash) = bao::encode::encode(&data);
        let (actual_encoded, actual_hash) = bao_encode_reference(&data);
        prop_assert_eq!(expected_encoded, actual_encoded);
        prop_assert_eq!(expected_hash.as_bytes(), actual_hash.as_bytes());
    }

    /// Compare a random encoded slice from the bao crate to the encoded from the reference implementation
    ///
    /// Since bao does not support chunk groups, this can only be done for chunk group size 0.
    #[test_strategy::proptest]
    fn bao_encode_slice_comparison(
        #[strategy(size_and_slice())] size_and_slice: (usize, Range<usize>),
    ) {
        let (size, Range { start, end }) = size_and_slice;
        let data = make_test_data(size);
        let (outboard, _) = bao::encode::outboard(&data);
        let mut encoder = bao::encode::SliceExtractor::new_outboard(
            Cursor::new(&data),
            Cursor::new(&outboard),
            start as u64,
            (end - start) as u64,
        );
        let mut expected_encoded = Vec::new();
        encoder.read_to_end(&mut expected_encoded).unwrap();
        let chunk_start = ChunkNum::full_chunks(start as u64);
        let chunk_end = ChunkNum::chunks(end as u64).max(chunk_start + 1);
        let ranges = ChunkRanges::from(chunk_start..chunk_end);
        let mut actual_encoded = encode_ranges_reference(&data, &ranges, BlockSize::ZERO).0;
        actual_encoded.splice(..0, size.to_le_bytes().into_iter());
        prop_assert_eq!(expected_encoded, actual_encoded);
    }

    /// Decode a single chunk with the reference encode impl [encode_ranges_reference] and
    /// decode it using the bao crate
    #[test_strategy::proptest]
    fn bao_decode_slice_comparison(#[strategy(size_and_start())] size_and_start: (usize, usize)) {
        // ignore the end, we always want to encode a single chunk
        let (size, start) = size_and_start;
        let end = start + 1;
        let data = make_test_data(size);
        let chunk_start = ChunkNum::full_chunks(start as u64);
        let chunk_end = ChunkNum::chunks(end as u64).max(chunk_start + 1);
        let ranges = ChunkRanges::from(chunk_start..chunk_end);
        let (mut encoded, hash) = encode_ranges_reference(&data, &ranges, BlockSize::ZERO);
        encoded.splice(..0, size.to_le_bytes().into_iter());
        let bao_hash = bao::Hash::from(*hash.as_bytes());
        let mut decoder =
            bao::decode::SliceDecoder::new(Cursor::new(&encoded), &bao_hash, start as u64, 1);
        let mut expected_decoded = Vec::new();
        decoder.read_to_end(&mut expected_decoded).unwrap();
        let actual_decoded = data[start..end.min(data.len())].to_vec();
        prop_assert_eq!(expected_decoded, actual_decoded);
    }
}
