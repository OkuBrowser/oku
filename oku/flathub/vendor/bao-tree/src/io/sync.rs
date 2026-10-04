//! Sync IO operations
//!
//! The traits to perform positioned io are re-exported from
//! [positioned-io](https://crates.io/crates/positioned-io).
use std::{
    io::{self, Read, Seek, Write},
    result,
};

use bytes::BytesMut;
pub use positioned_io::{ReadAt, Size, WriteAt};
use smallvec::SmallVec;

use super::{combine_hash_pair, BaoContentItem, DecodeError};
pub use crate::rec::truncate_ranges;
use crate::{
    blake3,
    io::{
        error::EncodeError,
        outboard::{parse_hash_pair, PostOrderOutboard, PreOrderOutboard},
        Leaf, Parent,
    },
    iter::{BaoChunk, ResponseIterRef},
    rec::encode_selected_rec,
    BaoTree, BlockSize, ChunkRangesRef, HashMode, TreeNode,
};

/// A binary merkle tree for blake3 hashes of a blob.
///
/// This trait contains information about the geometry of the tree, the root hash,
/// and a method to load the hashes at a given node.
///
/// It is up to the implementor to decide how to store the hashes.
///
/// In the original bao crate, the hashes are stored in a file in pre order.
/// This is implemented for a generic io object in [super::outboard::PreOrderOutboard]
/// and for a memory region in [super::outboard::PreOrderMemOutboard].
///
/// For files that grow over time, it is more efficient to store the hashes in post order.
/// This is implemented for a generic io object in [super::outboard::PostOrderOutboard]
/// and for a memory region in [super::outboard::PostOrderMemOutboard].
///
/// If you use a different storage engine, you can implement this trait for it. E.g.
/// you could store the hashes in a database and use the node number as the key.
pub trait Outboard {
    /// The root hash
    fn root(&self) -> blake3::Hash;
    /// The tree. This contains the information about the size of the file and the block size.
    fn tree(&self) -> BaoTree;
    /// load the hash pair for a node
    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>>;
}

/// A mutable outboard.
///
/// This trait provides a way to save a hash pair for a node and to set the
/// length of the data file.
///
/// This trait can be used to incrementally save an outboard when receiving data.
/// If you want to just ignore outboard data, there is a special placeholder outboard
/// implementation [super::outboard::EmptyOutboard].
pub trait OutboardMut: Sized {
    /// Save a hash pair for a node
    fn save(&mut self, node: TreeNode, hash_pair: &(blake3::Hash, blake3::Hash)) -> io::Result<()>;

    /// Sync the outboard.
    fn sync(&mut self) -> io::Result<()>;
}

/// Convenience trait to initialize an outboard from a data source.
///
/// In complex real applications, you might want to do this manually.
pub trait CreateOutboard {
    /// Create an outboard from a data source.
    fn create(mut data: impl Read + Seek, block_size: BlockSize) -> io::Result<Self>
    where
        Self: Default + Sized,
    {
        let size = data.seek(io::SeekFrom::End(0))?;
        data.rewind()?;
        Self::create_sized(data, size, block_size)
    }

    /// create an outboard from a data source. This requires the outboard to
    /// have a default implementation, which is the case for the memory
    /// implementations.
    fn create_sized(data: impl Read, size: u64, block_size: BlockSize) -> io::Result<Self>
    where
        Self: Default + Sized;

    /// Init the outboard from a data source. This will use the existing
    /// tree and only init the data and set the root hash.
    ///
    /// So this can be used to initialize an outboard that does not have a default,
    /// such as a file based one.
    ///
    /// It will only include data up the the current tree size.
    fn init_from(&mut self, data: impl Read) -> io::Result<()>;
}

impl<O: OutboardMut> OutboardMut for &mut O {
    fn save(&mut self, node: TreeNode, hash_pair: &(blake3::Hash, blake3::Hash)) -> io::Result<()> {
        (**self).save(node, hash_pair)
    }

    fn sync(&mut self) -> io::Result<()> {
        (**self).sync()
    }
}

impl<O: Outboard> Outboard for &O {
    fn root(&self) -> blake3::Hash {
        (**self).root()
    }
    fn tree(&self) -> BaoTree {
        (**self).tree()
    }
    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        (**self).load(node)
    }
}

impl<O: Outboard> Outboard for &mut O {
    fn root(&self) -> blake3::Hash {
        (**self).root()
    }
    fn tree(&self) -> BaoTree {
        (**self).tree()
    }
    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        (**self).load(node)
    }
}

impl<R: ReadAt> Outboard for PreOrderOutboard<R> {
    fn root(&self) -> blake3::Hash {
        self.root
    }

    fn tree(&self) -> BaoTree {
        self.tree
    }

    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        let Some(offset) = self.tree.pre_order_offset(node) else {
            return Ok(None);
        };
        let offset = offset * 64;
        let mut content = [0u8; 64];
        self.data.read_exact_at(offset, &mut content)?;
        Ok(Some(parse_hash_pair(content)))
    }
}

impl<W: WriteAt> OutboardMut for PreOrderOutboard<W> {
    fn save(&mut self, node: TreeNode, hash_pair: &(blake3::Hash, blake3::Hash)) -> io::Result<()> {
        let Some(offset) = self.tree.pre_order_offset(node) else {
            return Ok(());
        };
        let offset = offset * 64;
        let mut content = [0u8; 64];
        content[0..32].copy_from_slice(hash_pair.0.as_bytes());
        content[32..64].copy_from_slice(hash_pair.1.as_bytes());
        self.data.write_all_at(offset, &content)?;
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        self.data.flush()
    }
}

impl<W: WriteAt> CreateOutboard for PreOrderOutboard<W> {
    fn create_sized(data: impl Read, size: u64, block_size: BlockSize) -> io::Result<Self>
    where
        Self: Default + Sized,
    {
        let tree = BaoTree::new(size, block_size);
        let mut res = Self {
            tree,
            ..Default::default()
        };
        res.init_from(data)?;
        res.sync()?;
        Ok(res)
    }

    fn init_from(&mut self, data: impl Read) -> io::Result<()> {
        let mut this = self;
        let root = outboard(data, this.tree, &mut this)?;
        this.root = root;
        this.sync()?;
        Ok(())
    }
}

impl<W: WriteAt> CreateOutboard for PostOrderOutboard<W> {
    fn create_sized(data: impl Read, size: u64, block_size: BlockSize) -> io::Result<Self>
    where
        Self: Default + Sized,
    {
        let tree = BaoTree::new(size, block_size);
        let mut res = Self {
            tree,
            ..Default::default()
        };
        res.init_from(data)?;
        res.sync()?;
        Ok(res)
    }

    fn init_from(&mut self, data: impl Read) -> io::Result<()> {
        let mut this = self;
        let root = outboard(data, this.tree, &mut this)?;
        this.root = root;
        this.sync()?;
        Ok(())
    }
}

impl<W: WriteAt> OutboardMut for PostOrderOutboard<W> {
    fn save(&mut self, node: TreeNode, hash_pair: &(blake3::Hash, blake3::Hash)) -> io::Result<()> {
        let Some(offset) = self.tree.post_order_offset(node) else {
            return Ok(());
        };
        let offset = offset.value() * 64;
        let mut content = [0u8; 64];
        content[0..32].copy_from_slice(hash_pair.0.as_bytes());
        content[32..64].copy_from_slice(hash_pair.1.as_bytes());
        self.data.write_all_at(offset, &content)?;
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        self.data.flush()
    }
}

impl<R: ReadAt> Outboard for PostOrderOutboard<R> {
    fn root(&self) -> blake3::Hash {
        self.root
    }

    fn tree(&self) -> BaoTree {
        self.tree
    }

    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        let Some(offset) = self.tree.post_order_offset(node) else {
            return Ok(None);
        };
        let offset = offset.value() * 64;
        let mut content = [0u8; 64];
        self.data.read_exact_at(offset, &mut content)?;
        Ok(Some(parse_hash_pair(content)))
    }
}

/// Iterator that can be used to decode a response to a range request.
///
/// Keyed callers should use [Self::new_keyed].
#[derive(Debug)]
pub struct DecodeResponseIter<'a, R> {
    inner: ResponseIterRef<'a>,
    stack: SmallVec<[blake3::Hash; 10]>,
    encoded: R,
    buf: BytesMut,
    mode: HashMode,
}

impl<'a, R: Read> DecodeResponseIter<'a, R> {
    /// Create a new iterator to decode a response.
    ///
    /// For decoding you need to know the root hash, block size, and the ranges that were requested.
    /// Additionally you need to provide a reader that can be used to read the encoded data.
    pub fn new(root: blake3::Hash, tree: BaoTree, encoded: R, ranges: &'a ChunkRangesRef) -> Self {
        let buf = BytesMut::with_capacity(tree.block_size().bytes());
        Self::new_with_buffer(root, tree, encoded, ranges, buf)
    }

    /// Create a new iterator to decode a response.
    ///
    /// This is the same as [Self::new], but allows you to provide a buffer to use for decoding.
    /// The buffer will be resized as needed, but it's capacity should be the [crate::BlockSize::bytes].
    pub fn new_with_buffer(
        root: blake3::Hash,
        tree: BaoTree,
        encoded: R,
        ranges: &'a ChunkRangesRef,
        buf: BytesMut,
    ) -> Self {
        DecodeResponseIter::with_mode(root, tree, encoded, ranges, buf, HashMode::Standard)
    }

    /// Create a new iterator to decode a keyed response.
    pub fn new_keyed(
        root: blake3::Hash,
        tree: BaoTree,
        encoded: R,
        ranges: &'a ChunkRangesRef,
        key: &[u8; 32],
    ) -> Self {
        let buf = BytesMut::with_capacity(tree.block_size().bytes());
        Self::new_keyed_with_buffer(root, tree, encoded, ranges, buf, key)
    }

    /// Create a new iterator to decode a keyed response.
    ///
    /// This is the same as [Self::new_keyed], but allows you to provide a buffer to use for decoding.
    /// The buffer will be resized as needed, but it's capacity should be the [crate::BlockSize::bytes].
    pub fn new_keyed_with_buffer(
        root: blake3::Hash,
        tree: BaoTree,
        encoded: R,
        ranges: &'a ChunkRangesRef,
        buf: BytesMut,
        key: &[u8; 32],
    ) -> Self {
        DecodeResponseIter::with_mode(root, tree, encoded, ranges, buf, HashMode::Keyed(*key))
    }
    pub(crate) fn with_mode(
        root: blake3::Hash,
        tree: BaoTree,
        encoded: R,
        ranges: &'a ChunkRangesRef,
        buf: BytesMut,
        mode: HashMode,
    ) -> Self {
        let ranges = truncate_ranges(ranges, tree.size());
        let mut stack = SmallVec::new();
        stack.push(root);
        Self {
            stack,
            inner: ResponseIterRef::new(tree, ranges),
            encoded,
            buf,
            mode,
        }
    }

    /// Get a reference to the buffer used for decoding.
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    /// Get a reference to the tree used for decoding.
    ///
    /// This is only available after the first chunk has been decoded.
    pub fn tree(&self) -> BaoTree {
        self.inner.tree()
    }

    fn next0(&mut self) -> result::Result<Option<BaoContentItem>, DecodeError> {
        match self.inner.next() {
            Some(BaoChunk::Parent {
                is_root,
                left,
                right,
                node,
                ..
            }) => {
                let pair @ (l_hash, r_hash) = read_parent(&mut self.encoded)
                    .map_err(|e| DecodeError::maybe_parent_not_found(e, node))?;
                let parent_hash = self.stack.pop().unwrap();
                let actual = self.mode.parent_cv(&l_hash, &r_hash, is_root);
                if parent_hash != actual {
                    return Err(DecodeError::ParentHashMismatch(node));
                }
                if right {
                    self.stack.push(r_hash);
                }
                if left {
                    self.stack.push(l_hash);
                }
                Ok(Some(Parent { node, pair }.into()))
            }
            Some(BaoChunk::Leaf {
                size,
                is_root,
                start_chunk,
                ..
            }) => {
                self.buf.resize(size, 0);
                self.encoded
                    .read_exact(&mut self.buf)
                    .map_err(|e| DecodeError::maybe_leaf_not_found(e, start_chunk))?;
                let actual = self.mode.hash_subtree(start_chunk.0, &self.buf, is_root);
                let leaf_hash = self.stack.pop().unwrap();
                if leaf_hash != actual {
                    return Err(DecodeError::LeafHashMismatch(start_chunk));
                }
                Ok(Some(
                    Leaf {
                        offset: start_chunk.to_bytes(),
                        data: self.buf.split().freeze(),
                    }
                    .into(),
                ))
            }
            None => Ok(None),
        }
    }
}

impl<R: Read> Iterator for DecodeResponseIter<'_, R> {
    type Item = result::Result<BaoContentItem, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next0().transpose()
    }
}

/// Encode ranges relevant to a query from a reader and outboard to a writer
///
/// This will not validate on writing, so data corruption will be detected on reading
///
/// It is possible to encode ranges from a partial file and outboard.
/// This will either succeed if the requested ranges are all present, or fail
/// as soon as a range is missing.
pub fn encode_ranges<D: ReadAt + Size, O: Outboard, W: Write>(
    data: D,
    outboard: O,
    ranges: &ChunkRangesRef,
    encoded: W,
) -> result::Result<(), EncodeError> {
    let data = data;
    let mut encoded = encoded;
    let tree = outboard.tree();
    let mut buffer = vec![0u8; tree.chunk_group_bytes()];
    for item in tree.ranges_pre_order_chunks_iter_ref(ranges, 0) {
        match item {
            BaoChunk::Parent { node, .. } => {
                let (l_hash, r_hash) = outboard.load(node)?.unwrap();
                let pair = combine_hash_pair(&l_hash, &r_hash);
                encoded.write_all(&pair)?;
            }
            BaoChunk::Leaf {
                start_chunk, size, ..
            } => {
                let start = start_chunk.to_bytes();
                let buf = &mut buffer[..size];
                data.read_exact_at(start, buf)?;
                encoded.write_all(buf)?;
            }
        }
    }
    Ok(())
}

/// Encode ranges relevant to a query from a reader and outboard to a writer
///
/// This function validates the data before writing.
///
/// It is possible to encode ranges from a partial file and outboard.
/// This will either succeed if the requested ranges are all present, or fail
/// as soon as a range is missing.
pub fn encode_ranges_validated<D: ReadAt, O: Outboard, W: Write>(
    data: D,
    outboard: O,
    ranges: &ChunkRangesRef,
    encoded: W,
) -> result::Result<(), EncodeError> {
    encode_ranges_validated_impl(data, outboard, ranges, encoded, HashMode::Standard)
}

/// Encode ranges with BLAKE3 keyed hash validation.
pub fn keyed_encode_ranges_validated<D: ReadAt, O: Outboard, W: Write>(
    data: D,
    outboard: O,
    ranges: &ChunkRangesRef,
    encoded: W,
    key: &[u8; 32],
) -> result::Result<(), EncodeError> {
    encode_ranges_validated_impl(data, outboard, ranges, encoded, HashMode::Keyed(*key))
}

fn encode_ranges_validated_impl<D: ReadAt, O: Outboard, W: Write>(
    data: D,
    outboard: O,
    ranges: &ChunkRangesRef,
    encoded: W,
    mode: HashMode,
) -> result::Result<(), EncodeError> {
    if ranges.is_empty() {
        return Ok(());
    }
    let mut stack = SmallVec::<[blake3::Hash; 10]>::new();
    stack.push(outboard.root());
    let data = data;
    let mut encoded = encoded;
    let tree = outboard.tree();
    let mut buffer = vec![0u8; tree.chunk_group_bytes()];
    let mut out_buf = Vec::new();
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
                let pair = combine_hash_pair(&l_hash, &r_hash);
                encoded.write_all(&pair)?;
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
                let buf = &mut buffer[..size];
                data.read_exact_at(start, buf)?;
                let (actual, to_write) = if !ranges.is_all() {
                    // we need to encode just a part of the data
                    //
                    // write into an out buffer to ensure we detect mismatches
                    // before writing to the output.
                    out_buf.clear();
                    let actual = encode_selected_rec(
                        start_chunk,
                        buf,
                        is_root,
                        ranges,
                        tree.block_size.to_u32(),
                        true,
                        &mut out_buf,
                        mode,
                    );
                    (actual, &out_buf[..])
                } else {
                    let actual = mode.hash_subtree(start_chunk.0, buf, is_root);
                    #[allow(clippy::redundant_slicing)]
                    (actual, &buf[..])
                };
                if actual != expected {
                    return Err(EncodeError::LeafHashMismatch(start_chunk));
                }
                encoded.write_all(to_write)?;
            }
        }
    }
    Ok(())
}

/// Decode a response into a file while updating an outboard.
///
/// If you do not want to update an outboard, use [super::outboard::EmptyOutboard] as
/// the outboard.
pub fn decode_ranges<R, O, W>(
    encoded: R,
    ranges: &ChunkRangesRef,
    target: W,
    outboard: O,
) -> std::result::Result<(), DecodeError>
where
    O: OutboardMut + Outboard,
    R: Read,
    W: WriteAt,
{
    decode_ranges_impl(encoded, ranges, target, outboard, HashMode::Standard)
}

/// Decode a keyed response into a file while updating an outboard.
pub fn keyed_decode_ranges<R, O, W>(
    encoded: R,
    ranges: &ChunkRangesRef,
    target: W,
    outboard: O,
    key: &[u8; 32],
) -> std::result::Result<(), DecodeError>
where
    O: OutboardMut + Outboard,
    R: Read,
    W: WriteAt,
{
    decode_ranges_impl(encoded, ranges, target, outboard, HashMode::Keyed(*key))
}

fn decode_ranges_impl<R, O, W>(
    encoded: R,
    ranges: &ChunkRangesRef,
    mut target: W,
    mut outboard: O,
    mode: HashMode,
) -> std::result::Result<(), DecodeError>
where
    O: OutboardMut + Outboard,
    R: Read,
    W: WriteAt,
{
    let iter = DecodeResponseIter::with_mode(
        outboard.root(),
        outboard.tree(),
        encoded,
        ranges,
        BytesMut::with_capacity(outboard.tree().block_size().bytes()),
        mode,
    );
    for item in iter {
        match item? {
            BaoContentItem::Parent(Parent { node, pair }) => {
                outboard.save(node, &pair)?;
            }
            BaoContentItem::Leaf(Leaf { offset, data }) => {
                target.write_all_at(offset, &data)?;
            }
        }
    }
    Ok(())
}

/// Compute the outboard for the given data.
///
/// Unlike [outboard_post_order], this will work with any outboard
/// implementation, but it is not guaranteed that writes are sequential.
pub fn outboard(
    data: impl Read,
    tree: BaoTree,
    outboard: impl OutboardMut,
) -> io::Result<blake3::Hash> {
    outboard_with_mode(data, tree, outboard, HashMode::Standard)
}

/// Compute the keyed outboard for the given data.
pub fn keyed_outboard(
    data: impl Read,
    tree: BaoTree,
    outboard: impl OutboardMut,
    key: &[u8; 32],
) -> io::Result<blake3::Hash> {
    outboard_with_mode(data, tree, outboard, HashMode::Keyed(*key))
}

/// Allocates a chunk group buffer and delegates to [outboard_impl].
fn outboard_with_mode(
    data: impl Read,
    tree: BaoTree,
    mut outboard: impl OutboardMut,
    mode: HashMode,
) -> io::Result<blake3::Hash> {
    let mut buffer = vec![0u8; tree.chunk_group_bytes()];
    outboard_impl(tree, data, &mut outboard, &mut buffer, mode)
}

fn outboard_impl(
    tree: BaoTree,
    mut data: impl Read,
    mut outboard: impl OutboardMut,
    buffer: &mut [u8],
    mode: HashMode,
) -> io::Result<blake3::Hash> {
    // do not allocate for small trees
    let mut stack = SmallVec::<[blake3::Hash; 10]>::new();
    debug_assert!(buffer.len() == tree.chunk_group_bytes());
    for item in tree.post_order_chunks_iter() {
        match item {
            BaoChunk::Parent { is_root, node, .. } => {
                let right_hash = stack.pop().unwrap();
                let left_hash = stack.pop().unwrap();
                outboard.save(node, &(left_hash, right_hash))?;
                let parent = mode.parent_cv(&left_hash, &right_hash, is_root);
                stack.push(parent);
            }
            BaoChunk::Leaf {
                size,
                is_root,
                start_chunk,
                ..
            } => {
                let buf = &mut buffer[..size];
                data.read_exact(buf)?;
                let hash = mode.hash_subtree(start_chunk.0, buf, is_root);
                stack.push(hash);
            }
        }
    }
    debug_assert_eq!(stack.len(), 1);
    let hash = stack.pop().unwrap();
    Ok(hash)
}

/// Compute the post order outboard for the given data, writing into a io::Write
///
/// For the post order outboard, writes to the target are sequential.
///
/// This will not add the size to the output. You need to store it somewhere else
/// or append it yourself.
pub fn outboard_post_order(
    data: impl Read,
    tree: BaoTree,
    outboard: impl Write,
) -> io::Result<blake3::Hash> {
    outboard_post_order_with_mode(data, tree, outboard, HashMode::Standard)
}

/// Compute the keyed post order outboard for the given data.
pub fn keyed_outboard_post_order(
    data: impl Read,
    tree: BaoTree,
    outboard: impl Write,
    key: &[u8; 32],
) -> io::Result<blake3::Hash> {
    outboard_post_order_with_mode(data, tree, outboard, HashMode::Keyed(*key))
}

/// Allocates a chunk group buffer and delegates to [outboard_post_order_impl].
fn outboard_post_order_with_mode(
    data: impl Read,
    tree: BaoTree,
    mut outboard: impl Write,
    mode: HashMode,
) -> io::Result<blake3::Hash> {
    let mut buffer = vec![0u8; tree.chunk_group_bytes()];
    outboard_post_order_impl(tree, data, &mut outboard, &mut buffer, mode)
}

fn outboard_post_order_impl(
    tree: BaoTree,
    mut data: impl Read,
    mut outboard: impl Write,
    buffer: &mut [u8],
    mode: HashMode,
) -> io::Result<blake3::Hash> {
    // do not allocate for small trees
    let mut stack = SmallVec::<[blake3::Hash; 10]>::new();
    debug_assert!(buffer.len() == tree.chunk_group_bytes());
    for item in tree.post_order_chunks_iter() {
        match item {
            BaoChunk::Parent { is_root, .. } => {
                let right_hash = stack.pop().unwrap();
                let left_hash = stack.pop().unwrap();
                outboard.write_all(left_hash.as_bytes())?;
                outboard.write_all(right_hash.as_bytes())?;
                let parent = mode.parent_cv(&left_hash, &right_hash, is_root);
                stack.push(parent);
            }
            BaoChunk::Leaf {
                size,
                is_root,
                start_chunk,
                ..
            } => {
                let buf = &mut buffer[..size];
                data.read_exact(buf)?;
                let hash = mode.hash_subtree(start_chunk.0, buf, is_root);
                stack.push(hash);
            }
        }
    }
    debug_assert_eq!(stack.len(), 1);
    let hash = stack.pop().unwrap();
    Ok(hash)
}

fn read_parent(mut from: impl Read) -> std::io::Result<(blake3::Hash, blake3::Hash)> {
    let mut buf = [0; 64];
    from.read_exact(&mut buf)?;
    let l_hash = blake3::Hash::from(<[u8; 32]>::try_from(&buf[..32]).unwrap());
    let r_hash = blake3::Hash::from(<[u8; 32]>::try_from(&buf[32..]).unwrap());
    Ok((l_hash, r_hash))
}

/// Copy an outboard to another outboard.
///
/// This can be used to persist an in memory outboard or to change from
/// pre-order to post-order.
pub fn copy(from: impl Outboard, mut to: impl OutboardMut) -> io::Result<()> {
    let tree = from.tree();
    for node in tree.pre_order_nodes_iter() {
        if let Some(hash_pair) = from.load(node)? {
            to.save(node, &hash_pair)?;
        }
    }
    Ok(())
}

#[cfg(feature = "validate")]
mod validate {
    use std::{io, ops::Range};

    use genawaiter::sync::{Co, Gen};
    use positioned_io::ReadAt;

    use super::Outboard;
    use crate::{
        blake3, io::LocalBoxFuture, rec::truncate_ranges, split, BaoTree, ChunkNum, ChunkRangesRef,
        HashMode, TreeNode,
    };

    /// Given a data file and an outboard, compute all valid ranges.
    ///
    /// This is not cheap since it recomputes the hashes for all chunks.
    ///
    /// To reduce the amount of work, you can specify a range you are interested in.
    pub fn valid_ranges<'a, O, D>(
        outboard: O,
        data: D,
        ranges: &'a ChunkRangesRef,
    ) -> impl IntoIterator<Item = io::Result<Range<ChunkNum>>> + 'a
    where
        O: Outboard + 'a,
        D: ReadAt + 'a,
    {
        valid_ranges_impl(outboard, data, ranges, HashMode::Standard)
    }

    /// Given a data file and a keyed outboard, compute all valid ranges.
    pub fn keyed_valid_ranges<'a, O, D>(
        outboard: O,
        data: D,
        ranges: &'a ChunkRangesRef,
        key: &[u8; 32],
    ) -> impl IntoIterator<Item = io::Result<Range<ChunkNum>>> + 'a
    where
        O: Outboard + 'a,
        D: ReadAt + 'a,
    {
        valid_ranges_impl(outboard, data, ranges, HashMode::Keyed(*key))
    }

    fn valid_ranges_impl<'a, O, D>(
        outboard: O,
        data: D,
        ranges: &'a ChunkRangesRef,
        mode: HashMode,
    ) -> impl IntoIterator<Item = io::Result<Range<ChunkNum>>> + 'a
    where
        O: Outboard + 'a,
        D: ReadAt + 'a,
    {
        Gen::new(move |co| async move {
            if let Err(cause) =
                RecursiveDataValidator::validate(outboard, data, ranges, &co, mode).await
            {
                co.yield_(Err(cause)).await;
            }
        })
    }

    struct RecursiveDataValidator<'a, O: Outboard, D: ReadAt> {
        tree: BaoTree,
        shifted_filled_size: TreeNode,
        outboard: O,
        data: D,
        buffer: Vec<u8>,
        co: &'a Co<io::Result<Range<ChunkNum>>>,
        mode: HashMode,
    }

    impl<O: Outboard, D: ReadAt> RecursiveDataValidator<'_, O, D> {
        async fn validate(
            outboard: O,
            data: D,
            ranges: &ChunkRangesRef,
            co: &Co<io::Result<Range<ChunkNum>>>,
            mode: HashMode,
        ) -> io::Result<()> {
            let tree = outboard.tree();
            let mut buffer = vec![0u8; tree.chunk_group_bytes()];
            if tree.blocks() == 1 {
                // special case for a tree that fits in one block / chunk group
                let tmp = &mut buffer[..tree.size().try_into().unwrap()];
                data.read_exact_at(0, tmp)?;
                let actual = mode.hash_subtree(0, tmp, true);
                if actual == outboard.root() {
                    co.yield_(Ok(ChunkNum(0)..tree.chunks())).await;
                }
                return Ok(());
            }
            let ranges = truncate_ranges(ranges, tree.size());
            let root_hash = outboard.root();
            let (shifted_root, shifted_filled_size) = tree.shifted();
            let mut validator = RecursiveDataValidator {
                tree,
                shifted_filled_size,
                outboard,
                data,
                buffer,
                co,
                mode,
            };
            validator
                .validate_rec(&root_hash, shifted_root, true, ranges)
                .await
        }

        async fn yield_if_valid(
            &mut self,
            range: Range<u64>,
            hash: &blake3::Hash,
            is_root: bool,
        ) -> io::Result<()> {
            let len = (range.end - range.start).try_into().unwrap();
            let tmp = &mut self.buffer[..len];
            self.data.read_exact_at(range.start, tmp)?;
            // is_root is always false because the case of a single chunk group is handled before calling this function
            let actual = self
                .mode
                .hash_subtree(ChunkNum::full_chunks(range.start).0, tmp, is_root);
            if &actual == hash {
                // yield the left range
                self.co
                    .yield_(Ok(
                        ChunkNum::full_chunks(range.start)..ChunkNum::chunks(range.end)
                    ))
                    .await;
            }
            io::Result::Ok(())
        }

        fn validate_rec<'b>(
            &'b mut self,
            parent_hash: &'b blake3::Hash,
            shifted: TreeNode,
            is_root: bool,
            ranges: &'b ChunkRangesRef,
        ) -> LocalBoxFuture<'b, io::Result<()>> {
            Box::pin(async move {
                if ranges.is_empty() {
                    // this part of the tree is not of interest, so we can skip it
                    return Ok(());
                }
                let node = shifted.subtract_block_size(self.tree.block_size.0);
                let (l, m, r) = self.tree.leaf_byte_ranges3(node);
                if !self.tree.is_relevant_for_outboard(node) {
                    self.yield_if_valid(l..r, parent_hash, is_root).await?;
                    return Ok(());
                }
                let Some((l_hash, r_hash)) = self.outboard.load(node)? else {
                    // outboard is incomplete, we can't validate
                    return Ok(());
                };
                let actual = self.mode.parent_cv(&l_hash, &r_hash, is_root);
                if &actual != parent_hash {
                    // hash mismatch, we can't validate
                    return Ok(());
                };
                let (l_ranges, r_ranges) = split(ranges, node);
                if shifted.is_leaf() {
                    if !l_ranges.is_empty() {
                        self.yield_if_valid(l..m, &l_hash, false).await?;
                    }
                    if !r_ranges.is_empty() {
                        self.yield_if_valid(m..r, &r_hash, false).await?;
                    }
                } else {
                    // recurse (we are in the domain of the shifted tree)
                    let left = shifted.left_child().unwrap();
                    self.validate_rec(&l_hash, left, false, l_ranges).await?;
                    let right = shifted.right_descendant(self.shifted_filled_size).unwrap();
                    self.validate_rec(&r_hash, right, false, r_ranges).await?;
                }
                Ok(())
            })
        }
    }

    /// Given just an outboard, compute all valid ranges.
    ///
    /// This is not cheap since it recomputes the hashes for all chunks.
    pub fn valid_outboard_ranges<'a, O>(
        outboard: O,
        ranges: &'a ChunkRangesRef,
    ) -> impl IntoIterator<Item = io::Result<Range<ChunkNum>>> + 'a
    where
        O: Outboard + 'a,
    {
        valid_outboard_ranges_impl(outboard, ranges, HashMode::Standard)
    }

    /// Given just a keyed outboard, compute all valid ranges.
    ///
    /// This is not cheap since it recomputes the hashes for all chunks.
    pub fn keyed_valid_outboard_ranges<'a, O>(
        outboard: O,
        ranges: &'a ChunkRangesRef,
        key: &[u8; 32],
    ) -> impl IntoIterator<Item = io::Result<Range<ChunkNum>>> + 'a
    where
        O: Outboard + 'a,
    {
        valid_outboard_ranges_impl(outboard, ranges, HashMode::Keyed(*key))
    }

    fn valid_outboard_ranges_impl<'a, O>(
        outboard: O,
        ranges: &'a ChunkRangesRef,
        mode: HashMode,
    ) -> impl IntoIterator<Item = io::Result<Range<ChunkNum>>> + 'a
    where
        O: Outboard + 'a,
    {
        Gen::new(move |co| async move {
            if let Err(cause) =
                RecursiveOutboardValidator::validate(outboard, ranges, &co, mode).await
            {
                co.yield_(Err(cause)).await;
            }
        })
    }

    struct RecursiveOutboardValidator<'a, O: Outboard> {
        tree: BaoTree,
        shifted_filled_size: TreeNode,
        outboard: O,
        co: &'a Co<io::Result<Range<ChunkNum>>>,
        mode: HashMode,
    }

    impl<O: Outboard> RecursiveOutboardValidator<'_, O> {
        async fn validate(
            outboard: O,
            ranges: &ChunkRangesRef,
            co: &Co<io::Result<Range<ChunkNum>>>,
            mode: HashMode,
        ) -> io::Result<()> {
            let tree = outboard.tree();
            if tree.blocks() == 1 {
                // special case for a tree that fits in one block / chunk group
                co.yield_(Ok(ChunkNum(0)..tree.chunks())).await;
                return Ok(());
            }
            let ranges = truncate_ranges(ranges, tree.size());
            let root_hash = outboard.root();
            let (shifted_root, shifted_filled_size) = tree.shifted();
            let mut validator = RecursiveOutboardValidator {
                tree,
                shifted_filled_size,
                outboard,
                co,
                mode,
            };
            validator
                .validate_rec(&root_hash, shifted_root, true, ranges)
                .await
        }

        fn validate_rec<'b>(
            &'b mut self,
            parent_hash: &'b blake3::Hash,
            shifted: TreeNode,
            is_root: bool,
            ranges: &'b ChunkRangesRef,
        ) -> LocalBoxFuture<'b, io::Result<()>> {
            Box::pin(async move {
                let yield_node_range = |range: Range<u64>| {
                    self.co.yield_(Ok(
                        ChunkNum::full_chunks(range.start)..ChunkNum::chunks(range.end)
                    ))
                };
                if ranges.is_empty() {
                    // this part of the tree is not of interest, so we can skip it
                    return Ok(());
                }
                let node = shifted.subtract_block_size(self.tree.block_size.0);
                let (l, m, r) = self.tree.leaf_byte_ranges3(node);
                if !self.tree.is_relevant_for_outboard(node) {
                    yield_node_range(l..r).await;
                    return Ok(());
                }
                let Some((l_hash, r_hash)) = self.outboard.load(node)? else {
                    // outboard is incomplete, we can't validate
                    return Ok(());
                };
                let actual = self.mode.parent_cv(&l_hash, &r_hash, is_root);
                if &actual != parent_hash {
                    // hash mismatch, we can't validate
                    return Ok(());
                };
                let (l_ranges, r_ranges) = split(ranges, node);
                if shifted.is_leaf() {
                    if !l_ranges.is_empty() {
                        yield_node_range(l..m).await;
                    }
                    if !r_ranges.is_empty() {
                        yield_node_range(m..r).await;
                    }
                } else {
                    // recurse (we are in the domain of the shifted tree)
                    let left = shifted.left_child().unwrap();
                    self.validate_rec(&l_hash, left, false, l_ranges).await?;
                    let right = shifted.right_descendant(self.shifted_filled_size).unwrap();
                    self.validate_rec(&r_hash, right, false, r_ranges).await?;
                }
                Ok(())
            })
        }
    }
}
#[cfg(feature = "validate")]
pub use validate::{
    keyed_valid_outboard_ranges, keyed_valid_ranges, valid_outboard_ranges, valid_ranges,
};
