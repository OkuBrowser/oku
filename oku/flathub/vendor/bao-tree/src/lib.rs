//! # Efficient BLAKE3 based verified streaming
//!
//! This crate is similar to the [bao crate](https://crates.io/crates/bao), but
//! takes a slightly different approach.
//!
//! The core struct is [BaoTree], which describes the geometry of the tree and
//! various ways to traverse it. An individual tree node is identified by
//! [TreeNode], which is just a newtype wrapper for an u64.
//!
//! [TreeNode] provides various helpers to e.g. get the offset of a node in
//! different traversal orders.
//!
//! There are newtypes for the different kinds of integers used in the
//! tree:
//! [ChunkNum] is an u64 number of chunks,
//! [TreeNode] is an u64 tree node identifier,
//! and [BlockSize] is the log base 2 of the chunk group size.
//!
//! All this is then used in the [io] module to implement the actual io, both
//! [synchronous](io::sync) and [asynchronous](io::tokio).
//!
//! # Basic usage
//!
//! The basic workflow is like this: you have some existing data, for which
//! you want to enable verified streaming. This data can either be in memory,
//! in a file, or even a remote resource such as an HTTP server.
//!
//! ## Outboard creation
//!
//! You create an outboard using the [CreateOutboard](io::sync::CreateOutboard)
//! trait. It has functions to [create](io::sync::CreateOutboard::create) an
//! outboard from scratch or to [initialize](io::sync::CreateOutboard::init_from)
//! data and root hash from existing data.
//!
//! ## Serving requests
//!
//! You serve streaming requests by using the
//! [encode_ranges](io::sync::encode_ranges) or
//! [encode_ranges_validated](io::sync::encode_ranges_validated) functions
//! in the sync or async io module. For this you need data and a matching
//! outboard.
//!
//! The difference between the two functions is that the validated version
//! will check the hash of each chunk against the bao tree encoded in the
//! outboard, so you will detect data corruption before sending out data
//! to the requester. When using the unvalidated version, you might send out
//! corrupted data without ever noticing and earn a bad reputation.
//!
//! Due to the speed of the blake3 hash function, validation is not a
//! significant performance overhead compared to network operations and
//! encryption.
//!
//! The requester will send a set of chunk ranges they are interested in.
//! To compute chunk ranges from byte ranges, there is a helper function
//! [round_up_to_chunks](io::round_up_to_chunks) that takes a byte range and
//! rounds up to chunk ranges.
//!
//! If you just want to stream the entire blob, you can use [ChunkRanges::all]
//! as the range.
//!
//! ## Processing requests
//!
//! You process requests by using the [decode_ranges](io::sync::decode_ranges)
//! function in the sync or async io module. This function requires prior
//! knowledge of the tree geometry (total data size and block size). A common
//! way to get this information is to have the block size as a common parameter
//! of both sides, and send the total data size as a prefix of the encoded data.
//! E.g. the original bao crate uses a little endian u64 as the prefix.
//!
//! This function will perform validation in any case, there is no variant
//! that skips validation since that would defeat the purpose of verified
//! streaming.
//!
//! ## Simple end to end example
//!
//! ```no_run
//! use std::io;
//!
//! use bao_tree::{
//!     io::{
//!         outboard::PreOrderOutboard,
//!         round_up_to_chunks,
//!         sync::{decode_ranges, encode_ranges_validated, valid_ranges, CreateOutboard},
//!     },
//!     BlockSize, ByteRanges, ChunkRanges,
//! };
//!
//! /// Use a block size of 16 KiB, a good default for most cases
//! const BLOCK_SIZE: BlockSize = BlockSize::from_chunk_log(4);
//!
//! # fn main() -> io::Result<()> {
//! // The file we want to serve
//! let file = std::fs::File::open("video.mp4")?;
//! // Create an outboard for the file, using the current size
//! let ob = PreOrderOutboard::<Vec<u8>>::create(&file, BLOCK_SIZE)?;
//! // Encode the first 100000 bytes of the file
//! let ranges = ByteRanges::from(0..100000);
//! let ranges = round_up_to_chunks(&ranges);
//! // Stream of data to client. Needs to implement `io::Write`. We just use a vec here.
//! let mut to_client = vec![];
//! encode_ranges_validated(&file, &ob, &ranges, &mut to_client)?;
//!
//! // Stream of data from client. Needs to implement `io::Read`. We just wrap the vec in a cursor.
//! let from_server = io::Cursor::new(to_client);
//! let root = ob.root;
//! let tree = ob.tree;
//!
//! // Decode the encoded data into a file
//! let mut decoded = std::fs::File::create("copy.mp4")?;
//! let mut ob = PreOrderOutboard {
//!     tree,
//!     root,
//!     data: vec![],
//! };
//! decode_ranges(from_server, &ranges, &mut decoded, &mut ob)?;
//!
//! // the first 100000 bytes of the file should now be in `decoded`
//! // in addition, the required part of the tree to validate that the data is
//! // correct are in `ob.data`
//!
//! // Print the valid ranges of the file
//! for range in valid_ranges(&ob, &decoded, &ChunkRanges::all()) {
//!     println!("{:?}", range);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Async end to end example
//!
//! The async version is very similar to the sync version, except that it needs
//! an async context. All functions that do IO are async. The file has to be
//! an [iroh_io::File], which is just a wrapper for [std::fs::File] that implements
//! async random access via the [AsyncSliceReader](iroh_io::AsyncSliceReader) trait.
//!
//! We use [futures_lite] crate, but using the normal futures crate will also work.
//!
//! ```no_run
//! use std::io;
//!
//! use bao_tree::{
//!     io::{
//!         fsm::{decode_ranges, encode_ranges_validated, valid_ranges, CreateOutboard},
//!         outboard::PreOrderOutboard,
//!         round_up_to_chunks,
//!     },
//!     BlockSize, ByteRanges, ChunkRanges,
//! };
//! use bytes::BytesMut;
//! use futures_lite::StreamExt;
//!
//! /// Use a block size of 16 KiB, a good default for most cases
//! const BLOCK_SIZE: BlockSize = BlockSize::from_chunk_log(4);
//!
//! # #[tokio::main]
//! # async fn main() -> io::Result<()> {
//! // The file we want to serve
//! let mut file = iroh_io::File::open("video.mp4".into()).await?;
//! // Create an outboard for the file, using the current size
//! let mut ob = PreOrderOutboard::<BytesMut>::create(&mut file, BLOCK_SIZE).await?;
//! // Encode the first 100000 bytes of the file
//! let ranges = ByteRanges::from(0..100000);
//! let ranges = round_up_to_chunks(&ranges);
//! // Stream of data to client. Needs to implement `io::Write`. We just use a vec here.
//! let mut to_client = Vec::new();
//! encode_ranges_validated(file, &mut ob, &ranges, &mut to_client).await?;
//!
//! // Stream of data from client. Needs to implement `io::Read`. We just wrap the vec in a cursor.
//! let from_server = io::Cursor::new(to_client.as_slice());
//! let root = ob.root;
//! let tree = ob.tree;
//!
//! // Decode the encoded data into a file
//! let mut decoded = iroh_io::File::open("copy.mp4".into()).await?;
//! let mut ob = PreOrderOutboard {
//!     tree,
//!     root,
//!     data: BytesMut::new(),
//! };
//! decode_ranges(from_server, ranges, &mut decoded, &mut ob).await?;
//!
//! // the first 100000 bytes of the file should now be in `decoded`
//! // in addition, the required part of the tree to validate that the data is
//! // correct are in `ob.data`
//!
//! // Print the valid ranges of the file
//! let ranges = ChunkRanges::all();
//! let mut stream = valid_ranges(&mut ob, &mut decoded, &ranges);
//! while let Some(range) = stream.next().await {
//!     println!("{:?}", range);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Keyed hashing
//!
//! For domain-separated trees, use the `keyed_*` functions. They mirror the
//! standard API with an additional `key: &[u8; 32]` argument, like
//! [`blake3::keyed_hash`] mirrors [`blake3::hash`]. The key is out-of-band
//! metadata and is not included in the encoded stream.
//!
//! # Compatibility with the [bao crate](https://crates.io/crates/bao)
//!
//! This crate will be compatible with the bao crate, provided you do the
//! following:
//!
//! - use a block size of 1024, so no chunk groups
//! - use a little endian u64 as the prefix for the encoded data
//! - use only a single range
#![deny(missing_docs)]
use std::{
    fmt::{self, Debug},
    ops::Range,
};

use range_collections::RangeSetRef;
pub mod iter;
mod rec;
mod tree;
use iter::*;
pub use tree::{BlockSize, ChunkNum};
pub mod io;
pub use blake3;

#[cfg(all(test, feature = "tokio_fsm"))]
mod tests;
#[cfg(all(test, feature = "tokio_fsm"))]
mod tests2;

/// A set of chunk ranges
pub type ChunkRanges = range_collections::RangeSet2<ChunkNum>;

/// A set of byte ranges
pub type ByteRanges = range_collections::RangeSet2<u64>;

/// A referenceable set of chunk ranges
///
/// [ChunkRanges] implements [`AsRef<ChunkRangesRef>`].
pub type ChunkRangesRef = range_collections::RangeSetRef<ChunkNum>;

/// Hashing mode for shared encode and decode paths, either standard or keyed BLAKE3.
#[derive(Clone, Copy)]
pub(crate) enum HashMode {
    Standard,
    Keyed([u8; 32]),
}

impl std::fmt::Debug for HashMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashMode::Standard => f.write_str("Standard"),
            HashMode::Keyed(_) => f.debug_struct("Keyed").finish_non_exhaustive(),
        }
    }
}

impl HashMode {
    pub(crate) fn hash_subtree(
        &self,
        start_chunk: u64,
        data: &[u8],
        is_root: bool,
    ) -> blake3::Hash {
        use blake3::hazmat::{ChainingValue, HasherExt};
        if is_root {
            debug_assert!(start_chunk == 0);
            match self {
                HashMode::Standard => blake3::hash(data),
                HashMode::Keyed(key) => blake3::keyed_hash(key, data),
            }
        } else {
            let mut hasher = match self {
                HashMode::Standard => blake3::Hasher::new(),
                HashMode::Keyed(key) => blake3::Hasher::new_keyed(key),
            };
            hasher.set_input_offset(start_chunk * 1024);
            hasher.update(data);
            let non_root_hash: ChainingValue = hasher.finalize_non_root();
            blake3::Hash::from(non_root_hash)
        }
    }

    pub(crate) fn parent_cv(
        &self,
        left_child: &blake3::Hash,
        right_child: &blake3::Hash,
        is_root: bool,
    ) -> blake3::Hash {
        use blake3::hazmat::{merge_subtrees_non_root, merge_subtrees_root, ChainingValue, Mode};
        let left_child: ChainingValue = *left_child.as_bytes();
        let right_child: ChainingValue = *right_child.as_bytes();
        let mode = match self {
            HashMode::Standard => Mode::Hash,
            HashMode::Keyed(key) => Mode::KeyedHash(key),
        };
        if is_root {
            merge_subtrees_root(&left_child, &right_child, mode)
        } else {
            blake3::Hash::from(merge_subtrees_non_root(&left_child, &right_child, mode))
        }
    }
}

/// Compute the hash of a subtree using BLAKE3 keyed mode.
///
/// See [keyed_parent_cv] for merging child hashes in keyed mode.
#[inline]
pub fn keyed_hash_subtree(
    start_chunk: u64,
    data: &[u8],
    is_root: bool,
    key: &[u8; 32],
) -> blake3::Hash {
    HashMode::Keyed(*key).hash_subtree(start_chunk, data, is_root)
}

/// Merge two child subtree hashes using BLAKE3 keyed mode.
#[inline]
pub fn keyed_parent_cv(
    left_child: &blake3::Hash,
    right_child: &blake3::Hash,
    is_root: bool,
    key: &[u8; 32],
) -> blake3::Hash {
    HashMode::Keyed(*key).parent_cv(left_child, right_child, is_root)
}

/// Defines a Bao tree.
///
/// This is just the specification of the tree, it does not contain any actual data.
///
/// Usually trees are self-contained. This means that the tree starts at chunk 0,
/// and the hash of the root node is computed with the is_root flag set to true.
///
/// For some internal use, it is also possible to create trees that are just subtrees
/// of a larger tree. In this case, the start_chunk is the chunk number of the first
/// chunk in the tree, and the is_root flag can be false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaoTree {
    /// Total number of bytes in the file
    size: u64,
    /// Log base 2 of the chunk group size
    block_size: BlockSize,
}

/// An offset of a node in a post-order outboard
#[derive(Debug, Clone, Copy)]
pub enum PostOrderOffset {
    /// the node is stable and won't change when appending data
    Stable(u64),
    /// the node is unstable and will change when appending data
    Unstable(u64),
}

impl PostOrderOffset {
    /// Just get the offset value, ignoring whether it's stable or unstable
    pub fn value(self) -> u64 {
        match self {
            Self::Stable(n) => n,
            Self::Unstable(n) => n,
        }
    }
}

impl BaoTree {
    /// Create a new self contained BaoTree
    pub fn new(size: u64, block_size: BlockSize) -> Self {
        Self { size, block_size }
    }

    /// The size of the blob from which this tree was constructed, in bytes
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The block size of the tree
    pub fn block_size(&self) -> BlockSize {
        self.block_size
    }

    /// Given a tree of size `size` and block size `block_size`,
    /// compute the root node and the number of nodes for a shifted tree.
    pub(crate) fn shifted(&self) -> (TreeNode, TreeNode) {
        let level = self.block_size.0;
        let size = self.size;
        let shift = 10 + level;
        let mask = (1 << shift) - 1;
        // number of full blocks of size 1024 << level
        let full_blocks = size >> shift;
        // 1 if the last block is non zero, 0 otherwise
        let open_block = ((size & mask) != 0) as u64;
        // total number of blocks, rounding up to 1 if there are no blocks
        let blocks = (full_blocks + open_block).max(1);
        let n = blocks.div_ceil(2);
        // root node
        let root = n.next_power_of_two() - 1;
        // number of nodes in the tree
        let filled_size = n + n.saturating_sub(1);
        (TreeNode(root), TreeNode(filled_size))
    }

    fn byte_range(&self, node: TreeNode) -> Range<u64> {
        let start = node.chunk_range().start.to_bytes();
        let end = node.chunk_range().end.to_bytes();
        start..end.min(self.size)
    }

    /// Compute the byte ranges for a leaf node
    ///
    /// Returns two ranges, the first is the left range, the second is the right range
    /// If the leaf is partially contained in the tree, the right range will be empty
    fn leaf_byte_ranges3(&self, leaf: TreeNode) -> (u64, u64, u64) {
        let Range { start, end } = leaf.byte_range();
        let mid = leaf.mid().to_bytes();
        if !(start < self.size || (start == 0 && self.size == 0)) {
            debug_assert!(start < self.size || (start == 0 && self.size == 0));
        }
        (start, mid.min(self.size), end.min(self.size))
    }

    /// Traverse the entire tree in post order as [BaoChunk]s
    ///
    /// This iterator is used by both the sync and async io code for computing
    /// an outboard from existing data
    pub fn post_order_chunks_iter(&self) -> PostOrderChunkIter {
        PostOrderChunkIter::new(*self)
    }

    /// Traverse the part of the tree that is relevant for a ranges query
    /// in pre order as [BaoChunk]s
    ///
    /// This iterator is used by both the sync and async io code for encoding
    /// from an outboard and ranges as well as decoding an encoded stream.
    pub fn ranges_pre_order_chunks_iter_ref<'a>(
        &self,
        ranges: &'a RangeSetRef<ChunkNum>,
        min_level: u8,
    ) -> PreOrderPartialChunkIterRef<'a> {
        PreOrderPartialChunkIterRef::new(*self, ranges, min_level)
    }

    /// Traverse the entire tree in post order as [TreeNode]s,
    /// down to the level given by the block size.
    pub fn post_order_nodes_iter(&self) -> impl Iterator<Item = TreeNode> {
        let (root, len) = self.shifted();
        let shift = self.block_size.0;
        PostOrderNodeIter::new(root, len).map(move |x| x.subtract_block_size(shift))
    }

    /// Traverse the entire tree in pre order as [TreeNode]s,
    /// down to the level given by the block size.
    pub fn pre_order_nodes_iter(&self) -> impl Iterator<Item = TreeNode> {
        let (root, len) = self.shifted();
        let shift = self.block_size.0;
        PreOrderNodeIter::new(root, len).map(move |x| x.subtract_block_size(shift))
    }

    /// Traverse the part of the tree that is relevant for a ranges querys
    /// in pre order as [NodeInfo]s
    ///
    /// This is mostly used internally.
    ///
    /// When `min_level` is set to a value greater than 0, the iterator will
    /// skip all branch nodes that are at a level < min_level if they are fully
    /// covered by the ranges.
    #[cfg(test)]
    pub fn ranges_pre_order_nodes_iter<'a>(
        &self,
        ranges: &'a RangeSetRef<ChunkNum>,
        min_level: u8,
    ) -> PreOrderPartialIterRef<'a> {
        PreOrderPartialIterRef::new(*self, ranges, min_level)
    }

    /// Root of the tree
    ///
    /// Does not consider block size
    pub fn root(&self) -> TreeNode {
        let shift = 10;
        let mask = (1 << shift) - 1;
        let full_blocks = self.size >> shift;
        let open_block = ((self.size & mask) != 0) as u64;
        let blocks = (full_blocks + open_block).max(1);
        let chunks = ChunkNum(blocks);
        TreeNode::root(chunks)
    }

    /// Number of blocks in the tree
    ///
    /// At chunk group size 1, this is the same as the number of chunks
    /// Even a tree with 0 bytes size has a single block
    pub fn blocks(&self) -> u64 {
        // handle the case of an empty tree having 1 block
        blocks(self.size, self.block_size).max(1)
    }

    /// Number of chunks in the tree
    pub fn chunks(&self) -> ChunkNum {
        ChunkNum::chunks(self.size)
    }

    /// Number of hash pairs in the outboard
    fn outboard_hash_pairs(&self) -> u64 {
        self.blocks() - 1
    }

    /// The outboard size for this tree.
    ///
    /// This is the outboard size *without* the size prefix.
    pub fn outboard_size(&self) -> u64 {
        self.outboard_hash_pairs() * 64
    }

    #[allow(dead_code)]
    fn filled_size(&self) -> TreeNode {
        let blocks = self.chunks();
        let n = blocks.0.div_ceil(2);
        TreeNode(n + n.saturating_sub(1))
    }

    /// true if the node is a leaf for this tree
    ///
    /// If a tree has a non-zero block size, this is different than the node
    /// being a leaf (level=0).
    #[cfg(test)]
    const fn is_leaf(&self, node: TreeNode) -> bool {
        node.level() == self.block_size.to_u32()
    }

    /// true if the given node is persisted
    ///
    /// the only node that is not persisted is the last leaf node, if it is
    /// less than half full
    #[inline]
    #[cfg(test)]
    const fn is_persisted(&self, node: TreeNode) -> bool {
        !self.is_leaf(node) || node.mid().to_bytes() < self.size
    }

    /// true if this is a node that is relevant for the outboard
    #[inline]
    const fn is_relevant_for_outboard(&self, node: TreeNode) -> bool {
        let level = node.level();
        if level < self.block_size.to_u32() {
            // too small, this outboard does not track it
            false
        } else if level > self.block_size.to_u32() {
            // a parent node, always relevant
            true
        } else {
            node.mid().to_bytes() < self.size
        }
    }

    /// The offset of the given node in the pre order traversal
    pub fn pre_order_offset(&self, node: TreeNode) -> Option<u64> {
        // if the node has a level less than block_size, this will return None
        let shifted = node.add_block_size(self.block_size.0)?;
        let is_half_leaf = shifted.is_leaf() && node.mid().to_bytes() >= self.size;
        if !is_half_leaf {
            let (_, tree_filled_size) = self.shifted();
            Some(pre_order_offset_loop(shifted.0, tree_filled_size.0))
        } else {
            None
        }
    }

    /// The offset of the given node in the post order traversal
    pub fn post_order_offset(&self, node: TreeNode) -> Option<PostOrderOffset> {
        // if the node has a level less than block_size, this will return None
        let shifted = node.add_block_size(self.block_size.0)?;
        if node.byte_range().end <= self.size {
            // stable node, use post_order_offset
            Some(PostOrderOffset::Stable(shifted.post_order_offset()))
        } else {
            // unstable node
            if shifted.is_leaf() && node.mid().to_bytes() >= self.size {
                // half full leaf node, not considered
                None
            } else {
                // compute the offset based on the total size and the height of the node
                self.outboard_hash_pairs()
                    .checked_sub(u64::from(node.right_count()) + 1)
                    .map(PostOrderOffset::Unstable)
            }
        }
    }

    const fn chunk_group_chunks(&self) -> ChunkNum {
        ChunkNum(1 << self.block_size.0)
    }

    fn chunk_group_bytes(&self) -> usize {
        self.chunk_group_chunks().to_bytes().try_into().unwrap()
    }
}

/// number of blocks that this number of bytes covers,
/// given a block size
pub(crate) const fn blocks(size: u64, block_size: BlockSize) -> u64 {
    let chunk_group_log = block_size.0;
    let block_bits = chunk_group_log + 10;
    let block_mask = (1 << block_bits) - 1;
    let full_blocks = size >> block_bits;
    let open_block = ((size & block_mask) != 0) as u64;
    full_blocks + open_block
}

/// An u64 that defines a node in a bao tree.
///
/// You typically don't have to use this, but it can be useful for debugging
/// and error handling. Hash validation errors contain a `TreeNode` that allows
/// you to find the position where validation failed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeNode(u64);

impl fmt::Display for TreeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for TreeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            write!(f, "TreeNode({})", self.0)
        } else if self.is_leaf() {
            write!(f, "TreeNode::Leaf({})", self.0)
        } else {
            write!(f, "TreeNode::Branch({}, level={})", self.0, self.level())
        }
    }
}

impl TreeNode {
    /// Create a new tree node from a start chunk and a level
    ///
    /// The start chunk must be the start of a subtree with the given level.
    /// So for level 0, the start chunk must even. For level 1, the start chunk
    /// must be divisible by 4, etc.
    ///
    /// This is a bridge from the recursive reference implementation to the node
    /// based implementations, and is therefore only used in tests.
    #[cfg(all(test, feature = "tokio_fsm"))]
    fn from_start_chunk_and_level(start_chunk: ChunkNum, level: BlockSize) -> Self {
        let start_chunk = start_chunk.0;
        let level = level.0;
        // check that the start chunk a start of a subtree with level `level`
        // this ensures that there is a 0 at bit `level`.
        let check_mask = (1 << (level + 1)) - 1;
        debug_assert_eq!(start_chunk & check_mask, 0);
        let level_mask = (1 << level) - 1;
        // set the trailing `level` bits to 1.
        // The level is the number of trailing ones.
        Self(start_chunk | level_mask)
    }

    /// Given a number of blocks, gives root node
    fn root(chunks: ChunkNum) -> TreeNode {
        Self(chunks.0.div_ceil(2).next_power_of_two() - 1)
    }

    /// the middle of the tree node, in blocks
    pub const fn mid(&self) -> ChunkNum {
        ChunkNum(self.0 + 1)
    }

    #[inline]
    const fn half_span(&self) -> u64 {
        1 << self.level()
    }

    /// The level of the node in the tree, 0 for leafs.
    #[inline]
    pub const fn level(&self) -> u32 {
        self.0.trailing_ones()
    }

    /// True if this is a leaf node.
    #[inline]
    pub const fn is_leaf(&self) -> bool {
        (self.0 & 1) == 0
    }

    /// Convert a node to a node in a tree with a smaller block size
    ///
    /// E.g. a leaf node in a tree with block size 4 will become a node
    /// with level 4 in a tree with block size 0.
    ///
    /// This works by just adding n trailing 1 bits to the node by shifting
    /// to the left.
    #[inline]
    pub const fn subtract_block_size(&self, n: u8) -> Self {
        let shifted = !(!self.0 << n);
        Self(shifted)
    }

    /// Convert a node to a node in a tree with a larger block size
    ///
    /// If the nodes has n trailing 1 bits, they are removed by shifting
    /// the node to the right by n bits.
    ///
    /// If the node has less than n trailing 1 bits, the node is too small
    /// to be represented in the target tree.
    #[inline]
    pub const fn add_block_size(&self, n: u8) -> Option<Self> {
        let mask = (1 << n) - 1;
        // check if the node has a high enough level
        if self.0 & mask == mask {
            Some(Self(self.0 >> n))
        } else {
            None
        }
    }

    /// Range of blocks that this node covers, given a block size
    ///
    /// Note that this will give the untruncated range, which may be larger than
    /// the actual tree. To get the exact byte range for a tree, use
    /// [BaoTree::byte_range];
    fn byte_range(&self) -> Range<u64> {
        let range = self.chunk_range();
        range.start.to_bytes()..range.end.to_bytes()
    }

    /// Number of nodes below this node, excluding this node.
    #[inline]
    pub const fn count_below(&self) -> u64 {
        // go to representation where trailing zeros are the level
        let x = self.0 + 1;
        // isolate the lowest bit
        let lowest_bit = x & (-(x as i64) as u64);
        // number of nodes is n * 2 - 1, subtract 1 for the node itself
        lowest_bit * 2 - 2
    }

    /// Get the next left ancestor of this node, or None if there is none.
    pub fn next_left_ancestor(&self) -> Option<Self> {
        self.next_left_ancestor0().map(Self)
    }

    /// Get the left child of this node, or None if it is a child node.
    pub fn left_child(&self) -> Option<Self> {
        let offset = 1 << self.level().checked_sub(1)?;
        Some(Self(self.0 - offset))
    }

    /// Get the right child of this node, or None if it is a child node.
    pub fn right_child(&self) -> Option<Self> {
        let offset = 1 << self.level().checked_sub(1)?;
        Some(Self(self.0 + offset))
    }

    /// Unrestricted parent, can only be None if we are at the top
    pub fn parent(&self) -> Option<Self> {
        let level = self.level();
        if level == 63 {
            return None;
        }
        let span = 1u64 << level;
        let offset = self.0;
        Some(Self(if (offset & (span * 2)) == 0 {
            offset + span
        } else {
            offset - span
        }))
    }

    /// Restricted parent, will be None if we call parent on the root
    pub fn restricted_parent(&self, len: Self) -> Option<Self> {
        let mut curr = *self;
        while let Some(parent) = curr.parent() {
            if parent.0 < len.0 {
                return Some(parent);
            }
            curr = parent;
        }
        // we hit the top
        None
    }

    /// Get a valid right descendant for an offset
    pub(crate) fn right_descendant(&self, len: Self) -> Option<Self> {
        let mut node = self.right_child()?;
        while node >= len {
            node = node.left_child()?;
        }
        Some(node)
    }

    /// Get the range of nodes this node covers
    pub const fn node_range(&self) -> Range<Self> {
        let half_span = self.half_span();
        let nn = self.0;
        let r = nn + half_span;
        let l = nn + 1 - half_span;
        Self(l)..Self(r)
    }

    /// Get the range of blocks this node covers
    pub fn chunk_range(&self) -> Range<ChunkNum> {
        let level = self.level();
        let span = 1 << level;
        let mid = self.0 + 1;
        // at level 0 (leaf), range will be nn..nn+2
        // at level >0 (branch), range will be centered on nn+1
        ChunkNum(mid - span)..ChunkNum(mid + span)
    }

    /// the number of times you have to go right from the root to get to this node
    ///
    /// 0 for a root node
    pub fn right_count(&self) -> u32 {
        (self.0 + 1).count_ones() - 1
    }

    /// Get the post order offset of this node
    #[inline]
    pub const fn post_order_offset(&self) -> u64 {
        // compute number of nodes below me
        let below_me = self.count_below();
        // compute next ancestor that is to the left
        let next_left_ancestor = self.next_left_ancestor0();
        // compute offset
        match next_left_ancestor {
            Some(nla) => below_me + nla + 1 - ((nla + 1).count_ones() as u64),
            None => below_me,
        }
    }

    /// Get the range of post order offsets this node covers
    pub const fn post_order_range(&self) -> Range<u64> {
        let offset = self.post_order_offset();
        let end = offset + 1;
        let start = offset - self.count_below();
        start..end
    }

    /// Get the next left ancestor, or None if we don't have one
    ///
    /// this is a separate fn so it can be const.
    #[inline]
    const fn next_left_ancestor0(&self) -> Option<u64> {
        // add 1 to go to the representation where trailing zeroes = level
        let x = self.0 + 1;
        // clear the lowest bit
        let without_lowest_bit = x & (x - 1);
        // go back to the normal representation,
        // producing None if without_lowest_bit is 0, which means that there is no next left ancestor
        without_lowest_bit.checked_sub(1)
    }
}

/// Iterative way to find the offset of a node in a pre-order traversal.
///
/// I am sure there is a way that does not require a loop, but this will do for now.
/// It is slower than the direct formula, but it is still in the nanosecond range,
/// so at a block size of 16 KiB it should not be the limiting factor for anything.
fn pre_order_offset_loop(node: u64, len: u64) -> u64 {
    // node level, 0 for leaf nodes
    let level = (!node).trailing_zeros();
    // span of the node, 1 for leaf nodes
    let span = 1u64 << level;
    // nodes to the left of the tree of this node
    let left = node + 1 - span;
    // count the parents with a loop
    let mut parent_count = 0;
    let mut offset = node;
    let mut span = span;
    // loop until we reach the root, adding valid parents
    loop {
        let pspan = span * 2;
        // find parent
        offset = if (offset & pspan) == 0 {
            offset + span
        } else {
            offset - span
        };
        // if parent is inside the tree, increase parent count
        if offset < len {
            parent_count += 1;
        }
        if pspan >= len {
            // we are at the root
            break;
        }
        span = pspan;
    }
    left - (left.count_ones() as u64) + parent_count
}

/// Split a range set into range sets for the left and right half of a node
///
/// Requires that the range set is minimal, it should not contain any redundant
/// boundaries outside of the range of the node. The values outside of the node
/// range don't matter, so any change outside the range must be omitted.
///
/// Produces two range sets that are also minimal. A range set for left or right
/// that covers the entire range of the node will be replaced with the set of
/// all chunks, so an is_all() check can be used to check if further recursion
/// is necessary.
pub(crate) fn split(
    ranges: &RangeSetRef<ChunkNum>,
    node: TreeNode,
) -> (&RangeSetRef<ChunkNum>, &RangeSetRef<ChunkNum>) {
    let mid = node.mid();
    let start = node.chunk_range().start;
    split_inner(ranges, start, mid)
}

/// The actual implementation of split. This is used from split and from the
/// recursive reference implementation.
pub(crate) fn split_inner(
    ranges: &RangeSetRef<ChunkNum>,
    start: ChunkNum,
    mid: ChunkNum,
) -> (&RangeSetRef<ChunkNum>, &RangeSetRef<ChunkNum>) {
    let (mut a, mut b) = ranges.split(mid);
    // check that a does not contain a redundant boundary at or after mid
    debug_assert!(a.boundaries().last() < Some(&mid));
    // Replace a with the canonicalized version if it is a single interval that
    // starts at or before start. This is necessary to be able to check it with
    // RangeSetRef::is_all()
    if a.boundaries().len() == 1 && a.boundaries()[0] <= start {
        a = RangeSetRef::new(&[ChunkNum(0)]).unwrap();
    }
    // Replace b with the canonicalized version if it is a single interval that
    // starts at or before mid. This is necessary to be able to check it with
    // RangeSetRef::is_all()
    if b.boundaries().len() == 1 && b.boundaries()[0] <= mid {
        b = RangeSetRef::new(&[ChunkNum(0)]).unwrap();
    }
    (a, b)
}

// Module that handles io::Error serialization/deserialization
#[cfg(feature = "serde")]
mod io_error_serde {
    use std::{fmt, io};

    use serde::{
        de::{self, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S>(error: &io::Error, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize the error kind and message
        serializer.serialize_str(&format!("{:?}:{}", error.kind(), error))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<io::Error, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IoErrorVisitor;

        impl Visitor<'_> for IoErrorVisitor {
            type Value = io::Error;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an io::Error string representation")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // For simplicity, create a generic error
                // In a real app, you might want to parse the kind from the string
                Ok(io::Error::other(value))
            }
        }

        deserializer.deserialize_str(IoErrorVisitor)
    }
}
