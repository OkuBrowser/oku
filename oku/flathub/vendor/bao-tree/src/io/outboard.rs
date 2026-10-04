//! Implementations of the outboard traits
//!
//! A number of implementations for the sync and async outboard traits are provided.
//! Implementations for in-memory outboards, for outboards where the data resides on disk,
//! and a special implementation [EmptyOutboard] that just ignores all writes.
use std::io;

use crate::{blake3, BaoTree, BlockSize, TreeNode};

/// An empty outboard, that just returns 0 hashes for all nodes.
///
/// Also allows you to write and will immediately discard the data, a bit like /dev/null
#[derive(Debug)]
pub struct EmptyOutboard {
    /// tree defining the geometry
    pub tree: BaoTree,
    /// root hash
    pub root: blake3::Hash,
}

impl crate::io::sync::Outboard for EmptyOutboard {
    fn root(&self) -> blake3::Hash {
        self.root
    }
    fn tree(&self) -> BaoTree {
        self.tree
    }
    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        Ok(if self.tree.is_relevant_for_outboard(node) {
            // behave as if it was an outboard file filled with 0s
            Some((blake3::Hash::from([0; 32]), blake3::Hash::from([0; 32])))
        } else {
            None
        })
    }
}

#[cfg(feature = "tokio_fsm")]
impl crate::io::fsm::Outboard for EmptyOutboard {
    fn root(&self) -> blake3::Hash {
        self.root
    }
    fn tree(&self) -> BaoTree {
        self.tree
    }
    async fn load(&mut self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        Ok(if self.tree.is_relevant_for_outboard(node) {
            // behave as if it was an outboard file filled with 0s
            Some((blake3::Hash::from([0; 32]), blake3::Hash::from([0; 32])))
        } else {
            None
        })
    }
}

impl crate::io::sync::OutboardMut for EmptyOutboard {
    fn save(&mut self, node: TreeNode, _pair: &(blake3::Hash, blake3::Hash)) -> io::Result<()> {
        if self.tree.is_relevant_for_outboard(node) {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid node for this outboard",
            ))
        }
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "tokio_fsm")]
impl crate::io::fsm::OutboardMut for EmptyOutboard {
    async fn save(
        &mut self,
        node: TreeNode,
        _pair: &(blake3::Hash, blake3::Hash),
    ) -> io::Result<()> {
        if self.tree.is_relevant_for_outboard(node) {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid node for this outboard",
            ))
        }
    }

    async fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A generic outboard in pre order
///
/// All fields are public, so an outboard does not enforce any invariants.
/// This is necessary since we want outboards to exist in an incomplete state
/// where data does not match the root hash.
///
/// Caution: unlike the outboard implementation in the bao crate, this
/// implementation does not assume an 8 byte size prefix.
#[derive(Debug, Clone)]
pub struct PreOrderOutboard<D = Vec<u8>> {
    /// root hash
    pub root: blake3::Hash,
    /// tree defining the data
    pub tree: BaoTree,
    /// hashes with length prefix
    pub data: D,
}

impl<R: Default> Default for PreOrderOutboard<R> {
    fn default() -> Self {
        Self {
            root: blake3::hash(&[]),
            tree: BaoTree::new(0, BlockSize::ZERO),
            data: Default::default(),
        }
    }
}

/// A generic outboard in post order
///
/// All fields are public, so an outboard does not enforce any invariants.
/// This is necessary since we want outboards to exist in an incomplete state
/// where data does not match the root hash.
#[derive(Debug, Clone)]
pub struct PostOrderOutboard<D = Vec<u8>> {
    /// root hash
    pub root: blake3::Hash,
    /// tree defining the data
    pub tree: BaoTree,
    /// hashes with length prefix
    pub data: D,
}

impl<D: Default> Default for PostOrderOutboard<D> {
    fn default() -> Self {
        Self {
            root: blake3::hash(&[]),
            tree: BaoTree::new(0, BlockSize::ZERO),
            data: Default::default(),
        }
    }
}

/// A post order outboard that is optimized for memory storage.
///
/// The [PostOrderOutboard] will work just fine using e.g. a [`Vec<u8>`] as storage.
/// However, it will not work for types such as [bytes::Bytes] or [bytes::BytesMut]
/// that do not implement the io traits.
///
/// The traits are implemented for fixed size slices or mutable slices, so
/// unlike the [PostOrderOutboard], you must make sure that the data is already
/// the right size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostOrderMemOutboard<T = Vec<u8>> {
    /// root hash
    pub root: blake3::Hash,
    /// tree defining the data
    pub tree: BaoTree,
    /// hashes without length suffix
    pub data: T,
}

impl<T: Default> Default for PostOrderMemOutboard<T> {
    fn default() -> Self {
        Self {
            root: blake3::hash(&[]),
            tree: BaoTree::new(0, BlockSize::ZERO),
            data: Default::default(),
        }
    }
}

impl PostOrderMemOutboard {
    /// Create a new outboard from `data` and a `block_size`.
    ///
    /// This will hash the data and create an outboard.
    ///
    /// It is just a shortcut that calls [crate::io::sync::outboard_post_order].
    pub fn create(data: impl AsRef<[u8]>, block_size: BlockSize) -> Self {
        let data = data.as_ref();
        let size = data.len() as u64;
        let tree = BaoTree::new(size, block_size);
        let mut outboard = Vec::with_capacity(tree.outboard_size().try_into().unwrap());
        let root = crate::io::sync::outboard_post_order(data, tree, &mut outboard).unwrap();
        Self {
            root,
            tree,
            data: outboard,
        }
    }

    /// Create a keyed outboard from `data` and a `block_size`.
    pub fn create_keyed(data: impl AsRef<[u8]>, block_size: BlockSize, key: &[u8; 32]) -> Self {
        let data = data.as_ref();
        let size = data.len() as u64;
        let tree = BaoTree::new(size, block_size);
        let mut outboard = Vec::with_capacity(tree.outboard_size().try_into().unwrap());
        let root =
            crate::io::sync::keyed_outboard_post_order(data, tree, &mut outboard, key).unwrap();
        Self {
            root,
            tree,
            data: outboard,
        }
    }

    /// returns the outboard data, with the length suffix.
    pub fn into_inner_with_suffix(self) -> Vec<u8> {
        let mut res = self.data;
        res.extend_from_slice(self.tree.size.to_le_bytes().as_slice());
        res
    }
}

impl<T> PostOrderMemOutboard<T> {
    /// Map the outboard data to a new type.
    pub fn map_data<F, U>(self, f: F) -> PostOrderMemOutboard<U>
    where
        F: FnOnce(T) -> U,
        U: AsRef<[u8]>,
    {
        PostOrderMemOutboard {
            root: self.root,
            tree: self.tree,
            data: f(self.data),
        }
    }

    /// Flip the outboard to pre order.
    pub fn flip(&self) -> PreOrderMemOutboard
    where
        T: AsRef<[u8]>,
    {
        let mut target = PreOrderMemOutboard {
            root: self.root,
            tree: self.tree,
            data: vec![0; self.tree.outboard_size().try_into().unwrap()],
        };
        crate::io::sync::copy(self, &mut target).unwrap();
        target
    }
}

impl<T: AsRef<[u8]>> crate::io::sync::Outboard for PostOrderMemOutboard<T> {
    fn root(&self) -> blake3::Hash {
        self.root
    }
    fn tree(&self) -> BaoTree {
        self.tree
    }
    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        Ok(load_post(&self.tree, self.data.as_ref(), node))
    }
}

#[cfg(feature = "tokio_fsm")]
impl<T: AsRef<[u8]>> crate::io::fsm::Outboard for PostOrderMemOutboard<T> {
    fn root(&self) -> blake3::Hash {
        self.root
    }
    fn tree(&self) -> BaoTree {
        self.tree
    }
    async fn load(&mut self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        Ok(load_post(&self.tree, self.data.as_ref(), node))
    }
}

impl<T: AsMut<[u8]>> crate::io::sync::OutboardMut for PostOrderMemOutboard<T> {
    fn save(&mut self, node: TreeNode, pair: &(blake3::Hash, blake3::Hash)) -> io::Result<()> {
        match self.tree.post_order_offset(node) {
            Some(offset) => {
                let offset = usize::try_from(offset.value() * 64).unwrap();
                let data = self.data.as_mut();
                data[offset..offset + 32].copy_from_slice(pair.0.as_bytes());
                data[offset + 32..offset + 64].copy_from_slice(pair.1.as_bytes());
                Ok(())
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid node for this outboard",
            )),
        }
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "tokio_fsm")]
impl<T: AsMut<[u8]>> crate::io::fsm::OutboardMut for PostOrderMemOutboard<T> {
    async fn save(
        &mut self,
        node: TreeNode,
        pair: &(blake3::Hash, blake3::Hash),
    ) -> io::Result<()> {
        match self.tree.post_order_offset(node) {
            Some(offset) => {
                let offset = usize::try_from(offset.value() * 64).unwrap();
                let data = self.data.as_mut();
                data[offset..offset + 32].copy_from_slice(pair.0.as_bytes());
                data[offset + 32..offset + 64].copy_from_slice(pair.1.as_bytes());
                Ok(())
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid node for this outboard",
            )),
        }
    }

    async fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn load_raw_post_mem(tree: &BaoTree, data: &[u8], node: TreeNode) -> Option<[u8; 64]> {
    let offset = tree.post_order_offset(node)?.value();
    let offset = usize::try_from(offset * 64).unwrap();
    let slice = &data[offset..offset + 64];
    Some(slice.try_into().unwrap())
}

fn load_post(tree: &BaoTree, data: &[u8], node: TreeNode) -> Option<(blake3::Hash, blake3::Hash)> {
    load_raw_post_mem(tree, data, node).map(parse_hash_pair)
}

/// A pre order outboard that is optimized for memory storage.
///
/// The traits are implemented for fixed size slices or mutable slices, so you
/// must make sure that the data is already the right size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreOrderMemOutboard<T = Vec<u8>> {
    /// root hash
    pub root: blake3::Hash,
    /// tree defining the data
    pub tree: BaoTree,
    /// hashes with length prefix
    pub data: T,
}

impl<T: Default> Default for PreOrderMemOutboard<T> {
    fn default() -> Self {
        Self {
            root: blake3::hash(&[]),
            tree: BaoTree::new(0, BlockSize::ZERO),
            data: Default::default(),
        }
    }
}

impl PreOrderMemOutboard {
    /// returns the outboard data, with the length prefix added.
    pub fn into_inner_with_prefix(self) -> Vec<u8> {
        let mut res = self.data;
        res.splice(0..0, self.tree.size.to_le_bytes());
        res
    }

    /// Create a new outboard from `data` and a `block_size`.
    ///
    /// This will hash the data and create an outboard
    pub fn create(data: impl AsRef<[u8]>, block_size: BlockSize) -> Self {
        let data = data.as_ref();
        let size = data.len() as u64;
        let tree = BaoTree::new(size, block_size);
        // the outboard impl for PreOrderMemOutboard requires just AsMut<[u8]>,
        // so the data must already be the right size.
        let outboard = vec![0u8; tree.outboard_size().try_into().unwrap()];
        let mut res = Self {
            root: blake3::Hash::from([0; 32]),
            tree,
            data: outboard,
        };
        let root = crate::io::sync::outboard(data, tree, &mut res).unwrap();
        res.root = root;
        res
    }

    /// Create a keyed outboard from `data` and a `block_size`.
    pub fn create_keyed(data: impl AsRef<[u8]>, block_size: BlockSize, key: &[u8; 32]) -> Self {
        let data = data.as_ref();
        let size = data.len() as u64;
        let tree = BaoTree::new(size, block_size);
        let outboard = vec![0u8; tree.outboard_size().try_into().unwrap()];
        let mut res = Self {
            root: blake3::Hash::from([0; 32]),
            tree,
            data: outboard,
        };
        let root = crate::io::sync::keyed_outboard(data, tree, &mut res, key).unwrap();
        res.root = root;
        res
    }
}

impl<T> PreOrderMemOutboard<T> {
    /// Map the outboard data to a new type.
    pub fn map_data<F, U>(self, f: F) -> PreOrderMemOutboard<U>
    where
        F: FnOnce(T) -> U,
        U: AsRef<[u8]>,
    {
        PreOrderMemOutboard {
            root: self.root,
            tree: self.tree,
            data: f(self.data),
        }
    }

    /// Flip the outboard to a post order outboard.
    pub fn flip(&self) -> PostOrderMemOutboard
    where
        T: AsRef<[u8]>,
    {
        let mut target = PostOrderMemOutboard {
            root: self.root,
            tree: self.tree,
            data: vec![0; self.tree.outboard_size().try_into().unwrap()],
        };
        crate::io::sync::copy(self, &mut target).unwrap();
        target
    }
}

impl<T: AsRef<[u8]>> crate::io::sync::Outboard for PreOrderMemOutboard<T> {
    fn root(&self) -> blake3::Hash {
        self.root
    }
    fn tree(&self) -> BaoTree {
        self.tree
    }
    fn load(&self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        Ok(load_pre(&self.tree, self.data.as_ref(), node))
    }
}

impl<T: AsMut<[u8]>> crate::io::sync::OutboardMut for PreOrderMemOutboard<T> {
    fn save(&mut self, node: TreeNode, pair: &(blake3::Hash, blake3::Hash)) -> io::Result<()> {
        match self.tree.pre_order_offset(node) {
            Some(offset) => {
                let offset_u64 = offset * 64;
                let offset = usize::try_from(offset_u64).unwrap();
                let data = self.data.as_mut();
                data[offset..offset + 32].copy_from_slice(pair.0.as_bytes());
                data[offset + 32..offset + 64].copy_from_slice(pair.1.as_bytes());
                Ok(())
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid node for this outboard",
            )),
        }
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "tokio_fsm")]
impl<T: AsRef<[u8]>> crate::io::fsm::Outboard for PreOrderMemOutboard<T> {
    fn root(&self) -> blake3::Hash {
        self.root
    }
    fn tree(&self) -> BaoTree {
        self.tree
    }
    async fn load(&mut self, node: TreeNode) -> io::Result<Option<(blake3::Hash, blake3::Hash)>> {
        Ok(load_raw_pre_mem(&self.tree, self.data.as_ref(), node).map(parse_hash_pair))
    }
}

#[cfg(feature = "tokio_fsm")]
impl<T: AsMut<[u8]>> crate::io::fsm::OutboardMut for PreOrderMemOutboard<T> {
    async fn save(
        &mut self,
        node: TreeNode,
        pair: &(blake3::Hash, blake3::Hash),
    ) -> io::Result<()> {
        match self.tree.pre_order_offset(node) {
            Some(offset) => {
                let offset_u64 = offset * 64;
                let offset = usize::try_from(offset_u64).unwrap();
                let data = self.data.as_mut();
                data[offset..offset + 32].copy_from_slice(pair.0.as_bytes());
                data[offset + 32..offset + 64].copy_from_slice(pair.1.as_bytes());
                Ok(())
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid node for this outboard",
            )),
        }
    }

    async fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn load_raw_pre_mem(tree: &BaoTree, data: &[u8], node: TreeNode) -> Option<[u8; 64]> {
    // this is a bit slow because pre_order_offset uses a loop.
    // pretty sure there is a way to write it as a single expression if you spend the time.
    // but profiling still has this in the nanosecond range, so this is unlikely to be a
    // bottleneck.
    let offset = tree.pre_order_offset(node)?;
    let offset = usize::try_from(offset * 64).unwrap();
    let slice = &data[offset..offset + 64];
    Some(slice.try_into().unwrap())
}

fn load_pre(tree: &BaoTree, data: &[u8], node: TreeNode) -> Option<(blake3::Hash, blake3::Hash)> {
    load_raw_pre_mem(tree, data, node).map(parse_hash_pair)
}

pub(crate) fn parse_hash_pair(buf: [u8; 64]) -> (blake3::Hash, blake3::Hash) {
    let l_hash = blake3::Hash::from(<[u8; 32]>::try_from(&buf[..32]).unwrap());
    let r_hash = blake3::Hash::from(<[u8; 32]>::try_from(&buf[32..]).unwrap());
    (l_hash, r_hash)
}
