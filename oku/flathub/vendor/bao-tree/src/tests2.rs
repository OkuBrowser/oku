//! About these tests
//!
//! The tests are structured as follows:
//!
//! There is a function <testname>_impl that is the actual implementation of the test.
//! There is a proptest called <testname>_proptest that calls the test multiple times with random data.
//! There is a test called <testname>_cases that calls the test with a few hardcoded values, either
//! handcrafted or from a previous failure of a proptest.
use std::ops::Range;

use bytes::{Bytes, BytesMut};
#[cfg(feature = "validate")]
use futures_lite::StreamExt;
use proptest::prelude::*;
use range_collections::{RangeSet2, RangeSetRef};
use smallvec::SmallVec;
use test_strategy::proptest;

use crate::{
    assert_tuple_eq, blake3,
    io::{
        fsm::ResponseDecoderNext,
        outboard::{PostOrderMemOutboard, PreOrderMemOutboard},
        sync::Outboard,
        BaoContentItem, Leaf, Parent,
    },
    iter::{BaoChunk, PreOrderPartialChunkIterRef, ResponseIterRef},
    prop_assert_tuple_eq,
    rec::{
        encode_selected_rec, get_leaf_ranges, keyed_outboard_functions_checks, make_test_data,
        partial_chunk_iter_reference, range_union, response_iter_reference, select_nodes_rec,
        truncate_ranges, ReferencePreOrderPartialChunkIterRef,
    },
    BaoTree, BlockSize, ChunkNum, ChunkRanges, ChunkRangesRef, HashMode, TreeNode,
};

fn keyed_test_key(context: &[u8]) -> [u8; 32] {
    blake3::derive_key("bao-tree.test", context)
}

fn tree() -> impl Strategy<Value = BaoTree> {
    (0u64..100000, 0u8..5).prop_map(|(size, block_size)| {
        let block_size = BlockSize(block_size);
        BaoTree::new(size, block_size)
    })
}

fn block_size() -> impl Strategy<Value = BlockSize> {
    (0..=6u8).prop_map(BlockSize)
}

/// Create a random selection
/// `size` is the size of the data
/// `n` is the number of ranges, roughly the complexity of the selection
fn selection(size: u64, n: usize) -> impl Strategy<Value = ChunkRanges> {
    let chunks = BaoTree::new(size, BlockSize(0)).chunks();
    proptest::collection::vec((..chunks.0, ..chunks.0), n).prop_map(|e| {
        let mut res = ChunkRanges::empty();
        for (a, b) in e {
            let min = a.min(b);
            let max = a.max(b) + 1;
            let elem = ChunkRanges::from(ChunkNum(min)..ChunkNum(max));
            if res != elem {
                res ^= elem;
            }
        }
        res
    })
}

fn size_and_selection(
    size_range: Range<usize>,
    n: usize,
) -> impl Strategy<Value = (usize, ChunkRanges)> {
    let selection = prop_oneof! {
        8 => selection(size_range.end as u64, n),
        1 => Just(ChunkRanges::all()),
        1 => Just(ChunkRanges::from(ChunkNum(u64::MAX)..))
    };
    size_range.prop_flat_map(move |size| (Just(size), selection.clone()))
}

/// Check that the pre order traversal iterator is consistent with the pre order
/// offset function.
fn pre_traversal_offset_impl(tree: BaoTree) {
    // iterate over all nodes
    let traversal: Vec<TreeNode> = tree.pre_order_nodes_iter().collect();
    // filter out the half leaf
    let relevant = traversal
        .iter()
        .copied()
        .filter(|x| tree.is_relevant_for_outboard(*x))
        .collect::<Vec<_>>();
    // check that there is at most one half leaf
    assert!(traversal.len() - relevant.len() <= 1);
    // for the relevant nodes, check that the offset is correct
    for (offset, node) in relevant.iter().enumerate() {
        let actual = tree.pre_order_offset(*node).unwrap();
        let expected = offset as u64;
        assert_eq!(actual, expected);
    }
}

#[proptest]
fn pre_traversal_offset_proptest(#[strategy(tree())] tree: BaoTree) {
    pre_traversal_offset_impl(tree);
}

/// Check that the post order traversal iterator is consistent with the post order
/// offset function.
fn post_traversal_offset_impl(tree: BaoTree) {
    let traversal = tree.post_order_nodes_iter().collect::<Vec<_>>();
    // filter out the half leaf
    let relevant = traversal
        .iter()
        .copied()
        .filter(|x| tree.is_relevant_for_outboard(*x))
        .collect::<Vec<_>>();
    // check that there is at most one half leaf
    assert!(traversal.len() - relevant.len() <= 1);
    // for the relevant nodes, check that the offset is correct
    for (offset, node) in relevant.iter().enumerate() {
        let expected = offset as u64;
        let actual = tree.post_order_offset(*node).unwrap().value();
        assert_eq!(actual, expected);
    }
}

#[proptest]
fn post_traversal_offset_proptest(#[strategy(tree())] tree: BaoTree) {
    post_traversal_offset_impl(tree);
}

/// Check that the post order traversal iterator is consistent with the post order
/// offset function.
fn post_traversal_chunks_iter_impl(tree: BaoTree) {
    let chunks = tree.post_order_chunks_iter();
    let ranges = get_leaf_ranges(chunks);
    let union = range_union(ranges).unwrap();
    assert_eq!(union, RangeSet2::from(..tree.size));
}

#[proptest]
fn post_traversal_chunks_iter_proptest(#[strategy(tree())] tree: BaoTree) {
    post_traversal_chunks_iter_impl(tree);
}

/// Brute force test for an outboard that computes the expected hash for each pair
fn outboard_test_sync(data: &[u8], outboard: impl crate::io::sync::Outboard, mode: HashMode) {
    let tree = outboard.tree();
    let nodes = tree
        .pre_order_nodes_iter()
        .enumerate()
        .map(|(i, node)| (node, i == 0))
        .filter(|(node, _)| tree.is_relevant_for_outboard(*node))
        .collect::<Vec<_>>();
    for (node, is_root) in nodes {
        let (l_hash, r_hash) = outboard.load(node).unwrap().unwrap();
        let start_chunk = node.chunk_range().start;
        let byte_range = tree.byte_range(node);
        let data = &data[byte_range.start.try_into().unwrap()..byte_range.end.try_into().unwrap()];
        let expected = mode.hash_subtree(start_chunk.0, data, is_root);
        let actual = mode.parent_cv(&l_hash, &r_hash, is_root);
        assert_eq!(actual, expected);
    }
}

/// Brute force test for an outboard that computes the expected hash for each pair
async fn outboard_test_fsm(
    data: &[u8],
    mut outboard: impl crate::io::fsm::Outboard,
    mode: HashMode,
) {
    let tree = outboard.tree();
    let nodes = tree
        .pre_order_nodes_iter()
        .enumerate()
        .map(|(i, node)| (node, i == 0))
        .filter(|(node, _)| tree.is_relevant_for_outboard(*node))
        .collect::<Vec<_>>();
    for (node, is_root) in nodes {
        let (l_hash, r_hash) = outboard.load(node).await.unwrap().unwrap();
        let start_chunk = node.chunk_range().start;
        let byte_range = tree.byte_range(node);
        let data = &data[byte_range.start.try_into().unwrap()..byte_range.end.try_into().unwrap()];
        let expected = mode.hash_subtree(start_chunk.0, data, is_root);
        let actual = mode.parent_cv(&l_hash, &r_hash, is_root);
        assert_eq!(actual, expected);
    }
}

fn post_oder_outboard_sync_impl(tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let outboard = PostOrderMemOutboard::create(&data, tree.block_size);
    assert_eq!(
        outboard.data.len() as u64,
        outboard.tree().outboard_hash_pairs() * 64
    );
    outboard_test_sync(&data, outboard, HashMode::Standard);
}

#[test]
fn post_oder_outboard_sync_cases() {
    let cases = [(0x3001, 0)];
    for (size, block_level) in cases {
        let tree = BaoTree::new(size, BlockSize(block_level));
        post_oder_outboard_sync_impl(tree);
    }
}

#[proptest]
fn post_oder_outboard_sync_proptest(#[strategy(tree())] tree: BaoTree) {
    post_oder_outboard_sync_impl(tree);
}

fn post_oder_outboard_fsm_impl(tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let outboard = PostOrderMemOutboard::create(&data, tree.block_size);
    assert_eq!(
        outboard.data.len() as u64,
        outboard.tree().outboard_hash_pairs() * 64
    );
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(outboard_test_fsm(&data, outboard, HashMode::Standard));
}

#[proptest]
fn post_oder_outboard_fsm_proptest(#[strategy(tree())] tree: BaoTree) {
    post_oder_outboard_fsm_impl(tree);
}

fn keyed_post_order_outboard_sync_impl(tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let key = keyed_test_key(&tree.size.to_le_bytes());
    let outboard = PostOrderMemOutboard::create_keyed(&data, tree.block_size, &key);
    assert_eq!(
        outboard.data.len() as u64,
        outboard.tree().outboard_hash_pairs() * 64
    );
    outboard_test_sync(&data, outboard, HashMode::Keyed(key));
}

#[proptest]
fn keyed_post_order_outboard_sync_proptest(#[strategy(tree())] tree: BaoTree) {
    keyed_post_order_outboard_sync_impl(tree);
}

fn keyed_post_order_outboard_fsm_impl(tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let key = keyed_test_key(&tree.size.to_le_bytes());
    let outboard = PostOrderMemOutboard::create_keyed(&data, tree.block_size, &key);
    assert_eq!(
        outboard.data.len() as u64,
        outboard.tree().outboard_hash_pairs() * 64
    );
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(outboard_test_fsm(&data, outboard, HashMode::Keyed(key)));
}

#[proptest]
fn keyed_post_order_outboard_fsm_proptest(#[strategy(tree())] tree: BaoTree) {
    keyed_post_order_outboard_fsm_impl(tree);
}

fn keyed_pre_order_outboard_sync_impl(tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let key = keyed_test_key(&tree.size.to_le_bytes());
    let outboard = PreOrderMemOutboard::create_keyed(&data, tree.block_size, &key);
    assert_eq!(
        outboard.data.len(),
        outboard.tree().outboard_size().try_into().unwrap()
    );
    outboard_test_sync(&data, outboard, HashMode::Keyed(key));
}

#[proptest]
fn keyed_pre_order_outboard_sync_proptest(#[strategy(tree())] tree: BaoTree) {
    keyed_pre_order_outboard_sync_impl(tree);
}

fn keyed_pre_order_outboard_fsm_impl(tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let key = keyed_test_key(&tree.size.to_le_bytes());
    let outboard = PreOrderMemOutboard::create_keyed(&data, tree.block_size, &key);
    assert_eq!(
        outboard.data.len(),
        outboard.tree().outboard_size().try_into().unwrap()
    );
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(outboard_test_fsm(&data, outboard, HashMode::Keyed(key)));
}

#[proptest]
fn keyed_pre_order_outboard_fsm_proptest(#[strategy(tree())] tree: BaoTree) {
    keyed_pre_order_outboard_fsm_impl(tree);
}

#[proptest]
fn keyed_outboard_functions_proptest(#[strategy(tree())] tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let key = keyed_test_key(&tree.size.to_le_bytes());
    keyed_outboard_functions_checks(&data, tree.block_size, &key);
}

fn mem_outboard_flip_impl(tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let post = PostOrderMemOutboard::create(&data, tree.block_size);
    let pre = PreOrderMemOutboard::create(data, tree.block_size);
    assert_eq!(post, pre.flip());
    assert_eq!(pre, post.flip());
    assert_eq!(post, post.flip().flip());
}

#[proptest]
fn mem_outboard_flip_proptest(#[strategy(tree())] tree: BaoTree) {
    mem_outboard_flip_impl(tree);
}

#[cfg(feature = "validate")]
mod validate {

    use super::*;

    /// range is a range of chunks. Just using u64 for convenience in tests
    fn valid_outboard_ranges_sync(outboard: impl crate::io::sync::Outboard) -> ChunkRanges {
        let ranges = ChunkRanges::all();
        let iter = crate::io::sync::valid_outboard_ranges(outboard, &ranges);
        let mut res = ChunkRanges::empty();
        for item in iter {
            res |= ChunkRanges::from(item.unwrap());
        }
        res
    }

    /// range is a range of chunks. Just using u64 for convenience in tests
    fn valid_ranges_fsm(outboard: impl crate::io::fsm::Outboard, data: Bytes) -> ChunkRanges {
        run_blocking(async move {
            let ranges = ChunkRanges::all();
            let mut stream = crate::io::fsm::valid_ranges(outboard, data, &ranges);
            let mut res = ChunkRanges::empty();
            while let Some(item) = stream.next().await {
                let item = item?;
                res |= ChunkRanges::from(item);
            }
            std::io::Result::Ok(res)
        })
        .unwrap()
    }

    /// range is a range of chunks. Just using u64 for convenience in tests
    fn valid_ranges_sync(outboard: impl crate::io::sync::Outboard, data: &[u8]) -> ChunkRanges {
        let ranges = ChunkRanges::all();
        let iter = crate::io::sync::valid_ranges(outboard, data, &ranges);
        let mut res = ChunkRanges::empty();
        for item in iter {
            let item = item.unwrap();
            res |= ChunkRanges::from(item);
        }
        res
    }

    fn keyed_valid_ranges_sync(
        outboard: impl crate::io::sync::Outboard,
        data: &[u8],
        key: &[u8; 32],
    ) -> ChunkRanges {
        let ranges = ChunkRanges::all();
        let iter = crate::io::sync::keyed_valid_ranges(outboard, data, &ranges, key);
        let mut res = ChunkRanges::empty();
        for item in iter {
            let item = item.unwrap();
            res |= ChunkRanges::from(item);
        }
        res
    }

    fn keyed_valid_ranges_fsm(
        outboard: impl crate::io::fsm::Outboard,
        data: Bytes,
        key: &[u8; 32],
    ) -> ChunkRanges {
        run_blocking(async move {
            let ranges = ChunkRanges::all();
            let mut stream = crate::io::fsm::keyed_valid_ranges(outboard, data, &ranges, key);
            let mut res = ChunkRanges::empty();
            while let Some(item) = stream.next().await {
                let item = item?;
                res |= ChunkRanges::from(item);
            }
            std::io::Result::Ok(res)
        })
        .unwrap()
    }

    /// range is a range of chunks. Just using u64 for convenience in tests
    fn valid_outboard_ranges_fsm(outboard: &mut PostOrderMemOutboard) -> ChunkRanges {
        run_blocking(async move {
            let ranges = ChunkRanges::all();
            let mut stream = crate::io::fsm::valid_outboard_ranges(outboard, &ranges);
            let mut res = ChunkRanges::empty();
            while let Some(item) = stream.next().await {
                let item = item?;
                res |= ChunkRanges::from(item);
            }
            std::io::Result::Ok(res)
        })
        .unwrap()
    }

    fn validate_outboard_pos_impl(tree: BaoTree) {
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let mut outboard = PostOrderMemOutboard::create(data, block_size);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        let actual = valid_outboard_ranges_sync(&mut outboard);
        assert_eq!(expected, actual);
        let actual = valid_outboard_ranges_fsm(&mut outboard);
        assert_eq!(expected, actual)
    }

    #[proptest]
    fn validate_outboard_pos_proptest(#[strategy(tree())] tree: BaoTree) {
        validate_outboard_pos_impl(tree);
    }

    #[test]
    fn validate_outboard_pos_cases() {
        let cases = [(0x10001, 0)];
        for (size, block_level) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_outboard_pos_impl(tree);
        }
    }

    fn keyed_valid_outboard_ranges_sync(
        outboard: impl crate::io::sync::Outboard,
        key: &[u8; 32],
    ) -> ChunkRanges {
        let ranges = ChunkRanges::all();
        let iter = crate::io::sync::keyed_valid_outboard_ranges(outboard, &ranges, key);
        let mut res = ChunkRanges::empty();
        for item in iter {
            res |= ChunkRanges::from(item.unwrap());
        }
        res
    }

    fn keyed_valid_outboard_ranges_fsm(
        outboard: &mut PostOrderMemOutboard,
        key: &[u8; 32],
    ) -> ChunkRanges {
        run_blocking(async move {
            let ranges = ChunkRanges::all();
            let mut stream = crate::io::fsm::keyed_valid_outboard_ranges(outboard, &ranges, key);
            let mut res = ChunkRanges::empty();
            while let Some(item) = stream.next().await {
                let item = item?;
                res |= ChunkRanges::from(item);
            }
            std::io::Result::Ok(res)
        })
        .unwrap()
    }

    fn validate_keyed_outboard_pos_impl(tree: BaoTree) {
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let key = blake3::derive_key("bao-tree.test", b"valid-outboard-ranges");
        let mut outboard = PostOrderMemOutboard::create_keyed(data, block_size, &key);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        let actual = keyed_valid_outboard_ranges_sync(&mut outboard, &key);
        assert_eq!(expected, actual);
        let actual = keyed_valid_outboard_ranges_fsm(&mut outboard, &key);
        assert_eq!(expected, actual)
    }

    #[proptest]
    fn validate_keyed_outboard_pos_proptest(#[strategy(tree())] tree: BaoTree) {
        validate_keyed_outboard_pos_impl(tree);
    }

    #[test]
    fn validate_keyed_outboard_pos_cases() {
        let cases = [(0x10001, 0)];
        for (size, block_level) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_keyed_outboard_pos_impl(tree);
        }
    }

    /// Wrong key must not report a multi-block keyed outboard as fully valid.
    fn validate_keyed_outboard_wrong_key_impl(tree: BaoTree) {
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let key = blake3::derive_key("bao-tree.test", b"valid-outboard-ranges");
        let wrong_key = blake3::derive_key("bao-tree.test", b"wrong-key");
        let mut outboard = PostOrderMemOutboard::create_keyed(data, block_size, &key);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        let actual = keyed_valid_outboard_ranges_sync(&mut outboard, &wrong_key);
        assert_ne!(expected, actual);
        let actual = keyed_valid_outboard_ranges_fsm(&mut outboard, &wrong_key);
        assert_ne!(expected, actual)
    }

    #[test]
    fn validate_keyed_outboard_wrong_key_cases() {
        let cases = [(0x10001, 0), (0x2001, 0), (5000, 1)];
        for (size, block_level) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_keyed_outboard_wrong_key_impl(tree);
        }
    }

    fn validate_pos_impl(tree: BaoTree) {
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let mut outboard = PostOrderMemOutboard::create(&data, block_size);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        let actual = valid_ranges_sync(&outboard, &data);
        assert_eq!(expected, actual);
        let actual = valid_ranges_fsm(&mut outboard, data.into());
        assert_eq!(expected, actual);
    }

    #[proptest]
    fn validate_pos_proptest(#[strategy(tree())] tree: BaoTree) {
        validate_pos_impl(tree);
    }

    #[test]
    fn validate_pos_cases() {
        let cases = [
            // (0x10001, 0),
            (0x401, 0),
        ];
        for (size, block_level) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_pos_impl(tree);
        }
    }

    fn validate_keyed_pos_impl(tree: BaoTree) {
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let key = blake3::derive_key("bao-tree.test", b"valid-ranges");
        let mut outboard = PostOrderMemOutboard::create_keyed(&data, block_size, &key);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        let actual = keyed_valid_ranges_sync(&outboard, &data, &key);
        assert_eq!(expected, actual);
        let actual = keyed_valid_ranges_fsm(&mut outboard, data.into(), &key);
        assert_eq!(expected, actual);
    }

    #[proptest]
    fn validate_keyed_pos_proptest(#[strategy(tree())] tree: BaoTree) {
        validate_keyed_pos_impl(tree);
    }

    #[test]
    fn validate_keyed_pos_cases() {
        let cases = [(0x401, 0), (0, 0), (1, 0), (1024, 0), (1025, 2)];
        for (size, block_level) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_keyed_pos_impl(tree);
        }
    }

    fn keyed_chunk_count(ranges: &ChunkRanges) -> u64 {
        ranges
            .boundaries()
            .windows(2)
            .map(|w| (w[1] - w[0]).0)
            .sum()
    }

    fn assert_keyed_valid_ranges_wrong_key(
        outboard: &PostOrderMemOutboard,
        data: &[u8],
        wrong_key: &[u8; 32],
        expected: &ChunkRanges,
    ) {
        let actual = keyed_valid_ranges_sync(outboard, data, wrong_key);
        assert!(expected.is_superset(&actual));
        assert_ne!(actual, *expected);
        let expected_chunks = keyed_chunk_count(expected);
        let actual_chunks = keyed_chunk_count(&actual);
        assert!(actual_chunks < expected_chunks);
        let actual_fsm = keyed_valid_ranges_fsm(outboard.clone(), data.to_vec().into(), wrong_key);
        assert!(expected.is_superset(&actual_fsm));
        assert_ne!(actual_fsm, *expected);
        let actual_fsm_chunks = keyed_chunk_count(&actual_fsm);
        assert!(actual_fsm_chunks < expected_chunks);
    }

    fn validate_keyed_neg_impl(tree: BaoTree) {
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let key = blake3::derive_key("bao-tree.test", b"valid-ranges");
        let wrong_key = blake3::derive_key("bao-tree.test", b"wrong-key");
        let outboard = PostOrderMemOutboard::create_keyed(&data, block_size, &key);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        if size > 0 {
            assert_keyed_valid_ranges_wrong_key(&outboard, &data, &wrong_key, &expected);
        }
    }

    #[test]
    fn validate_keyed_neg_cases() {
        let cases = [(0x2001, 0), (1025, 1)];
        for (size, block_level) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_keyed_neg_impl(tree);
        }
    }

    #[proptest]
    fn validate_keyed_neg_proptest(#[strategy(tree())] tree: BaoTree) {
        if tree.size > 0 {
            validate_keyed_neg_impl(tree);
        }
    }

    /// Check that flipping a random bit in a keyed outboard makes at least one range invalid
    fn validate_keyed_outboard_neg_impl(tree: BaoTree, rand: u32) {
        let rand = rand as usize;
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let key = blake3::derive_key("bao-tree.test", b"valid-ranges");
        let mut outboard = PostOrderMemOutboard::create_keyed(&data, block_size, &key);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        if !outboard.data.is_empty() {
            flip_bit(&mut outboard.data, rand);
            let actual = keyed_valid_ranges_sync(&outboard, &data, &key);
            assert_ne!(expected, actual);
            let actual_fsm = keyed_valid_ranges_fsm(outboard.clone(), data.into(), &key);
            assert_ne!(expected, actual_fsm);
        }
    }

    #[test]
    fn validate_keyed_outboard_neg_cases() {
        let cases = [((0x2001, 0), 2738363904)];
        for ((size, block_level), rand) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_keyed_outboard_neg_impl(tree, rand);
        }
    }

    #[proptest]
    fn validate_keyed_outboard_neg_proptest(#[strategy(tree())] tree: BaoTree, rand: u32) {
        if tree.size > 0 && tree.outboard_hash_pairs() > 0 {
            validate_keyed_outboard_neg_impl(tree, rand);
        }
    }

    fn flip_bit(data: &mut [u8], rand: usize) {
        // flip a random bit in the outboard
        // this is the post order outboard without the length suffix,
        // so it's all hashes
        let bit = rand % data.len() * 8;
        let byte = bit / 8;
        let bit = bit % 8;
        data[byte] ^= 1 << bit;
    }

    /// Check that flipping a random bit in the outboard makes at least one range invalid
    fn validate_outboard_sync_neg_impl(tree: BaoTree, rand: u32) {
        let rand = rand as usize;
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let mut outboard = PostOrderMemOutboard::create(data, block_size);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        if !outboard.data.is_empty() {
            // flip a random bit in the outboard
            flip_bit(&mut outboard.data, rand);
            // Check that at least one range is invalid
            let actual = valid_outboard_ranges_sync(&outboard);
            assert_ne!(expected, actual);
        }
    }

    #[test]
    fn validate_outboard_sync_neg_cases() {
        let cases = [((0x6001, 3), 1265277760)];
        for ((size, block_level), rand) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_outboard_sync_neg_impl(tree, rand);
        }
    }

    #[proptest]
    fn validate_outboard_sync_neg_proptest(#[strategy(tree())] tree: BaoTree, rand: u32) {
        validate_outboard_sync_neg_impl(tree, rand);
    }

    /// Check that flipping a random bit in the outboard makes at least one range invalid
    fn validate_outboard_neg_impl(tree: BaoTree, rand: u32) {
        let rand = rand as usize;
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let mut outboard = PostOrderMemOutboard::create(data, block_size);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        if !outboard.data.is_empty() {
            // flip a random bit in the outboard
            flip_bit(&mut outboard.data, rand);
            // Check that at least one range is invalid
            let actual = valid_outboard_ranges_sync(&mut outboard);
            assert_ne!(expected, actual);
            let actual = valid_outboard_ranges_fsm(&mut outboard);
            assert_ne!(expected, actual);
        }
    }

    #[proptest]
    fn validate_outboard_neg_proptest(#[strategy(tree())] tree: BaoTree, rand: u32) {
        validate_outboard_neg_impl(tree, rand);
    }

    #[test]
    fn validate_outboard_neg_cases() {
        let cases = [((0x2001, 0), 2738363904)];
        for ((size, block_level), rand) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_outboard_neg_impl(tree, rand);
        }
    }

    /// Check that flipping a random bit in the outboard makes at least one range invalid
    fn validate_neg_impl(tree: BaoTree, rand: u32) {
        let rand = rand as usize;
        let size = tree.size.try_into().unwrap();
        let block_size = tree.block_size;
        let data = make_test_data(size);
        let mut outboard = PostOrderMemOutboard::create(&data, block_size);
        let expected = ChunkRanges::from(..outboard.tree().chunks());
        if !outboard.data.is_empty() {
            // flip a random bit in the outboard
            flip_bit(&mut outboard.data, rand);
            // Check that at least one range is invalid
            let actual = valid_ranges_sync(&mut outboard, &data);
            assert_ne!(expected, actual);
            let actual = valid_ranges_fsm(&mut outboard, data.into());
            assert_ne!(expected, actual);
        }
    }

    #[proptest]
    fn validate_neg_proptest(#[strategy(tree())] tree: BaoTree, rand: u32) {
        validate_neg_impl(tree, rand);
    }

    #[test]
    fn validate_neg_cases() {
        let cases = [((0x2001, 0), 2738363904)];
        for ((size, block_level), rand) in cases {
            let tree = BaoTree::new(size, BlockSize(block_level));
            validate_neg_impl(tree, rand);
        }
    }

    #[test]
    fn validate_bug() {
        let data = Bytes::from(make_test_data(19308432));
        let outboard = PostOrderMemOutboard::create(&data, BlockSize(4));
        let expected = ChunkRanges::from(..ChunkNum::chunks(data.len() as u64));
        let actual = valid_ranges_fsm(outboard, data.clone());
        assert_eq!(expected, actual);
    }
}

/// Encode data fully, decode it again, and check that both data and outboard are the same
///
/// using the sync io api
fn encode_decode_full_sync_impl(
    data: &[u8],
    outboard: PostOrderMemOutboard,
) -> (
    (Vec<u8>, PostOrderMemOutboard),
    (Vec<u8>, PostOrderMemOutboard),
) {
    let ranges = ChunkRanges::all();
    let size = outboard.tree.size;
    let mut encoded = Vec::new();
    crate::io::sync::encode_ranges_validated(data, &outboard, &ChunkRanges::all(), &mut encoded)
        .unwrap();
    let encoded_read = std::io::Cursor::new(encoded);
    let tree = BaoTree::new(size, outboard.tree().block_size());
    let mut decoded = Vec::new();
    let mut ob_res = PostOrderMemOutboard {
        root: outboard.root(),
        tree,
        data: vec![0; tree.outboard_size().try_into().unwrap()],
    };
    crate::io::sync::decode_ranges(encoded_read, &ranges, &mut decoded, &mut ob_res).unwrap();
    ((decoded, ob_res), (data.to_vec(), outboard))
}

/// Encode data fully, decode it again, and check that both data and outboard are the same
///
/// using the fsm io api
async fn encode_decode_full_fsm_impl(
    data: Vec<u8>,
    outboard: PostOrderMemOutboard,
) -> (
    (Vec<u8>, PostOrderMemOutboard),
    (Vec<u8>, PostOrderMemOutboard),
) {
    let size = outboard.tree.size;
    let mut outboard = outboard;
    let ranges = ChunkRanges::all();
    let mut encoded = Vec::new();
    crate::io::fsm::encode_ranges_validated(
        Bytes::from(data.clone()),
        &mut outboard,
        &ranges,
        &mut encoded,
    )
    .await
    .unwrap();

    let read_encoded = std::io::Cursor::new(encoded.as_slice());
    let mut ob_res = {
        let tree = BaoTree::new(size, outboard.tree().block_size());
        let root = outboard.root();
        let outboard_size = usize::try_from(tree.outboard_hash_pairs() * 64).unwrap();
        let outboard_data = vec![0u8; outboard_size];
        PostOrderMemOutboard {
            root,
            tree,
            data: outboard_data,
        }
    };
    let mut decoded = BytesMut::new();
    crate::io::fsm::decode_ranges(read_encoded, ranges, &mut decoded, &mut ob_res)
        .await
        .unwrap();
    ((data, outboard), (decoded.to_vec(), ob_res))
}

fn keyed_encode_decode_full_sync_impl(
    data: &[u8],
    outboard: PostOrderMemOutboard,
    key: &[u8; 32],
) -> (
    (Vec<u8>, PostOrderMemOutboard),
    (Vec<u8>, PostOrderMemOutboard),
) {
    let ranges = ChunkRanges::all();
    let size = outboard.tree.size;
    let mut encoded = Vec::new();
    crate::io::sync::keyed_encode_ranges_validated(data, &outboard, &ranges, &mut encoded, key)
        .unwrap();
    let encoded_read = std::io::Cursor::new(encoded);
    let tree = BaoTree::new(size, outboard.tree().block_size());
    let mut decoded = Vec::new();
    let mut ob_res = PostOrderMemOutboard {
        root: outboard.root(),
        tree,
        data: vec![0; tree.outboard_size().try_into().unwrap()],
    };
    crate::io::sync::keyed_decode_ranges(encoded_read, &ranges, &mut decoded, &mut ob_res, key)
        .unwrap();
    ((decoded, ob_res), (data.to_vec(), outboard))
}

async fn keyed_encode_decode_full_fsm_impl(
    data: Vec<u8>,
    outboard: PostOrderMemOutboard,
    key: &[u8; 32],
) -> (
    (Vec<u8>, PostOrderMemOutboard),
    (Vec<u8>, PostOrderMemOutboard),
) {
    let size = outboard.tree.size;
    let mut outboard = outboard;
    let ranges = ChunkRanges::all();
    let mut encoded = Vec::new();
    crate::io::fsm::keyed_encode_ranges_validated(
        Bytes::from(data.clone()),
        &mut outboard,
        &ranges,
        &mut encoded,
        key,
    )
    .await
    .unwrap();

    let read_encoded = std::io::Cursor::new(encoded.as_slice());
    let mut ob_res = {
        let tree = BaoTree::new(size, outboard.tree().block_size());
        let root = outboard.root();
        let outboard_size = usize::try_from(tree.outboard_hash_pairs() * 64).unwrap();
        let outboard_data = vec![0u8; outboard_size];
        PostOrderMemOutboard {
            root,
            tree,
            data: outboard_data,
        }
    };
    let mut decoded = BytesMut::new();
    crate::io::fsm::keyed_decode_ranges(read_encoded, ranges, &mut decoded, &mut ob_res, key)
        .await
        .unwrap();
    ((data, outboard), (decoded.to_vec(), ob_res))
}

fn keyed_encode_decode_partial_sync_impl(
    data: &[u8],
    outboard: PostOrderMemOutboard,
    ranges: &ChunkRangesRef,
    key: &[u8; 32],
) -> bool {
    let mut encoded = Vec::new();
    let size = outboard.tree.size;
    crate::io::sync::keyed_encode_ranges_validated(data, &outboard, ranges, &mut encoded, key)
        .unwrap();
    let expected_data = data;
    let encoded_read = std::io::Cursor::new(encoded);
    let tree = BaoTree::new(size, outboard.tree.block_size);
    let iter = crate::io::sync::DecodeResponseIter::new_keyed(
        outboard.root,
        tree,
        encoded_read,
        ranges,
        key,
    );
    for item in iter {
        let item = match item {
            Ok(item) => item,
            Err(_) => {
                return false;
            }
        };
        match item {
            BaoContentItem::Parent(Parent { node, pair }) => {
                if let Some(expected_pair) = outboard.load(node).unwrap() {
                    if pair != expected_pair {
                        return false;
                    }
                }
            }
            BaoContentItem::Leaf(Leaf { offset, data }) => {
                let offset = offset.try_into().unwrap();
                if expected_data[offset..offset + data.len()] != data {
                    return false;
                }
            }
        }
    }
    true
}

async fn keyed_encode_decode_partial_fsm_impl(
    data: &[u8],
    outboard: PostOrderMemOutboard,
    ranges: ChunkRanges,
    key: &[u8; 32],
) -> bool {
    let size = outboard.tree.size;
    let mut encoded = Vec::new();
    let mut outboard = outboard;
    crate::io::fsm::keyed_encode_ranges_validated(
        Bytes::from(data.to_vec()),
        &mut outboard,
        &ranges,
        &mut encoded,
        key,
    )
    .await
    .unwrap();
    let expected_data = data;
    let encoded_read = std::io::Cursor::new(encoded.as_slice());
    let mut reading = crate::io::fsm::ResponseDecoder::new_keyed(
        outboard.root,
        ranges,
        BaoTree::new(size, outboard.tree.block_size),
        encoded_read,
        key,
    );
    if size != outboard.tree.size {
        return false;
    }
    while let ResponseDecoderNext::More((reading1, result)) = reading.next().await {
        let item = match result {
            Ok(item) => item,
            Err(_) => {
                return false;
            }
        };
        match item {
            BaoContentItem::Leaf(Leaf { offset, data }) => {
                let offset: usize = offset.try_into().unwrap();
                if expected_data[offset..offset + data.len()] != data {
                    return false;
                }
            }
            BaoContentItem::Parent(Parent { node, pair }) => {
                if let Some(expected_pair) = outboard.load(node).unwrap() {
                    if pair != expected_pair {
                        return false;
                    }
                }
            }
        }
        reading = reading1;
    }
    true
}

fn encode_decode_partial_sync_impl(
    data: &[u8],
    outboard: PostOrderMemOutboard,
    ranges: &ChunkRangesRef,
) -> bool {
    let mut encoded = Vec::new();
    let size = outboard.tree.size;
    crate::io::sync::encode_ranges_validated(data, &outboard, ranges, &mut encoded).unwrap();
    let expected_data = data;
    let encoded_read = std::io::Cursor::new(encoded);
    let tree = BaoTree::new(size, outboard.tree.block_size);
    let iter = crate::io::sync::DecodeResponseIter::new(outboard.root, tree, encoded_read, ranges);
    for item in iter {
        let item = match item {
            Ok(item) => item,
            Err(_) => {
                return false;
            }
        };
        match item {
            BaoContentItem::Parent(Parent { node, pair }) => {
                // check that the hash pair matches
                if let Some(expected_pair) = outboard.load(node).unwrap() {
                    if pair != expected_pair {
                        return false;
                    }
                }
            }
            BaoContentItem::Leaf(Leaf { offset, data }) => {
                // check that the data matches
                let offset = offset.try_into().unwrap();
                if expected_data[offset..offset + data.len()] != data {
                    return false;
                }
            }
        }
    }
    true
}

async fn encode_decode_partial_fsm_impl(
    data: &[u8],
    outboard: PostOrderMemOutboard,
    ranges: ChunkRanges,
) -> bool {
    let size = outboard.tree.size;
    let mut encoded = Vec::new();
    let mut outboard = outboard;
    crate::io::fsm::encode_ranges_validated(
        Bytes::from(data.to_vec()),
        &mut outboard,
        &ranges,
        &mut encoded,
    )
    .await
    .unwrap();
    let expected_data = data;
    let encoded_read = std::io::Cursor::new(encoded.as_slice());
    let mut reading = crate::io::fsm::ResponseDecoder::new(
        outboard.root,
        ranges,
        BaoTree::new(size, outboard.tree.block_size),
        encoded_read,
    );
    if size != outboard.tree.size {
        return false;
    }
    while let ResponseDecoderNext::More((reading1, result)) = reading.next().await {
        let item = match result {
            Ok(item) => item,
            Err(_) => {
                return false;
            }
        };
        match item {
            BaoContentItem::Leaf(Leaf { offset, data }) => {
                // check that the data matches
                let offset: usize = offset.try_into().unwrap();
                if expected_data[offset..offset + data.len()] != data {
                    return false;
                }
            }
            BaoContentItem::Parent(Parent { node, pair }) => {
                // check that the hash pair matches
                if let Some(expected_pair) = outboard.load(node).unwrap() {
                    if pair != expected_pair {
                        return false;
                    }
                }
            }
        }
        reading = reading1;
    }
    true
}

#[test]
fn encode_decode_full_sync_cases() {
    let cases = [(1024 + 1, 1)];
    for (size, block_level) in cases {
        let data = &make_test_data(size);
        let block_size = BlockSize(block_level);
        let outboard = PostOrderMemOutboard::create(data, block_size);
        let pair = encode_decode_full_sync_impl(data, outboard);
        assert_tuple_eq!(pair);
    }
}

#[proptest]
fn encode_decode_full_sync_proptest(#[strategy(tree())] tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let outboard = PostOrderMemOutboard::create(&data, tree.block_size);
    prop_assert_tuple_eq!(encode_decode_full_sync_impl(&data, outboard));
}

#[proptest]
fn encode_decode_partial_sync_proptest(
    #[strategy(size_and_selection(0..100000, 2))] size_and_selection: (usize, ChunkRanges),
    #[strategy(block_size())] block_size: BlockSize,
) {
    let (size, selection) = size_and_selection;
    let data = make_test_data(size);
    let outboard = PostOrderMemOutboard::create(&data, block_size);
    let ok = encode_decode_partial_sync_impl(&data, outboard, &selection);
    prop_assert!(ok);
}

#[proptest]
fn keyed_encode_decode_full_sync_proptest(#[strategy(tree())] tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let key = keyed_test_key(&tree.size.to_le_bytes());
    let outboard = PostOrderMemOutboard::create_keyed(&data, tree.block_size, &key);
    prop_assert_tuple_eq!(keyed_encode_decode_full_sync_impl(&data, outboard, &key));
}

#[proptest]
fn keyed_encode_decode_partial_sync_proptest(
    #[strategy(size_and_selection(0..100000, 2))] size_and_selection: (usize, ChunkRanges),
    #[strategy(block_size())] block_size: BlockSize,
) {
    let (size, selection) = size_and_selection;
    let data = make_test_data(size);
    let key = keyed_test_key(&(size as u64).to_le_bytes());
    let outboard = PostOrderMemOutboard::create_keyed(&data, block_size, &key);
    let ok = keyed_encode_decode_partial_sync_impl(&data, outboard, &selection, &key);
    prop_assert!(ok);
}

#[test]
fn encode_decode_full_fsm_cases() {
    let cases = [BaoTree::new(0x1001, BlockSize(1))];
    for tree in cases {
        let data = make_test_data(tree.size.try_into().unwrap());
        let outboard = PostOrderMemOutboard::create(&data, tree.block_size);
        let pair = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(encode_decode_full_fsm_impl(data, outboard));
        assert_tuple_eq!(pair);
    }
}

#[proptest]
fn encode_decode_full_fsm_proptest(#[strategy(tree())] tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let outboard = PostOrderMemOutboard::create(&data, tree.block_size);
    let pair = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(encode_decode_full_fsm_impl(data, outboard));
    prop_assert_tuple_eq!(pair);
}

#[proptest]
fn encode_decode_partial_fsm_proptest(
    #[strategy(size_and_selection(0..100000, 2))] size_and_selection: (usize, ChunkRanges),
    #[strategy(block_size())] block_size: BlockSize,
) {
    let (size, selection) = size_and_selection;
    let data = make_test_data(size);
    let outboard = PostOrderMemOutboard::create(&data, block_size);
    let ok = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(encode_decode_partial_fsm_impl(&data, outboard, selection));
    prop_assert!(ok);
}

#[proptest]
fn keyed_encode_decode_full_fsm_proptest(#[strategy(tree())] tree: BaoTree) {
    let data = make_test_data(tree.size.try_into().unwrap());
    let key = keyed_test_key(&tree.size.to_le_bytes());
    let outboard = PostOrderMemOutboard::create_keyed(&data, tree.block_size, &key);
    let pair = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(keyed_encode_decode_full_fsm_impl(data, outboard, &key));
    prop_assert_tuple_eq!(pair);
}

#[proptest]
fn keyed_encode_decode_partial_fsm_proptest(
    #[strategy(size_and_selection(0..100000, 2))] size_and_selection: (usize, ChunkRanges),
    #[strategy(block_size())] block_size: BlockSize,
) {
    let (size, selection) = size_and_selection;
    let data = make_test_data(size);
    let key = keyed_test_key(&(size as u64).to_le_bytes());
    let outboard = PostOrderMemOutboard::create_keyed(&data, block_size, &key);
    let ok =
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(keyed_encode_decode_partial_fsm_impl(
                &data, outboard, selection, &key,
            ));
    prop_assert!(ok);
}

fn pre_order_nodes_iter_reference(tree: BaoTree, ranges: &ChunkRangesRef) -> Vec<TreeNode> {
    let mut res = Vec::new();
    select_nodes_rec(
        ChunkNum(0),
        tree.size.try_into().unwrap(),
        true,
        ranges,
        tree.block_size.to_u32(),
        tree.block_size.to_u32() + 1,
        &mut |x| {
            let node = match x {
                BaoChunk::Parent { node, .. } => node,
                BaoChunk::Leaf { start_chunk, .. } => {
                    TreeNode::from_start_chunk_and_level(start_chunk, tree.block_size)
                }
            };
            res.push(node);
        },
    );
    res
}

#[proptest]
fn pre_order_node_iter_proptest(#[strategy(tree())] tree: BaoTree) {
    let actual = tree.pre_order_nodes_iter().collect::<Vec<_>>();
    let expected = pre_order_nodes_iter_reference(tree, &ChunkRanges::all());
    prop_assert_eq!(expected, actual);
}

#[test]
fn selection_reference_comparison_cases() {
    let cases = [
        ((1026, 1), ChunkRanges::all()),
        ((2050, 1), ChunkRanges::all()),
        ((1066, 2), ChunkRanges::from(..ChunkNum(1))),
        ((1045, 0), ChunkRanges::all()),
        ((10000, 0), ChunkRanges::from(ChunkNum(u64::MAX)..)),
    ];
    for ((size, block_level), ranges) in cases {
        // println!("{} {} {:?}", size, block_level, ranges);
        let tree = BaoTree::new(size, BlockSize(block_level));
        let expected = partial_chunk_iter_reference(tree, &ranges, u8::MAX);
        let actual =
            ReferencePreOrderPartialChunkIterRef::new(tree, &ranges, u8::MAX).collect::<Vec<_>>();
        assert_eq!(expected, actual);
    }
}

#[proptest]
fn selection_reference_comparison_proptest(
    #[strategy(size_and_selection(0..100000, 2))] size_and_selection: (usize, ChunkRanges),
    #[strategy(block_size())] block_size: BlockSize,
) {
    let (size, ranges) = size_and_selection;
    let tree = BaoTree::new(size as u64, block_size);
    let expected = partial_chunk_iter_reference(tree, &ranges, 0);
    // let actual1 = ResponseIterRef::new(tree, &ranges).collect::<Vec<_>>();
    let actual2 = ReferencePreOrderPartialChunkIterRef::new(tree, &ranges, 0).collect::<Vec<_>>();
    if actual2 != expected {
        println!();
        println!("{tree:?} {ranges:?}");
        println!("actual new {actual2:?}");
        println!("expected   {expected:?}");
        panic!();
    }
}

/// Reference implementation of encode_ranges_validated that uses the simple recursive impl.
///
/// Uses unkeyed BLAKE3 mode.
fn encode_selected_reference(
    data: &[u8],
    block_size: BlockSize,
    ranges: &ChunkRangesRef,
) -> (blake3::Hash, Vec<u8>) {
    let mut res = Vec::new();
    res.extend_from_slice(&(data.len() as u64).to_le_bytes());
    let max_skip_level = block_size.to_u32();
    let hash = encode_selected_rec(
        ChunkNum(0),
        data,
        true,
        ranges,
        max_skip_level,
        true,
        &mut res,
        HashMode::Standard,
    );
    (hash, res)
}

fn cases() -> impl Iterator<Item = (BaoTree, ChunkRanges, u8)> {
    [
        // ((1, 0), ChunkRanges::all(), 0),
        // ((1025, 0), ChunkRanges::all(), 0),
        // ((2048, 0), ChunkRanges::all(), 0),
        // ((2049, 0), ChunkRanges::all(), 0),
        // ((2049, 1), ChunkRanges::all(), 0),
        // ((2049, 0), ChunkRanges::from(..ChunkNum(1)), 0),
        // ((1024 * 32, 0), ChunkRanges::from(..ChunkNum(1)), 4),
        // ((1024 * 32, 0), ChunkRanges::all(), 4),
        // ((1024 * 32, 0), ChunkRanges::from(ChunkNum(16)..), 4),
        // ((1024 * 32, 0), ChunkRanges::from(..ChunkNum(16)), 4),
        // ((2048 + 1, 0), ChunkRanges::from(..ChunkNum(2)), 0),
        // ((8192 + 1, 0), ChunkRanges::from(..ChunkNum(8)), 2),
        // ((1037, 0), ChunkRanges::all(), 1),
        // ((1024 + 1, 0), ChunkRanges::from(..ChunkNum(1)), 2),
        ((1024 * 32 - 1, 0), ChunkRanges::from(ChunkNum(16)..), 4),
        (
            (1024 * 32 - 1, 0),
            ChunkRanges::from(ChunkNum(16)..ChunkNum(32)),
            4,
        ),
    ]
    .into_iter()
    .map(|((size, block_level), ranges, min_full_level)| {
        (
            BaoTree::new(size, BlockSize(block_level)),
            ranges,
            min_full_level,
        )
    })
}

#[test]
fn filtered_chunks() {
    for (tree, ranges, min_full_level) in cases() {
        println!("{tree:?} {ranges:?}");
        println!("encode:");
        let data = make_test_data(tree.size.try_into().unwrap());
        let (_, encoded) = encode_selected_reference(&data, BlockSize(min_full_level), &ranges);
        println!("{}", hex::encode(&encoded));
        println!("select:");
        let selected = ReferencePreOrderPartialChunkIterRef::new(tree, &ranges, min_full_level)
            .collect::<Vec<_>>();
        for item in selected {
            println!("{}", item.to_debug_string(10));
        }
    }
}

#[test]
fn response_iter_cases() {
    for (tree, ranges, min_full_level) in cases() {
        let expected = partial_chunk_iter_reference(tree, &ranges, min_full_level);
        let actual =
            PreOrderPartialChunkIterRef::new(tree, &ranges, min_full_level).collect::<Vec<_>>();
        // if expected != actual {
        println!("expected:");
        for chunk in expected {
            println!("{}", chunk.to_debug_string(10));
        }
        println!("actual:");
        for chunk in actual {
            println!("{}", chunk.to_debug_string(10));
        }
        // panic!();
        // }
    }
}

#[proptest]
fn response_iter_proptest(
    #[strategy(size_and_selection(0..100000, 2))] size_and_selection: (usize, ChunkRanges),
    #[strategy(block_size())] block_size: BlockSize,
) {
    let (size, ranges) = size_and_selection;
    let tree = BaoTree::new(size as u64, BlockSize::ZERO);
    let expected = partial_chunk_iter_reference(tree, &ranges, block_size.0);
    let actual = PreOrderPartialChunkIterRef::new(tree, &ranges, block_size.0).collect::<Vec<_>>();
    if expected != actual {
        println!("expected:");
        for chunk in expected {
            println!("{}", chunk.to_debug_string(10));
        }
        println!("actual:");
        for chunk in actual {
            println!("{}", chunk.to_debug_string(10));
        }
        panic!();
    }
}

#[test]
fn response_iter_2_cases() {
    let cases = [((1024 + 1, 1), ChunkRanges::all())]
        .into_iter()
        .map(|((size, block_level), ranges)| (BaoTree::new(size, BlockSize(block_level)), ranges));
    for (tree, ranges) in cases {
        let expected = response_iter_reference(tree, &ranges);
        let actual = ResponseIterRef::new(tree, &ranges).collect::<Vec<_>>();
        if expected != actual {
            println!("expected:");
            for chunk in expected {
                println!("{}", chunk.to_debug_string(10));
            }
            println!("actual:");
            for chunk in actual {
                println!("{}", chunk.to_debug_string(10));
            }
            panic!();
        }
    }
}

#[proptest]
fn response_iter_2_proptest(
    #[strategy(size_and_selection(0..100000, 2))] size_and_selection: (usize, ChunkRanges),
    #[strategy(block_size())] block_size: BlockSize,
) {
    let (size, ranges) = size_and_selection;
    let tree = BaoTree::new(size as u64, block_size);
    let expected = response_iter_reference(tree, &ranges);
    let actual = ResponseIterRef::new(tree, &ranges).collect::<Vec<_>>();
    if expected != actual {
        println!("expected:");
        for chunk in expected {
            println!("{}", chunk.to_debug_string(10));
        }
        println!("actual:");
        for chunk in actual {
            println!("{}", chunk.to_debug_string(10));
        }
        panic!();
    }
}

fn chunk_ranges(int_ranges: &RangeSetRef<u64>) -> ChunkRanges {
    let mut bounds = SmallVec::with_capacity(int_ranges.boundaries().len());
    for b in int_ranges.boundaries() {
        bounds.push(ChunkNum(*b));
    }
    ChunkRanges::new_unchecked(bounds)
}

fn cr(x: impl Into<RangeSet2<u64>>) -> RangeSet2<ChunkNum> {
    chunk_ranges(&x.into())
}

#[test]
fn canonicalize_ranges_test() {
    fn size(x: u64) -> u64 {
        x
    }

    let cases = [
        // gets canonicalized to all
        (cr(..1), size(1024), cr(..)),
        // gets left alone
        (cr(1..), size(1024), cr(1..)),
        // gets canonicalized to all
        (cr(..31), size(31 * 1024), cr(..)),
        // gets canonicalized to all
        (cr(..31) | cr(40..50), size(31 * 1024), cr(..)),
        // gets left alone
        (cr(99..), size(31 * 1024), cr(99..)),
        // gets left alone, last chunk is not included
        (cr(..30), size(31 * 1024), cr(..30)),
        // ..30 is missing the last chunk. but that will be included due to
        // having something behind the end, so canonicalize to all
        (cr(..30) | cr(1000..), size(31 * 1024), cr(..)),
        // gets left alone, two chunks are missing
        (
            cr(..29) | cr(1000..),
            size(31 * 1024),
            cr(0..29) | cr(1000..),
        ),
        // gets left alone, two chunks are missing
        (
            cr(1..2) | cr(4..29) | cr(1000..),
            size(31 * 1024),
            cr(1..2) | cr(4..29) | cr(1000..),
        ),
        // gets turned into open range
        (
            cr(1..2) | cr(4..30) | cr(1000..),
            size(31 * 1024),
            cr(1..2) | cr(4..),
        ),
        (cr(..6), size(7 * 1024), cr(..6)),
        (cr(..7), size(7 * 1024), cr(..)),
        (cr(7..), size(7 * 1024), cr(7..)),
        (cr(..10) | cr(11..12), size(7 * 1024), cr(..)),
        (cr(..6) | cr(7..10), size(7 * 1024), cr(..)),
        (cr(3..6) | cr(7..10), size(7 * 1024), cr(3..)),
        (cr(..5) | cr(7..10), size(7 * 1024), cr(..5) | cr(7..)),
    ];
    for (ranges, size, expected) in cases {
        let expected: &RangeSetRef<ChunkNum> = &expected;
        let actual = truncate_ranges(&ranges, size);
        assert_eq!(expected, actual);
    }
}

fn run_blocking<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Runtime::new().unwrap().block_on(f)
}
