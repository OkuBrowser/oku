use super::{
    CustomFlatUnixFs, DirBuilder, Entry, Leaf, NamedLeaf, TreeConstructionFailed, TreeOptions,
};
use crate::Metadata;
use core::fmt;
use ipld_core::cid::{Cid, Version};
use std::collections::{HashMap, VecDeque};

use super::sharded::{self, ShardBlock};

/// Constructs the directory nodes required for a tree.
///
/// Implements the Iterator interface for owned values and the borrowed version, `next_borrowed`.
/// The tree is fully constructed once this has been exhausted.
pub struct PostOrderIterator {
    full_path: String,
    old_depth: usize,
    block_buffer: Vec<u8>,
    // our stack of pending work
    pending: Vec<Visited>,
    // "communication channel" from nested entries back to their parents; this hashmap is only used
    // in the event of mixed child nodes (leaves and nodes).
    persisted_cids: HashMap<u64, Vec<Option<NamedLeaf>>>,
    reused_children: Vec<Visited>,
    cid: Option<Cid>,
    total_size: u64,
    // interior HAMT shard blocks awaiting emission as anonymous nodes
    pending_blocks: VecDeque<ShardBlock>,
    // a sharded directory node held back until its interior shards (above) have been emitted, so a
    // post-order visit still yields the directory after its descendants
    deferred: Option<ShardBlock>,
    // from TreeOptions
    opts: TreeOptions,
}

/// The link list used to create the directory node. This list is created from a the BTreeMap
/// inside DirBuilder, and initially it will have `Some` values only for the initial leaves and
/// `None` values for subnodes which are not yet ready. At the time of use, this list is expected
/// to have only `Some` values.
type Leaves = Vec<Option<NamedLeaf>>;

/// The nodes in the visit. We need to do a post-order visit, which starts from a single
/// `DescentRoot`, followed by N `Descents` where N is the deepest directory in the tree. On each
/// descent, we'll need to first schedule a `Post` (or `PostRoot`) followed the immediate children
/// of the node. Directories are rendered when all of their direct and indirect descendants have
/// been serialized into NamedLeafs.
#[derive(Debug)]
enum Visited {
    // handle root differently not to infect with the Option<String> and Option<usize>
    DescentRoot(DirBuilder),
    Descent {
        node: DirBuilder,
        name: String,
        depth: usize,
        /// The index in the parents `Leaves` accessible through `PostOrderIterator::persisted_cids`.
        index: usize,
    },
    Post {
        parent_id: u64,
        depth: usize,
        name: String,
        index: usize,
        /// Leaves will be stored directly in this field when there are no DirBuilder descendants,
        /// in the `PostOrderIterator::persisted_cids` otherwise.
        leaves: LeafStorage,
        metadata: Metadata,
    },
    PostRoot {
        leaves: LeafStorage,
        metadata: Metadata,
    },
}

impl PostOrderIterator {
    pub(super) fn new(root: DirBuilder, opts: TreeOptions, longest_path: usize) -> Self {
        let root = Visited::DescentRoot(root);
        PostOrderIterator {
            full_path: String::with_capacity(longest_path),
            old_depth: 0,
            block_buffer: Default::default(),
            pending: vec![root],
            persisted_cids: Default::default(),
            reused_children: Vec::new(),
            cid: None,
            total_size: 0,
            pending_blocks: VecDeque::new(),
            deferred: None,
            opts,
        }
    }

    fn render_directory(
        links: &[Option<NamedLeaf>],
        buffer: &mut Vec<u8>,
        block_size_limit: &Option<u64>,
        cid_version: Version,
        hasher: multihash_codetable::Code,
        shard_threshold: &Option<u64>,
        metadata: &Metadata,
    ) -> Result<(Leaf, Vec<ShardBlock>), TreeConstructionFailed> {
        use crate::pb::{UnixFs, UnixFsType};
        use quick_protobuf::{MessageWrite, Writer};

        if let Some(threshold) = shard_threshold {
            let estimate = links
                .iter()
                .filter_map(|opt| opt.as_ref())
                .map(|NamedLeaf(name, cid, _)| (name.len() + cid.to_bytes().len()) as u64)
                .sum::<u64>();

            if estimate > *threshold {
                return sharded::build_sharded(links, buffer, cid_version, hasher, metadata);
            }
        }

        let (mode, mtime) = metadata.to_pb();
        let node = CustomFlatUnixFs {
            links,
            data: UnixFs {
                Type: UnixFsType::Directory,
                mode,
                mtime,
                ..Default::default()
            },
        };

        let size = node.get_size();

        if let Some(limit) = block_size_limit {
            let size = size as u64;
            if *limit < size {
                // FIXME: this could probably be detected at builder
                return Err(TreeConstructionFailed::TooLargeBlock(size));
            }
        }

        buffer.clear();
        buffer.reserve(size);

        let mut writer = Writer::new(&mut *buffer);
        node.write_message(&mut writer)
            .map_err(TreeConstructionFailed::Protobuf)?;

        let cid = crate::pb::make_cid(cid_version, hasher, crate::file::DAG_PB_CODEC, buffer);

        let combined_from_links = links
            .iter()
            .map(|opt| {
                opt.as_ref()
                    .map(|NamedLeaf(_, _, total_size)| total_size)
                    .unwrap()
            })
            .sum::<u64>();

        Ok((
            Leaf {
                link: cid,
                total_size: buffer.len() as u64 + combined_from_links,
            },
            Vec::new(),
        ))
    }

    /// Construct the next dag-pb node, if any.
    ///
    /// Returns a `TreeNode` of the latest constructed tree node.
    pub fn next_borrowed(&mut self) -> Option<Result<TreeNode<'_>, TreeConstructionFailed>> {
        if let Some(shard) = self.pending_blocks.pop_front() {
            self.block_buffer.clear();
            self.block_buffer.extend_from_slice(&shard.block);
            self.cid = Some(shard.cid);
            self.total_size = shard.total_size;

            return Some(Ok(TreeNode {
                path: "",
                cid: self.cid.as_ref().expect("just set"),
                total_size: self.total_size,
                block: &self.block_buffer,
            }));
        }

        if let Some(node) = self.deferred.take() {
            // full_path still holds this node's path: the interior shard drain above leaves it
            // untouched, and no pending entries have been popped since the node was rendered.
            self.block_buffer.clear();
            self.block_buffer.extend_from_slice(&node.block);
            self.cid = Some(node.cid);
            self.total_size = node.total_size;

            return Some(Ok(TreeNode {
                path: self.full_path.as_str(),
                cid: self.cid.as_ref().expect("just set"),
                total_size: self.total_size,
                block: &self.block_buffer,
            }));
        }

        while let Some(visited) = self.pending.pop() {
            let (name, depth) = match &visited {
                Visited::DescentRoot(_) => (None, 0),
                Visited::Descent { name, depth, .. } => (Some(name.as_ref()), *depth),
                Visited::Post { name, depth, .. } => (Some(name.as_ref()), *depth),
                Visited::PostRoot { .. } => (None, 0),
            };

            update_full_path((&mut self.full_path, &mut self.old_depth), name, depth);

            match visited {
                Visited::DescentRoot(node) => {
                    let children = &mut self.reused_children;
                    let metadata = node.metadata;
                    let leaves = partition_children_leaves(depth, node.nodes.into_iter(), children);
                    let any_children = !children.is_empty();

                    let leaves = if any_children {
                        self.persisted_cids.insert(node.id, leaves);
                        LeafStorage::from(node.id)
                    } else {
                        leaves.into()
                    };

                    self.pending.push(Visited::PostRoot { leaves, metadata });
                    self.pending.append(children);
                }
                Visited::Descent {
                    node,
                    name,
                    depth,
                    index,
                } => {
                    let children = &mut self.reused_children;
                    let metadata = node.metadata;
                    let parent_id = node.parent_id.expect("only roots parent_id is None");
                    let leaves = partition_children_leaves(depth, node.nodes.into_iter(), children);
                    let any_children = !children.is_empty();

                    let leaves = if any_children {
                        self.persisted_cids.insert(node.id, leaves);
                        node.id.into()
                    } else {
                        leaves.into()
                    };

                    self.pending.push(Visited::Post {
                        parent_id,
                        name,
                        depth,
                        leaves,
                        index,
                        metadata,
                    });

                    self.pending.append(children);
                }
                Visited::Post {
                    parent_id,
                    name,
                    leaves,
                    index,
                    metadata,
                    ..
                } => {
                    let leaves = leaves.into_inner(&mut self.persisted_cids);
                    let buffer = &mut self.block_buffer;

                    let (leaf, interior) = match Self::render_directory(
                        &leaves,
                        buffer,
                        &self.opts.block_size_limit,
                        self.opts.cid_version,
                        self.opts.hasher,
                        &self.opts.shard_threshold,
                        &metadata,
                    ) {
                        Ok(rendered) => rendered,
                        Err(e) => return Some(Err(e)),
                    };

                    self.cid = Some(leaf.link);
                    self.total_size = leaf.total_size;
                    let has_interior = !interior.is_empty();
                    self.pending_blocks.extend(interior);

                    {
                        // name is None only for wrap_with_directory, which cannot really be
                        // propagated up but still the parent_id is allowed to be None
                        let parent_leaves = self.persisted_cids.get_mut(&parent_id);

                        match (parent_id, parent_leaves, index) {
                            (pid, None, index) => {
                                panic!("leaves not found for parent_id = {pid} and index = {index}")
                            }
                            (_, Some(vec), index) => {
                                let cell = &mut vec[index];
                                // all
                                assert!(cell.is_none());
                                *cell = Some(NamedLeaf(name, leaf.link, leaf.total_size));
                            }
                        }
                    }

                    if has_interior {
                        self.deferred = Some(ShardBlock {
                            cid: leaf.link,
                            block: core::mem::take(&mut self.block_buffer),
                            total_size: leaf.total_size,
                        });
                        return self.next_borrowed();
                    }

                    return Some(Ok(TreeNode {
                        path: self.full_path.as_str(),
                        cid: self.cid.as_ref().unwrap(),
                        total_size: self.total_size,
                        block: &self.block_buffer,
                    }));
                }
                Visited::PostRoot { leaves, metadata } => {
                    let leaves = leaves.into_inner(&mut self.persisted_cids);

                    if !self.opts.wrap_with_directory {
                        break;
                    }

                    let buffer = &mut self.block_buffer;

                    let (leaf, interior) = match Self::render_directory(
                        &leaves,
                        buffer,
                        &self.opts.block_size_limit,
                        self.opts.cid_version,
                        self.opts.hasher,
                        &self.opts.shard_threshold,
                        &metadata,
                    ) {
                        Ok(rendered) => rendered,
                        Err(e) => return Some(Err(e)),
                    };

                    self.cid = Some(leaf.link);
                    self.total_size = leaf.total_size;
                    let has_interior = !interior.is_empty();
                    self.pending_blocks.extend(interior);

                    if has_interior {
                        self.deferred = Some(ShardBlock {
                            cid: leaf.link,
                            block: core::mem::take(&mut self.block_buffer),
                            total_size: leaf.total_size,
                        });
                        return self.next_borrowed();
                    }

                    return Some(Ok(TreeNode {
                        path: self.full_path.as_str(),
                        cid: self.cid.as_ref().unwrap(),
                        total_size: self.total_size,
                        block: &self.block_buffer,
                    }));
                }
            }
        }
        None
    }
}

impl Iterator for PostOrderIterator {
    type Item = Result<OwnedTreeNode, TreeConstructionFailed>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_borrowed()
            .map(|res| res.map(TreeNode::into_owned))
    }
}

/// Borrowed representation of a node in the tree.
pub struct TreeNode<'a> {
    /// Full path to the node.
    pub path: &'a str,
    /// The Cid of the document.
    pub cid: &'a Cid,
    /// Cumulative total size of the subtree in bytes.
    pub total_size: u64,
    /// Raw dag-pb document.
    pub block: &'a [u8],
}

impl fmt::Debug for TreeNode<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("TreeNode")
            .field("path", &format_args!("{:?}", self.path))
            .field("cid", &format_args!("{}", self.cid))
            .field("total_size", &self.total_size)
            .field("size", &self.block.len())
            .finish()
    }
}

impl TreeNode<'_> {
    /// Convert to an owned and detached representation.
    pub fn into_owned(self) -> OwnedTreeNode {
        OwnedTreeNode {
            path: self.path.to_owned(),
            cid: self.cid.to_owned(),
            total_size: self.total_size,
            block: self.block.into(),
        }
    }
}

/// Owned representation of a node in the tree.
pub struct OwnedTreeNode {
    /// Full path to the node.
    pub path: String,
    /// The Cid of the document.
    pub cid: Cid,
    /// Cumulative total size of the subtree in bytes.
    pub total_size: u64,
    /// Raw dag-pb document.
    pub block: Box<[u8]>,
}

fn update_full_path(
    (full_path, old_depth): (&mut String, &mut usize),
    name: Option<&str>,
    depth: usize,
) {
    if depth < 2 {
        // initially thought it might be a good idea to add a slash to all components; removing it made
        // it impossible to get back down to empty string, so fixing this for depths 0 and 1.
        full_path.clear();
        *old_depth = 0;
    } else {
        while *old_depth >= depth && *old_depth > 0 {
            // we now want to pop the last segment
            // this would be easier with PathBuf
            let slash_at = full_path.bytes().rposition(|ch| ch == b'/');
            if let Some(slash_at) = slash_at {
                if *old_depth == depth && Some(&full_path[(slash_at + 1)..]) == name {
                    // minor unmeasurable perf optimization:
                    // going from a/b/foo/zz => a/b/foo does not need to go through the a/b
                    return;
                }
                full_path.truncate(slash_at);
                *old_depth -= 1;
            } else {
                todo!(
                    "no last slash_at in {:?} yet {} >= {}",
                    full_path,
                    old_depth,
                    depth
                );
            }
        }
    }

    debug_assert!(*old_depth <= depth);

    if let Some(name) = name {
        if !full_path.is_empty() {
            full_path.push('/');
        }
        full_path.push_str(name);
        *old_depth += 1;
    }

    assert_eq!(*old_depth, depth);
}

/// Returns a Vec of the links in order with only the leaves, the given `children` will contain yet
/// incomplete nodes of the tree.
fn partition_children_leaves(
    depth: usize,
    it: impl Iterator<Item = (String, Entry)>,
    children: &mut Vec<Visited>,
) -> Leaves {
    let mut leaves = Vec::new();

    for (i, (k, v)) in it.enumerate() {
        match v {
            Entry::Directory(node) => {
                children.push(Visited::Descent {
                    node,
                    // this needs to be pushed down to update the full_path
                    name: k,
                    depth: depth + 1,
                    index: i,
                });

                // this will be overwritten later, but the order is fixed
                leaves.push(None);
            }
            Entry::Leaf(leaf) => leaves.push(Some(NamedLeaf(k, leaf.link, leaf.total_size))),
        }
    }

    leaves
}

#[derive(Debug)]
enum LeafStorage {
    Direct(Leaves),
    Stashed(u64),
}

impl LeafStorage {
    fn into_inner(self, stash: &mut HashMap<u64, Leaves>) -> Leaves {
        use LeafStorage::*;

        match self {
            Direct(leaves) => leaves,
            Stashed(id) => stash
                .remove(&id)
                .ok_or(id)
                .expect("leaves are either stashed or direct, must able to find with id"),
        }
    }
}

impl From<u64> for LeafStorage {
    fn from(key: u64) -> LeafStorage {
        LeafStorage::Stashed(key)
    }
}

impl From<Leaves> for LeafStorage {
    fn from(leaves: Leaves) -> LeafStorage {
        LeafStorage::Direct(leaves)
    }
}
