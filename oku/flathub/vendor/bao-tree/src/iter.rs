//! Iterators over BaoTree nodes
//!
//! Range iterators take a reference to the ranges, and therefore require a lifetime parameter.
//! They can be used without lifetime parameters using self referencing structs.
use std::fmt::{self, Debug};

use self_cell::self_cell;
use smallvec::SmallVec;

use crate::{split, BaoTree, BlockSize, ChunkNum, ChunkRanges, ChunkRangesRef, TreeNode};

/// Extended node info.
///
/// Some of the information is redundant, but it is convenient to have it all in one place.
///
/// Usually this is used within an iterator, so we hope that the compiler will optimize away
/// the redundant information.
#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub struct NodeInfo<'a> {
    /// the node
    pub node: TreeNode,
    /// the node is the root node (needs special handling when computing hash)
    pub is_root: bool,
    /// ranges of the node and it's two children
    pub ranges: &'a ChunkRangesRef,
    /// left child intersection with the query range
    pub l_ranges: &'a ChunkRangesRef,
    /// right child intersection with the query range
    pub r_ranges: &'a ChunkRangesRef,
    /// the node is fully included in the query range
    pub full: bool,
    /// the node is a leaf for the purpose of this query
    pub query_leaf: bool,
    /// true if this node is the last leaf, and it is <= half full
    pub is_half_leaf: bool,
}

/// Iterator over all nodes in a BaoTree in pre-order that overlap with a given chunk range.
///
/// This is mostly used internally
#[derive(Debug)]
#[cfg(test)]
pub struct PreOrderPartialIterRef<'a> {
    /// the tree we want to traverse
    tree: BaoTree,
    /// the minimum level to always emit, even if the node is fully within the query range
    min_level: u8,
    /// stack of nodes to visit, together with the ranges that are relevant for the node
    ///
    /// The node is shifted by the block size, so these are not normal nodes!
    stack: SmallVec<[(TreeNode, &'a ChunkRangesRef); 8]>,
    /// number of valid nodes, needed in shifted.right_descendant
    ///
    /// This is also shifted by the block size!
    shifted_filled_size: TreeNode,
    /// The root node, shifted by the block size, needed for the is_root check
    shifted_root: TreeNode,
}

#[cfg(test)]
impl<'a> PreOrderPartialIterRef<'a> {
    /// Create a new iterator over the tree.
    pub fn new(tree: BaoTree, ranges: &'a ChunkRangesRef, min_level: u8) -> Self {
        let mut stack = SmallVec::new();
        let (shifted_root, shifted_filled_size) = tree.shifted();
        stack.push((shifted_root, ranges));
        Self {
            tree,
            min_level,
            stack,
            shifted_filled_size,
            shifted_root,
        }
    }

    /// Get a reference to the tree.
    pub fn tree(&self) -> &BaoTree {
        &self.tree
    }
}

#[cfg(test)]
impl<'a> Iterator for PreOrderPartialIterRef<'a> {
    type Item = NodeInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let tree = &self.tree;
        loop {
            let (shifted, ranges) = self.stack.pop()?;
            if ranges.is_empty() {
                continue;
            }
            let node = shifted.subtract_block_size(tree.block_size.0);
            let is_half_leaf = !tree.is_persisted(node);
            let (l_ranges, r_ranges) = if !is_half_leaf {
                // split the ranges into left and right
                split(ranges, node)
            } else {
                (ranges, ranges)
            };
            // check if the node is fully included
            let full = ranges.is_all();
            // we can't recurse if the node is a leaf
            // we don't want to recurse if the node is full and below the minimum level
            let query_leaf = shifted.is_leaf() || (full && node.level() < self.min_level as u32);
            // recursion is just pushing the children onto the stack
            if !query_leaf {
                let l = shifted.left_child().unwrap();
                let r = shifted.right_descendant(self.shifted_filled_size).unwrap();
                // push right first so we pop left first
                self.stack.push((r, r_ranges));
                self.stack.push((l, l_ranges));
            }
            // the first node is the root, so just set the flag to false afterwards
            let is_root = shifted == self.shifted_root;
            // emit the node in any case
            break Some(NodeInfo {
                node,
                ranges,
                l_ranges,
                r_ranges,
                full,
                query_leaf,
                is_root,
                is_half_leaf,
            });
        }
    }
}

/// Iterator over all nodes in a tree in post-order.
///
/// If you want to iterate only down to some level, you need to shift the tree
/// before.
#[derive(Debug)]
pub struct PostOrderNodeIter {
    /// the overall number of nodes in the tree
    len: TreeNode,
    /// the current node, None if we are done
    curr: TreeNode,
    /// where we came from, used to determine the next node
    prev: Prev,
}

impl PostOrderNodeIter {
    /// Create a new iterator given a root node and a len
    pub fn new(root: TreeNode, len: TreeNode) -> Self {
        Self {
            curr: root,
            len,
            prev: Prev::Parent,
        }
    }

    fn go_up(&mut self, curr: TreeNode) {
        let prev = curr;
        (self.curr, self.prev) = if let Some(parent) = curr.restricted_parent(self.len) {
            (
                parent,
                if prev < parent {
                    Prev::Left
                } else {
                    Prev::Right
                },
            )
        } else {
            (curr, Prev::Done)
        };
    }
}

impl Iterator for PostOrderNodeIter {
    type Item = TreeNode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let curr = self.curr;
            match self.prev {
                Prev::Parent => {
                    if let Some(child) = curr.left_child() {
                        // go left first when coming from above, don't emit curr
                        self.curr = child;
                        self.prev = Prev::Parent;
                    } else {
                        // we are a left or right leaf, go up and emit curr
                        self.go_up(curr);
                        break Some(curr);
                    }
                }
                Prev::Left => {
                    // no need to check is_leaf, since we come from a left child
                    // go right when coming from left, don't emit curr
                    self.curr = curr.right_descendant(self.len).unwrap();
                    self.prev = Prev::Parent;
                }
                Prev::Right => {
                    // go up in any case, do emit curr
                    self.go_up(curr);
                    break Some(curr);
                }
                Prev::Done => {
                    break None;
                }
            }
        }
    }
}

/// Iterator over all nodes in a BaoTree in pre-order.
#[derive(Debug)]
pub struct PreOrderNodeIter {
    /// the overall number of nodes in the tree
    len: TreeNode,
    /// the current node, None if we are done
    curr: TreeNode,
    /// where we came from, used to determine the next node
    prev: Prev,
}

impl PreOrderNodeIter {
    /// Create a new iterator given a root node and a len
    pub fn new(root: TreeNode, len: TreeNode) -> Self {
        Self {
            curr: root,
            len,
            prev: Prev::Parent,
        }
    }

    fn go_up(&mut self, curr: TreeNode) {
        let prev = curr;
        (self.curr, self.prev) = if let Some(parent) = curr.restricted_parent(self.len) {
            (
                parent,
                if prev < parent {
                    Prev::Left
                } else {
                    Prev::Right
                },
            )
        } else {
            (curr, Prev::Done)
        };
    }
}

impl Iterator for PreOrderNodeIter {
    type Item = TreeNode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let curr = self.curr;
            match self.prev {
                Prev::Parent => {
                    if let Some(child) = curr.left_child() {
                        // go left first when coming from above
                        self.curr = child;
                        self.prev = Prev::Parent;
                    } else {
                        // we are a left or right leaf, go up
                        self.go_up(curr);
                    }
                    // emit curr before children (pre-order)
                    break Some(curr);
                }
                Prev::Left => {
                    // no need to check is_leaf, since we come from a left child
                    // go right when coming from left, don't emit curr
                    self.curr = curr.right_descendant(self.len).unwrap();
                    self.prev = Prev::Parent;
                }
                Prev::Right => {
                    // go up in any case
                    self.go_up(curr);
                }
                Prev::Done => {
                    break None;
                }
            }
        }
    }
}

#[derive(Debug)]
enum Prev {
    Parent,
    Left,
    Right,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A chunk describeds what to read or write next
///
/// In some cases you want additional information about what part of the chunk matches the query.
/// That is what the `R` type parameter is for. By default it is `()`.
pub enum BaoChunk<R = ()> {
    /// expect a 64 byte parent node.
    ///
    /// To validate, use parent_cv using the is_root value
    Parent {
        /// The tree node, useful for error reporting
        node: TreeNode,
        /// This is the root, to be passed to parent_cv
        is_root: bool,
        /// Push the left hash to the stack, since it will be needed later
        left: bool,
        /// Push the right hash to the stack, since it will be needed later
        right: bool,
        /// Additional information about what part of the chunk matches the query
        ranges: R,
    },
    /// expect data of size `size`
    ///
    /// To validate, use hash_block using the is_root and start_chunk values
    Leaf {
        /// Start chunk, to be passed to hash_block
        start_chunk: ChunkNum,
        /// Size of the data to expect. Will be chunk_group_bytes for all but the last block.
        size: usize,
        /// This is the root, to be passed to hash_block
        is_root: bool,
        /// Additional information about what part of the chunk matches the query
        ranges: R,
    },
}

impl<T> BaoChunk<T> {
    /// Produce a nice debug string for the chunk
    #[cfg(test)]
    pub fn to_debug_string(&self, max_level: usize) -> String {
        match self {
            BaoChunk::Parent { node, is_root, .. } => {
                let n = max_level.saturating_sub(node.level() as usize + 1);
                let prefix = " ".repeat(n);
                let start_chunk = node.chunk_range().start;
                format!(
                    "{}{},{},{}",
                    prefix,
                    start_chunk.to_bytes(),
                    node.level(),
                    is_root
                )
            }
            BaoChunk::Leaf {
                start_chunk,
                size,
                is_root,
                ..
            } => {
                let prefix = " ".repeat(max_level);
                format!("{}{},{},{}", prefix, start_chunk.to_bytes(), size, is_root)
            }
        }
    }

    /// Set the ranges to the unit value
    pub fn without_ranges(&self) -> BaoChunk {
        match self {
            Self::Parent {
                node,
                is_root,
                left,
                right,
                ..
            } => BaoChunk::Parent {
                node: *node,
                is_root: *is_root,
                left: *left,
                right: *right,
                ranges: (),
            },
            Self::Leaf {
                start_chunk,
                size,
                is_root,
                ..
            } => BaoChunk::Leaf {
                start_chunk: *start_chunk,
                size: *size,
                is_root: *is_root,
                ranges: (),
            },
        }
    }
}

/// Iterator over all chunks in a BaoTree in post-order.
#[derive(Debug)]
pub struct PostOrderChunkIter {
    tree: BaoTree,
    inner: PostOrderNodeIter,
    // stack with 2 elements, since we can only have 2 items in flight
    stack: SmallVec<[BaoChunk; 2]>,
    shifted_root: TreeNode,
}

impl PostOrderChunkIter {
    /// Create a new iterator over the tree.
    pub fn new(tree: BaoTree) -> Self {
        let (shifted_root, shifted_len) = tree.shifted();
        let inner = PostOrderNodeIter::new(shifted_root, shifted_len);
        Self {
            tree,
            inner,
            stack: Default::default(),
            shifted_root,
        }
    }
}

impl Iterator for PostOrderChunkIter {
    type Item = BaoChunk;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.stack.pop() {
                return Some(item);
            }
            let shifted = self.inner.next()?;
            // the is_root check needs to be done before shifting the node
            let is_root = shifted == self.shifted_root;
            let node = shifted.subtract_block_size(self.tree.block_size.0);
            if shifted.is_leaf() {
                let tree = &self.tree;
                let (s, m, e) = tree.leaf_byte_ranges3(node);
                let l_start_chunk = node.chunk_range().start;
                let r_start_chunk = l_start_chunk + tree.chunk_group_chunks();
                let is_half_leaf = m == e;
                // for the half leaf we emit just the leaf
                // for all other leaves we emit the parent and two leaves
                if !is_half_leaf {
                    self.stack.push(BaoChunk::Parent {
                        node,
                        is_root,
                        left: true,
                        right: true,
                        ranges: (),
                    });
                    self.stack.push(BaoChunk::Leaf {
                        is_root: false,
                        start_chunk: r_start_chunk,
                        size: (e - m).try_into().unwrap(),
                        ranges: (),
                    });
                };
                break Some(BaoChunk::Leaf {
                    is_root: is_root && is_half_leaf,
                    start_chunk: l_start_chunk,
                    size: (m - s).try_into().unwrap(),
                    ranges: (),
                });
            } else {
                self.stack.push(BaoChunk::Parent {
                    node,
                    is_root,
                    left: true,
                    right: true,
                    ranges: (),
                });
            }
        }
    }
}

impl BaoChunk {
    /// Return the size of the chunk in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::Parent { .. } => 64,
            Self::Leaf { size, .. } => *size,
        }
    }
}

impl<T: Default> Default for BaoChunk<T> {
    fn default() -> Self {
        Self::Leaf {
            is_root: true,
            size: 0,
            start_chunk: ChunkNum(0),
            ranges: T::default(),
        }
    }
}

/// Iterator over all nodes in a BaoTree in pre-order that overlap with a given chunk range.
///
/// This is mostly used internally
#[derive(Debug)]
pub struct PreOrderPartialChunkIterRef<'a> {
    /// the tree we want to traverse
    tree: BaoTree,
    /// the minimum level to always emit, even if the node is fully within the query range
    min_full_level: u8,
    /// stack of nodes to visit, together with the ranges that are relevant for the node
    ///
    /// The node is shifted by the block size, so these are not normal nodes!
    stack: SmallVec<[(TreeNode, &'a ChunkRangesRef); 8]>,
    /// number of valid nodes, needed in shifted.right_descendant
    ///
    /// This is also shifted by the block size!
    shifted_filled_size: TreeNode,
    /// The root node, shifted by the block size, needed for the is_root check
    shifted_root: TreeNode,
    /// chunk buffer. This will only ever contain leaves, and will never be more than 2 elements
    buffer: SmallVec<[BaoChunk<&'a ChunkRangesRef>; 2]>,
}

impl<'a> PreOrderPartialChunkIterRef<'a> {
    /// Create a new iterator over the tree.
    pub fn new(tree: BaoTree, ranges: &'a ChunkRangesRef, min_full_level: u8) -> Self {
        let mut stack = SmallVec::new();
        let (shifted_root, shifted_filled_size) = tree.shifted();
        stack.push((shifted_root, ranges));
        Self {
            tree,
            min_full_level,
            stack,
            shifted_filled_size,
            shifted_root,
            buffer: SmallVec::new(),
        }
    }

    /// Get a reference to the tree.
    pub fn tree(&self) -> &BaoTree {
        &self.tree
    }

    /// Get the minimum level to always emit, even if the node is fully within the query range
    pub fn min_full_level(&self) -> u8 {
        self.min_full_level
    }
}

impl<'a> Iterator for PreOrderPartialChunkIterRef<'a> {
    type Item = BaoChunk<&'a ChunkRangesRef>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.buffer.pop() {
            return Some(item);
        }
        let tree = &self.tree;
        let (shifted, ranges) = self.stack.pop()?;
        debug_assert!(!ranges.is_empty());
        let node = shifted.subtract_block_size(tree.block_size.0);
        // we don't want to recurse if the node is full and below the minimum level
        let ranges_is_all = ranges.is_all();
        let below_min_full_level = node.level() < self.min_full_level as u32;
        let query_leaf = ranges_is_all && below_min_full_level;
        // check if the node is the root by comparing the shifted node to the shifted root
        let is_root = shifted == self.shifted_root;
        let chunk_range = node.chunk_range();
        let byte_range = tree.byte_range(node);
        let size = (byte_range.end - byte_range.start).try_into().unwrap();
        // There are three cases.
        if query_leaf {
            // The node is a query leaf, meaning that we stop descending because the
            // node is fully within the query range and it's level is below min_full_level.
            // This can be fully disabled by setting min_full_level to 0.
            //
            // In this case we just emit the range of the node, and don't recurse.
            Some(BaoChunk::Leaf {
                start_chunk: chunk_range.start,
                size,
                is_root,
                ranges,
            })
        } else if !shifted.is_leaf() {
            // The node is either not fully within the query range, or it's level is above
            // min_full_level. In this case we need to recurse.
            let (l_ranges, r_ranges) = split(ranges, node);
            // emit right child first, so it gets yielded last
            if !r_ranges.is_empty() {
                let r = shifted.right_descendant(self.shifted_filled_size).unwrap();
                self.stack.push((r, r_ranges));
            }
            // emit left child second, so it gets yielded first
            if !l_ranges.is_empty() {
                let l = shifted.left_child().unwrap();
                self.stack.push((l, l_ranges));
            }
            // immediately emit the parent
            Some(BaoChunk::Parent {
                node,
                left: !l_ranges.is_empty(),
                right: !r_ranges.is_empty(),
                is_root,
                ranges,
            })
        } else {
            // The node is a real leaf.
            //
            // If it is a normal leaf and we got this far, we need to split it into 2 ranges.
            // E.g. the first leaf of a tree covers the range 0..2048, but we want two 1024
            // byte BLAKE3 chunks.
            //
            // There is a special case for the last leaf, if its right range is not within the
            // tree. In this case we don't need to split it, and can just emit it as is.
            let mid_chunk = node.mid();
            let mid = mid_chunk.to_bytes();
            if mid >= tree.size {
                // this is the last leaf node, and only it's left part is in the range
                // we can just emit it without splitting
                Some(BaoChunk::Leaf {
                    start_chunk: chunk_range.start,
                    size,
                    is_root,
                    ranges,
                })
            } else {
                let (l_ranges, r_ranges) = split(ranges, node);
                // emit right range first, so it gets yielded last
                if !r_ranges.is_empty() {
                    self.buffer.push(BaoChunk::Leaf {
                        start_chunk: mid_chunk,
                        size: (byte_range.end - mid).try_into().unwrap(),
                        is_root: false,
                        ranges: r_ranges,
                    });
                }
                // emit left range second, so it gets yielded first
                if !l_ranges.is_empty() {
                    self.buffer.push(BaoChunk::Leaf {
                        start_chunk: chunk_range.start,
                        size: (mid - byte_range.start).try_into().unwrap(),
                        is_root: false,
                        ranges: l_ranges,
                    });
                }
                // immediately emit the parent
                Some(BaoChunk::Parent {
                    node,
                    left: !l_ranges.is_empty(),
                    right: !r_ranges.is_empty(),
                    is_root,
                    ranges,
                })
            }
        }
    }
}

/// An iterator that produces chunks in pre order.
///
/// This wraps a `PreOrderPartialIterRef` and iterates over the chunk groups
/// all the way down to individual chunks if needed.
#[derive(Debug)]
pub struct ResponseIterRef<'a> {
    inner: PreOrderPartialChunkIterRef<'a>,
}

impl<'a> ResponseIterRef<'a> {
    /// Create a new iterator over the tree.
    pub fn new(tree: BaoTree, ranges: &'a ChunkRangesRef) -> Self {
        let tree1 = BaoTree::new(tree.size, BlockSize::ZERO);
        Self {
            inner: PreOrderPartialChunkIterRef::new(tree1, ranges, tree.block_size.0),
        }
    }

    /// Return the underlying tree.
    pub fn tree(&self) -> BaoTree {
        // the inner iterator uses a tree with block size 0, so we need to return the original tree
        BaoTree::new(
            self.inner.tree().size,
            BlockSize(self.inner.min_full_level()),
        )
    }
}

impl Iterator for ResponseIterRef<'_> {
    type Item = BaoChunk;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.inner.next()?.without_ranges())
    }
}

self_cell! {
    pub(crate) struct ResponseIterInner {
        owner: ChunkRanges,
        #[not_covariant]
        dependent: ResponseIterRef,
    }
}

impl ResponseIterInner {
    fn next(&mut self) -> Option<BaoChunk> {
        self.with_dependent_mut(|_, iter| iter.next())
    }

    fn tree(&self) -> BaoTree {
        self.with_dependent(|_, iter| iter.tree())
    }
}

/// The owned version of `ResponseIterRef`.
pub struct ResponseIter(ResponseIterInner);

impl fmt::Debug for ResponseIter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseIter").finish_non_exhaustive()
    }
}

impl ResponseIter {
    /// Create a new iterator over the tree.
    pub fn new(tree: BaoTree, ranges: ChunkRanges) -> Self {
        Self(ResponseIterInner::new(ranges, |ranges| {
            ResponseIterRef::new(tree, ranges)
        }))
    }

    /// The tree this iterator is iterating over.
    pub fn tree(&self) -> BaoTree {
        self.0.tree()
    }
}

impl Iterator for ResponseIter {
    type Item = BaoChunk;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
