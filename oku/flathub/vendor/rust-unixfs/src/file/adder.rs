use ipld_core::cid::{Cid, Version};
use multihash_codetable::Code;

use crate::pb::{FlatUnixFs, UnixFs, UnixFsType, WriteableCid};
use crate::Metadata;
use alloc::borrow::Cow;
use core::fmt;
use quick_protobuf::{MessageWrite, Writer, WriterBackend};

/// File tree builder. Implements [`core::default::Default`] which tracks the recent defaults.
///
/// Custom file tree builder can be created with [`FileAdder::builder()`] and configuring the
/// chunker and collector.
///
/// Current implementation maintains an internal buffer for the block creation and uses sha2-256 to
/// produce Cid version 1 with raw leaves (default) or version 0 links. Currently does not support
/// inline links.
#[derive(Default)]
pub struct FileAdder {
    chunker: Chunker,
    collector: Collector,
    block_buffer: Vec<u8>,
    config: Config,
    metadata: Metadata,
}

#[derive(Debug, Clone, Copy)]
struct Config {
    cid_version: Version,
    raw_leaves: bool,
    hasher: Code,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            cid_version: Version::V1,
            raw_leaves: true,
            hasher: Code::Sha2_256,
        }
    }
}

impl Config {
    fn cid_of(&self, codec: u64, bytes: &[u8]) -> Cid {
        crate::pb::make_cid(self.cid_version, self.hasher, codec, bytes)
    }

    fn cid_of_raw_leaf(&self, bytes: &[u8]) -> Cid {
        crate::pb::make_cid(Version::V1, self.hasher, crate::file::RAW_LEAF_CODEC, bytes)
    }
}

impl fmt::Debug for FileAdder {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "FileAdder {{ chunker: {:?}, block_buffer: {}/{} }}",
            self.chunker,
            self.block_buffer.len(),
            self.block_buffer.capacity(),
        )
    }
}

/// Represents an intermediate structure which will be serialized into link blocks as both PBLink
/// and UnixFs::blocksize. Also holds `depth`, which helps with compaction of the link blocks.
#[derive(Clone)]
struct Link {
    /// Depth of this link. Zero is leaf, and anything above it is, at least for
    /// [`BalancedCollector`], the compacted link blocks.
    depth: usize,
    /// The link target
    target: Cid,
    /// Total size is dag-pb specific part of the link: aggregated size of the linked subtree.
    total_size: u64,
    /// File size is the unixfs specific blocksize for this link. In UnixFs link blocks, there is a
    /// UnixFs::blocksizes item for each link.
    file_size: u64,
}

impl fmt::Debug for Link {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Link")
            .field("depth", &self.depth)
            .field("target", &format_args!("{}", self.target))
            .field("total_size", &self.total_size)
            .field("file_size", &self.file_size)
            .finish()
    }
}

/// Convenience type to facilitate configuring [`FileAdder`]s.
pub struct FileAdderBuilder {
    chunker: Chunker,
    collector: Collector,
    cid_version: Version,
    raw_leaves: Option<bool>,
    metadata: Metadata,
    hasher: Code,
}

impl Default for FileAdderBuilder {
    fn default() -> Self {
        FileAdderBuilder {
            chunker: Chunker::default(),
            collector: Collector::default(),
            cid_version: Version::V1,
            raw_leaves: None,
            metadata: Metadata::default(),
            hasher: Code::Sha2_256,
        }
    }
}

impl FileAdderBuilder {
    /// Configures the builder to use the given chunker.
    pub fn with_chunker(self, chunker: Chunker) -> Self {
        FileAdderBuilder { chunker, ..self }
    }

    /// Configures the builder to use the given collector or layout.
    pub fn with_collector(self, collector: impl Into<Collector>) -> Self {
        FileAdderBuilder {
            collector: collector.into(),
            ..self
        }
    }

    /// Sets the CID version of produced dag-pb nodes.
    pub fn with_cid_version(self, cid_version: Version) -> Self {
        FileAdderBuilder {
            cid_version,
            ..self
        }
    }

    /// Stores file leaves as bare raw (0x55) blocks instead of dag-pb file nodes. When left unset,
    /// raw leaves are enabled for [`Version::V1`] and disabled for [`Version::V0`].
    pub fn with_raw_leaves(self, raw_leaves: bool) -> Self {
        FileAdderBuilder {
            raw_leaves: Some(raw_leaves),
            ..self
        }
    }

    /// Sets the metadata to be applied
    pub fn with_metadata(self, metadata: Metadata) -> Self {
        FileAdderBuilder { metadata, ..self }
    }

    /// Sets the multihash used for produced links.
    pub fn with_hasher(self, hasher: Code) -> Self {
        FileAdderBuilder { hasher, ..self }
    }

    /// Returns a new FileAdder
    pub fn build(self) -> FileAdder {
        let FileAdderBuilder {
            chunker,
            collector,
            cid_version,
            raw_leaves,
            metadata,
            hasher,
        } = self;

        // A non-sha2-256 hash cannot be represented as CIDv0, so it forces CIDv1; raw leaves then
        // default on as they do for any V1.
        let cid_version = if hasher == Code::Sha2_256 {
            cid_version
        } else {
            Version::V1
        };
        let raw_leaves = raw_leaves.unwrap_or(matches!(cid_version, Version::V1));

        FileAdder {
            chunker,
            collector,
            config: Config {
                cid_version,
                raw_leaves,
                hasher,
            },
            metadata,
            ..Default::default()
        }
    }
}

impl FileAdder {
    /// Returns a [`FileAdderBuilder`] for creating a non-default FileAdder.
    pub fn builder() -> FileAdderBuilder {
        FileAdderBuilder::default()
    }

    /// Returns the likely amount of buffering the file adding will work with best.
    ///
    /// When using the size based chunker and input larger than or equal to the hint is `push()`'ed
    /// to the chunker, the internal buffer will not be used.
    pub fn size_hint(&self) -> usize {
        self.chunker.size_hint()
    }

    /// Called to push new file bytes into the tree builder.
    ///
    /// Returns the newly created blocks (at most 2) and their respective Cids, and the amount of
    /// `input` consumed.
    pub fn push(&mut self, input: &[u8]) -> (impl Iterator<Item = (Cid, Vec<u8>)>, usize) {
        let (accepted, ready) = self.chunker.accept(input, &self.block_buffer);

        let (blocks, consumed) = if self.block_buffer.is_empty() && ready {
            // the caller is giving us whole chunks, no internal buffering needed
            (self.flush_leaf(accepted), accepted.len())
        } else {
            // slower path as we manage the buffer.

            if self.block_buffer.capacity() == 0 {
                // delay the internal buffer creation until this point, as the caller clearly wants
                // to use it.
                self.block_buffer.reserve(self.size_hint());
            }

            self.block_buffer.extend_from_slice(accepted);
            let written = accepted.len();

            if !ready {
                // a new block did not become ready, which means we couldn't have gotten a new cid.
                (Vec::new(), written)
            } else {
                let buffered = core::mem::take(&mut self.block_buffer);
                let blocks = self.flush_leaf(&buffered);
                self.block_buffer = buffered;
                self.block_buffer.clear();
                (blocks, written)
            }
        };

        (blocks.into_iter(), consumed)
    }

    /// Renders a leaf for the given input and pushes it into the collector, returning the leaf block
    /// followed by any link blocks the push produced.
    fn flush_leaf(&mut self, input: &[u8]) -> Vec<(Cid, Vec<u8>)> {
        let has_prior = self.collector.has_links();
        let (cid, block, link) = Self::flush_buffered_leaf(input, has_prior, false, self.config)
            .expect("chunk completed, must produce a new block");
        let mut blocks = vec![(cid, block)];
        blocks.extend(self.collector.push_link(link, self.config));
        blocks
    }

    /// Called after the last [`FileAdder::push`] to finish the tree construction.
    ///
    /// Returns a list of Cids and their respective blocks.
    ///
    /// Note: the API will hopefully evolve in a direction which will not allocate a new Vec for
    /// every block in the near-ish future.
    pub fn finish(mut self) -> impl Iterator<Item = (Cid, Vec<u8>)> {
        let has_prior = self.collector.has_links();
        let buffered = core::mem::take(&mut self.block_buffer);
        let last_leaf = Self::flush_buffered_leaf(&buffered, has_prior, true, self.config);

        let mut blocks = Vec::new();
        if let Some((cid, block, link)) = last_leaf {
            blocks.push((cid, block));
            blocks.extend(self.collector.push_link(link, self.config));
        }
        blocks.extend(self.collector.finish(self.config));

        if self.metadata.mode().is_some() || self.metadata.mtime().is_some() {
            if let Some((cid, block)) = blocks.last_mut() {
                let (new_cid, new_block) =
                    apply_root_metadata(*cid, block, self.config, &self.metadata);
                *cid = new_cid;
                *block = new_block;
            }
        }

        blocks.into_iter()
    }

    /// Returns `None` when the input is empty but there are links, otherwise a new Cid and a
    /// block.
    fn flush_buffered_leaf(
        input: &[u8],
        has_prior_leaf: bool,
        finishing: bool,
        config: Config,
    ) -> Option<(Cid, Vec<u8>, Link)> {
        if input.is_empty() && (!finishing || has_prior_leaf) {
            return None;
        }

        let (cid, block, total_size) = if config.raw_leaves {
            let cid = config.cid_of_raw_leaf(input);
            (cid, input.to_vec(), input.len() as u64)
        } else {
            // for empty unixfs file the bytes is missing but filesize is present.
            let data = if !input.is_empty() {
                Some(Cow::Borrowed(input))
            } else {
                None
            };

            let inner = FlatUnixFs {
                links: Vec::new(),
                data: UnixFs {
                    Type: UnixFsType::File,
                    Data: data,
                    filesize: Some(input.len() as u64),
                    // no blocksizes as there are no links
                    ..Default::default()
                },
            };

            let (cid, vec) = render_and_hash(&inner, config);
            let total_size = vec.len() as u64;
            (cid, vec, total_size)
        };

        let link = Link {
            depth: 0,
            target: cid,
            total_size,
            file_size: input.len() as u64,
        };

        Some((cid, block, link))
    }

    /// Test helper for collecting all of the produced blocks; probably not a good idea outside
    /// smaller test cases. When `amt` is zero, the whole content is processed at the speed of
    /// chunker, otherwise `all_content` is pushed at `amt` sized slices with the idea of catching
    /// bugs in chunkers.
    #[cfg(test)]
    fn collect_blocks(mut self, all_content: &[u8], mut amt: usize) -> Vec<(Cid, Vec<u8>)> {
        let mut written = 0;
        let mut blocks_received = Vec::new();

        if amt == 0 {
            amt = all_content.len();
        }

        while written < all_content.len() {
            let end = written + (all_content.len() - written).min(amt);
            let slice = &all_content[written..end];

            let (blocks, pushed) = self.push(slice);
            blocks_received.extend(blocks);
            written += pushed;
        }

        let last_blocks = self.finish();
        blocks_received.extend(last_blocks);

        blocks_received
    }
}

fn render_and_hash<M: MessageWrite>(node: &M, config: Config) -> (Cid, Vec<u8>) {
    let mut out = Vec::with_capacity(node.get_size());
    let mut writer = Writer::new(&mut out);
    node.write_message(&mut writer)
        .expect("unsure how this could fail");
    let cid = config.cid_of(crate::file::DAG_PB_CODEC, &out);
    (cid, out)
}

/// A child link of a UnixFS file branch node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileBranchLink {
    /// Target block.
    pub cid: Cid,
    /// Content bytes this child covers (the UnixFS blocksize).
    pub blocksize: u64,
    /// Cumulative dag size of the child (the dag-pb link Tsize).
    pub tsize: u64,
}

/// A decoded UnixFS file branch node (a `File` node with links).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileBranch {
    /// Total content size of the subtree.
    pub filesize: u64,
    /// Immediate child links, in order.
    pub links: Vec<FileBranchLink>,
}

/// Decodes a UnixFS file branch node, or `None` for a leaf, a non-file, or a metadata-carrying node.
pub fn parse_file_branch(block: &[u8]) -> Option<FileBranch> {
    let flat = FlatUnixFs::try_parse(block).ok()?;
    if flat.data.Type != UnixFsType::File
        || flat.links.is_empty()
        || flat.links.len() != flat.data.blocksizes.len()
        || flat.data.mode.is_some()
        || flat.data.mtime.is_some()
    {
        return None;
    }

    let mut links = Vec::with_capacity(flat.links.len());
    for (link, &blocksize) in flat.links.iter().zip(flat.data.blocksizes.iter()) {
        let cid = Cid::try_from(link.Hash.as_deref()?).ok()?;
        links.push(FileBranchLink {
            cid,
            blocksize,
            tsize: link.Tsize.unwrap_or_default(),
        });
    }

    Some(FileBranch {
        filesize: flat.data.filesize.unwrap_or_default(),
        links,
    })
}

/// Renders a single file branch node from the given links, byte-identical to [`FileAdder`].
pub fn rebuild_file_branch(
    filesize: u64,
    links: &[FileBranchLink],
    cid_version: Version,
    hasher: Code,
) -> (Cid, Vec<u8>) {
    let config = Config {
        cid_version,
        raw_leaves: true,
        hasher,
    };
    let links: Vec<Link> = links
        .iter()
        .map(|l| Link {
            depth: 0,
            target: l.cid,
            total_size: l.tsize,
            file_size: l.blocksize,
        })
        .collect();
    render_and_hash(
        &LinkBlock {
            links: &links,
            filesize,
        },
        config,
    )
}

/// Builds a balanced file tree over the given leaf links, returning the root cid, its cumulative
/// size, and the branch blocks (the leaves are not re-emitted). Byte-identical to [`FileAdder`].
pub fn build_file_from_leaves(
    leaves: &[FileBranchLink],
    cid_version: Version,
    hasher: Code,
) -> (Cid, u64, Vec<(Cid, Vec<u8>)>) {
    assert!(!leaves.is_empty(), "at least one leaf is required");
    let config = Config {
        cid_version,
        raw_leaves: true,
        hasher,
    };
    let mut collector = BalancedCollector::default();
    let mut blocks = Vec::new();
    for leaf in leaves {
        let link = Link {
            depth: 0,
            target: leaf.cid,
            total_size: leaf.tsize,
            file_size: leaf.blocksize,
        };
        blocks.extend(collector.push_link(link, config));
    }
    blocks.extend(collector.finish(config));

    match blocks.last() {
        Some((cid, _)) => {
            let leaf_tsize: u64 = leaves.iter().map(|l| l.tsize).sum();
            let branch_tsize: u64 = blocks.iter().map(|(_, b)| b.len() as u64).sum();
            (*cid, leaf_tsize + branch_tsize, blocks)
        }
        None => (leaves[0].cid, leaves[0].tsize, blocks),
    }
}

/// Streaming dag-pb serializer for a file link block (a `File` node linking child blocks), avoiding
/// the per-link byte-vector allocations of building intermediate `PBLink`s.
struct LinkBlock<'a> {
    links: &'a [Link],
    filesize: u64,
}

impl MessageWrite for LinkBlock<'_> {
    fn get_size(&self) -> usize {
        use quick_protobuf::sizeofs::sizeof_len;
        let links = self
            .links
            .iter()
            .map(|link| 1 + sizeof_len(LinkAsPBLink(link).get_size()))
            .sum::<usize>();
        let data = LinkBlockData {
            filesize: self.filesize,
            links: self.links,
        };
        links + 1 + sizeof_len(data.get_size())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> quick_protobuf::Result<()> {
        for link in self.links {
            w.write_with_tag(18, |w| w.write_message(&LinkAsPBLink(link)))?;
        }
        let data = LinkBlockData {
            filesize: self.filesize,
            links: self.links,
        };
        w.write_with_tag(10, |w| w.write_message(&data))
    }
}

struct LinkAsPBLink<'a>(&'a Link);

impl MessageWrite for LinkAsPBLink<'_> {
    fn get_size(&self) -> usize {
        use quick_protobuf::sizeofs::*;
        1 + sizeof_len(WriteableCid(&self.0.target).get_size())
            + 1
            + sizeof_len(0)
            + 1
            + sizeof_varint(self.0.total_size)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> quick_protobuf::Result<()> {
        w.write_with_tag(10, |w| w.write_message(&WriteableCid(&self.0.target)))?;
        w.write_with_tag(18, |w| w.write_string(""))?;
        w.write_with_tag(24, |w| w.write_uint64(self.0.total_size))
    }
}

struct LinkBlockData<'a> {
    filesize: u64,
    links: &'a [Link],
}

impl MessageWrite for LinkBlockData<'_> {
    fn get_size(&self) -> usize {
        use quick_protobuf::sizeofs::*;
        1 + sizeof_varint(UnixFsType::File as u64)
            + 1
            + sizeof_varint(self.filesize)
            + self
                .links
                .iter()
                .map(|link| 1 + sizeof_varint(link.file_size))
                .sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> quick_protobuf::Result<()> {
        w.write_with_tag(8, |w| w.write_enum(UnixFsType::File as i32))?;
        w.write_with_tag(24, |w| w.write_uint64(self.filesize))?;
        for link in self.links {
            w.write_with_tag(32, |w| w.write_uint64(link.file_size))?;
        }
        Ok(())
    }
}

fn apply_root_metadata(
    cid: Cid,
    block: &[u8],
    config: Config,
    metadata: &Metadata,
) -> (Cid, Vec<u8>) {
    let (mode, mtime) = metadata.to_pb();

    if cid.codec() == crate::file::RAW_LEAF_CODEC {
        let inner = FlatUnixFs {
            links: Vec::new(),
            data: UnixFs {
                Type: UnixFsType::File,
                Data: (!block.is_empty()).then_some(Cow::Borrowed(block)),
                filesize: Some(block.len() as u64),
                mode,
                mtime,
                ..Default::default()
            },
        };
        render_and_hash(&inner, config)
    } else {
        let mut flat = FlatUnixFs::try_from(block).expect("file root must be a dag-pb node");
        flat.data.mode = mode;
        flat.data.mtime = mtime;
        render_and_hash(&flat, config)
    }
}

/// Chunker strategy
#[derive(Debug, Clone, Copy)]
pub enum Chunker {
    /// Size based chunking
    Size(usize),
}

impl Default for Chunker {
    /// Returns a default chunker
    fn default() -> Self {
        Chunker::Size(256 * 1024)
    }
}

impl Chunker {
    fn accept<'a>(&mut self, input: &'a [u8], buffered: &[u8]) -> (&'a [u8], bool) {
        use Chunker::*;

        match self {
            Size(max) => {
                let l = input.len().min(*max - buffered.len());
                let accepted = &input[..l];
                let ready = buffered.len() + l >= *max;
                (accepted, ready)
            }
        }
    }

    fn size_hint(&self) -> usize {
        use Chunker::*;

        match self {
            Size(max) => *max,
        }
    }
}

/// Collector or layout strategy. For more information, see the [Layout section of the spec].
/// Currently only the default balanced collector/layout has been implemented.
///
/// [Layout section of the spec]: https://github.com/ipfs/specs/blob/master/UNIXFS.md#layout
#[derive(Debug, Clone)]
pub enum Collector {
    /// Balanced trees.
    Balanced(BalancedCollector),
}

impl Default for Collector {
    fn default() -> Self {
        Collector::Balanced(Default::default())
    }
}

impl Collector {
    fn has_links(&self) -> bool {
        match self {
            Collector::Balanced(bc) => bc.has_links(),
        }
    }

    fn push_link(&mut self, link: Link, config: Config) -> Vec<(Cid, Vec<u8>)> {
        match self {
            Collector::Balanced(bc) => bc.push_link(link, config),
        }
    }

    fn finish(&mut self, config: Config) -> Vec<(Cid, Vec<u8>)> {
        match self {
            Collector::Balanced(bc) => bc.finish(config),
        }
    }
}

/// BalancedCollector creates balanced UnixFs trees, most optimized for random access to different
/// parts of the file. Currently supports only link count threshold or the branching factor.
#[derive(Clone)]
pub struct BalancedCollector {
    branching_factor: usize,
    // pending links per depth (0 == leaves); a layer is compacted into the next once it grows past
    // the branching factor
    layers: Vec<Vec<Link>>,
}

impl fmt::Debug for BalancedCollector {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "BalancedCollector {{ branching_factor: {} }}",
            self.branching_factor
        )
    }
}

impl Default for BalancedCollector {
    /// Returns a default collector which matches go-ipfs 0.6
    ///
    /// The origin for 174 is not described in the the [specs], but has likely to do something
    /// with being "good enough" regarding prefetching when reading and allows reusing some of the
    /// link blocks if parts of a longer file change.
    ///
    /// [specs]: https://github.com/ipfs/specs/blob/master/UNIXFS.md
    fn default() -> Self {
        Self::with_branching_factor(174)
    }
}

impl From<BalancedCollector> for Collector {
    fn from(b: BalancedCollector) -> Self {
        Collector::Balanced(b)
    }
}

impl BalancedCollector {
    /// Configure Balanced collector with the given branching factor.
    pub fn with_branching_factor(branching_factor: usize) -> Self {
        assert!(branching_factor > 0);

        Self {
            branching_factor,
            layers: Vec::new(),
        }
    }

    fn has_links(&self) -> bool {
        self.layers.iter().any(|layer| !layer.is_empty())
    }

    /// Pushes one link to the bottom layer, compacting any layer that grows past the branching
    /// factor into a single link in the layer above. Keeping per-depth buffers makes this O(n)
    /// overall rather than rescanning one flat vec on every call.
    fn push_link(&mut self, link: Link, config: Config) -> Vec<(Cid, Vec<u8>)> {
        let mut ret = Vec::new();

        if self.layers.is_empty() {
            self.layers.push(Vec::new());
        }
        self.layers[0].push(link);

        let mut depth = 0;
        while self.layers[depth].len() > self.branching_factor {
            let count = self.branching_factor;
            let (cid, block, promoted) = self.render(depth, count, config);
            ret.push((cid, block));

            if depth + 1 == self.layers.len() {
                self.layers.push(Vec::new());
            }
            self.layers[depth + 1].push(promoted);
            depth += 1;
        }

        ret
    }

    /// Drains the remaining links bottom-up into a single root. A lone remaining link is the root
    /// itself (e.g. a single chunk file) and is not wrapped any further.
    fn finish(&mut self, config: Config) -> Vec<(Cid, Vec<u8>)> {
        let mut ret = Vec::new();

        loop {
            let total = self.layers.iter().map(|layer| layer.len()).sum::<usize>();
            if total <= 1 {
                break;
            }

            let depth = self
                .layers
                .iter()
                .position(|layer| !layer.is_empty())
                .expect("total > 1 so some layer is non-empty");
            let count = self.layers[depth].len().min(self.branching_factor);
            let (cid, block, promoted) = self.render(depth, count, config);
            ret.push((cid, block));

            if depth + 1 == self.layers.len() {
                self.layers.push(Vec::new());
            }
            self.layers[depth + 1].push(promoted);
        }

        ret
    }

    /// Renders the first `count` links of `layers[depth]` into a single dag-pb file link block,
    /// removing them, and returns the block plus the promoted link at `depth + 1`.
    fn render(&mut self, depth: usize, count: usize, config: Config) -> (Cid, Vec<u8>, Link) {
        let links = &self.layers[depth][..count];
        let filesize = links.iter().map(|link| link.file_size).sum::<u64>();
        let nested_total_size = links.iter().map(|link| link.total_size).sum::<u64>();

        let (cid, vec) = render_and_hash(&LinkBlock { links, filesize }, config);

        self.layers[depth].drain(0..count);

        let promoted = Link {
            depth: depth + 1,
            target: cid,
            total_size: nested_total_size + vec.len() as u64,
            file_size: filesize,
        };

        (cid, vec, promoted)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_file_branch, rebuild_file_branch, BalancedCollector, Chunker, FileAdder};
    use crate::test_support::FakeBlockstore;
    use core::convert::TryFrom;
    use hex_literal::hex;

    #[test]
    fn rebuild_file_branch_matches_adder_root() {
        use ipld_core::cid::Version;
        use multihash_codetable::Code;
        let content: &[u8] = b"the quick brown fox jumps over the lazy dog";
        let blocks = FileAdder::builder()
            .with_chunker(Chunker::Size(4))
            .with_cid_version(Version::V1)
            .build()
            .collect_blocks(content, 0);
        let (root_cid, root_block) = blocks.last().unwrap().clone();
        let branch = parse_file_branch(&root_block).expect("multiblock root is a file branch");
        let (rebuilt, _) =
            rebuild_file_branch(branch.filesize, &branch.links, Version::V1, Code::Sha2_256);
        assert_eq!(rebuilt, root_cid);
    }

    #[test]
    fn build_file_from_leaves_matches_adder() {
        use super::{build_file_from_leaves, FileBranchLink};
        use ipld_core::cid::Version;
        use multihash_codetable::Code;

        let content: Vec<u8> = (0..1500u32).flat_map(|i| i.to_le_bytes()).collect();
        let adder_root = FileAdder::builder()
            .with_chunker(Chunker::Size(7))
            .with_cid_version(Version::V1)
            .build()
            .collect_blocks(&content, 0)
            .last()
            .unwrap()
            .0;

        let leaves: Vec<FileBranchLink> = content
            .chunks(7)
            .map(|c| FileBranchLink {
                cid: crate::pb::make_cid(Version::V1, Code::Sha2_256, 0x55, c),
                blocksize: c.len() as u64,
                tsize: c.len() as u64,
            })
            .collect();
        let (root, _, _) = build_file_from_leaves(&leaves, Version::V1, Code::Sha2_256);
        assert_eq!(root, adder_root);
    }

    #[test]
    fn parse_file_branch_rejects_metadata_root() {
        use crate::Metadata;
        use ipld_core::cid::Version;
        let content: &[u8] = b"the quick brown fox jumps over the lazy dog";
        let blocks = FileAdder::builder()
            .with_chunker(Chunker::Size(4))
            .with_cid_version(Version::V1)
            .with_metadata(Metadata::default().with_mode(0o644))
            .build()
            .collect_blocks(content, 0);
        let root_block = blocks.last().unwrap().1.clone();
        assert!(parse_file_branch(&root_block).is_none());
    }
    use ipld_core::cid::{Cid, Version};

    #[test]
    fn test_size_chunker() {
        assert_eq!(size_chunker_scenario(1, 4, 0), (1, true));
        assert_eq!(size_chunker_scenario(2, 4, 0), (2, true));
        assert_eq!(size_chunker_scenario(2, 1, 0), (1, false));
        assert_eq!(size_chunker_scenario(2, 1, 1), (1, true));
        assert_eq!(size_chunker_scenario(32, 3, 29), (3, true));
        // this took some debugging time:
        assert_eq!(size_chunker_scenario(32, 4, 29), (3, true));
    }

    fn size_chunker_scenario(max: usize, input_len: usize, existing_len: usize) -> (usize, bool) {
        let input = vec![0; input_len];
        let existing = vec![0; existing_len];

        let (accepted, ready) = Chunker::Size(max).accept(&input, &existing);
        (accepted.len(), ready)
    }

    #[test]
    fn cidv1_raw_single_leaf_is_root() {
        use ipld_core::cid::Version;

        let content: &[u8] = b"foobar\n";
        let blocks = FileAdder::builder()
            .with_cid_version(Version::V1)
            .build()
            .collect_blocks(content, 0);

        assert_eq!(blocks.len(), 1);
        let (cid, block) = &blocks[0];
        assert_eq!(cid.version(), Version::V1);
        assert_eq!(cid.codec(), 0x55);
        assert_eq!(block.as_slice(), content);
    }

    #[test]
    fn default_is_cidv1_raw_leaves() {
        let single = FileAdder::default().collect_blocks(b"foobar\n", 0);
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].0.version(), Version::V1);
        assert_eq!(
            single[0].0.codec(),
            0x55,
            "a single-chunk file is a bare raw leaf"
        );

        let multi = FileAdder::builder()
            .with_chunker(Chunker::Size(2))
            .build()
            .collect_blocks(b"foobar\n", 0);
        assert!(multi.len() > 1);
        assert!(multi.iter().all(|(cid, _)| cid.version() == Version::V1));
        assert_eq!(
            multi.last().unwrap().0.codec(),
            0x70,
            "root is a dag-pb node"
        );
        assert!(
            multi[..multi.len() - 1]
                .iter()
                .any(|(cid, _)| cid.codec() == 0x55),
            "leaves are raw"
        );
    }

    #[test]
    fn with_hasher_blake3() {
        use ipld_core::cid::Version;
        use multihash_codetable::{Code, MultihashDigest};

        let content: &[u8] = b"foobar\n";

        // A non-sha2-256 hash implies CIDv1 and raw leaves even without setting cid_version.
        let blocks = FileAdder::builder()
            .with_hasher(Code::Blake3_256)
            .build()
            .collect_blocks(content, 0);

        assert_eq!(blocks.len(), 1);
        let (cid, block) = &blocks[0];
        assert_eq!(cid.version(), Version::V1);
        assert_eq!(cid.codec(), 0x55);
        assert_eq!(block.as_slice(), content);
        assert_eq!(*cid.hash(), Code::Blake3_256.digest(content));

        // Multi-block: every produced node (leaves and the dag-pb root) is hashed with blake3.
        let blocks = FileAdder::builder()
            .with_hasher(Code::Blake3_256)
            .with_chunker(Chunker::Size(2))
            .build()
            .collect_blocks(content, 0);

        assert!(blocks.len() > 1);
        for (cid, block) in &blocks {
            assert_eq!(cid.version(), Version::V1);
            assert_eq!(*cid.hash(), Code::Blake3_256.digest(block));
        }
    }

    #[test]
    fn file_root_metadata() {
        use crate::Metadata;
        use ipld_core::cid::Version;

        let content: &[u8] = b"foobar\n";
        let md = Metadata::new(Some(0o100644), Some((1_700_000_000, 0)));

        // multi-block (dag-pb leaves): the root is a dag-pb file link node, re-rendered with metadata
        let blocks = FileAdder::builder()
            .with_chunker(Chunker::Size(2))
            .with_metadata(md.clone())
            .build()
            .collect_blocks(content, 0);
        let (root_cid, root_block) = blocks.last().unwrap();
        assert_eq!(root_cid.codec(), 0x70);
        let parsed = crate::pb::FlatUnixFs::try_parse(root_block).unwrap();
        assert_eq!(parsed.data.mode, Some(0o100644));
        let mtime = parsed.data.mtime.as_ref().unwrap();
        assert_eq!(mtime.Seconds, 1_700_000_000);
        assert_eq!(mtime.FractionalNanoseconds, None);

        // single raw leaf + metadata must convert to a dag-pb file node to hold it
        let blocks = FileAdder::builder()
            .with_cid_version(Version::V1)
            .with_metadata(md)
            .build()
            .collect_blocks(content, 0);
        assert_eq!(blocks.len(), 1);
        let (cid, block) = &blocks[0];
        assert_eq!(cid.codec(), 0x70, "raw leaf with metadata becomes dag-pb");
        let parsed = crate::pb::FlatUnixFs::try_parse(block).unwrap();
        assert_eq!(parsed.data.Type, crate::pb::UnixFsType::File);
        assert_eq!(parsed.data.mode, Some(0o100644));
        assert_eq!(parsed.data.Data.as_deref(), Some(content));
    }

    #[test]
    fn cidv1_raw_leaves_multiblock_roundtrip() {
        use crate::walk::{ContinuedWalk, Walker};
        use ipld_core::cid::Version;
        use std::collections::HashMap;

        let content: &[u8] = b"foobar\n";
        let blocks = FileAdder::builder()
            .with_chunker(Chunker::Size(2))
            .with_cid_version(Version::V1)
            .build()
            .collect_blocks(content, 0);

        let root_cid = blocks.last().unwrap().0;
        assert_eq!(root_cid.version(), Version::V1);
        assert_eq!(root_cid.codec(), 0x70);

        let store: HashMap<Cid, Vec<u8>> = blocks.into_iter().collect();
        assert!(store.keys().filter(|c| c.codec() == 0x55).count() >= 2);

        let mut walker = Walker::new(root_cid, String::new());
        let mut cache = None;
        let mut reassembled = Vec::new();

        while walker.should_continue() {
            let (next, _) = walker.pending_links();
            let block = store.get(next).expect("block present in store");
            match walker.next(block, &mut cache).unwrap() {
                ContinuedWalk::File(segment, ..) => {
                    reassembled.extend_from_slice(segment.as_ref());
                }
                x => unreachable!("{x:?}"),
            }
        }

        assert_eq!(reassembled, content);
    }

    #[test]
    fn balanced_layout_goldens() {
        // captured from the pre-rewrite balanced collector: (leaf_count, block_count, root_cid)
        let expected: &[(usize, usize, &str)] = &[
            (1, 1, "QmS9JArPwa55ePgDnyg6TzX24mYTS1b1vLqWNebyVotKxQ"),
            (2, 3, "QmTpBU2C6VXLUqBpR8ggWW8phBNR9Ca3rz5o7gD6xyPyG7"),
            (173, 174, "QmYHm62RFqvrEHc7ZqeJsdExdXWs4WvV4B8PKaYFfzp3ja"),
            (174, 175, "QmUSeZmRgMby21R1ooTqUddcMrme1SE5JUapFPBKY3do18"),
            (175, 178, "Qma9U731USCLRRP5Scd9m2Y6mJ9Zf6oEYPEr5mu7L96ts8"),
            (176, 179, "Qmdhoohod5MVTgRKFsvHFjPqbb78GEzgScqg1VZXJg9GfT"),
            (348, 351, "QmTfgy9uX5oyKuGm9QsU7SpmhP1HegJUaTNZwseg5VWECp"),
            (349, 353, "QmW3rbpL6Yxa1UREkN8bFvrm8b5LiyHNUWzu5w3Rm5q8T4"),
            (
                30276,
                30451,
                "QmcF5gzgZqJrvjrZbCSTXgFUS9qtq3yrQCXj5ds7QMbAj3",
            ),
            (
                30277,
                30455,
                "Qmf3YNMKeAwk6UAMdMGQFDcJmxyrESQReerPJPT9cRbEcq",
            ),
        ];

        for &(n, blocks_len, root) in expected {
            let content: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            let blocks = FileAdder::builder()
                .with_cid_version(Version::V0)
                .with_chunker(Chunker::Size(1))
                .build()
                .collect_blocks(&content, 0);
            assert_eq!(blocks.len(), blocks_len, "block count for {n} leaves");
            assert_eq!(
                blocks.last().unwrap().0.to_string(),
                root,
                "root cid for {n} leaves"
            );
        }
    }

    #[test]
    fn favourite_single_block_file() {
        let blocks = FakeBlockstore::with_fixtures();
        // everyones favourite content
        let content = b"foobar\n";

        let mut adder = FileAdder::builder().with_cid_version(Version::V0).build();

        {
            let (mut ready_blocks, bytes) = adder.push(content);
            assert!(ready_blocks.next().is_none());
            assert_eq!(bytes, content.len());
        }

        // real impl would probably hash this ... except maybe hashing is faster when done inline?
        // or maybe not
        let (_, file_block) = adder
            .finish()
            .next()
            .expect("there must have been the root block");

        assert_eq!(
            blocks.get_by_str("QmRgutAxd8t7oGkSm4wmeuByG6M51wcTso6cubDdQtuEfL"),
            file_block.as_slice()
        );
    }

    #[test]
    fn favourite_multi_block_file() {
        // root should be QmRJHYTNvC3hmd9gJQARxLR1QMEincccBV53bBw524yyq6

        let blocks = FakeBlockstore::with_fixtures();
        let content = b"foobar\n";
        let adder = FileAdder::builder()
            .with_cid_version(Version::V0)
            .with_chunker(Chunker::Size(2))
            .build();

        let blocks_received = adder.collect_blocks(content, 0);

        // the order here is "fo", "ob", "ar", "\n", root block
        // while verifying the root Cid would be *enough* this is easier to eyeball, ... not really
        // that much but ...
        let expected = [
            "QmfVyMoStzTvdnUR7Uotzh82gmL427q9z3xW5Y8fUoszi4",
            "QmdPyW4CWE3QBkgjWfjM5f7Tjb3HukxVuBXZtkqAGwsMnm",
            "QmNhDQpphvMWhdCzP74taRzXDaEfPGq8vWfFRzD7mEgePM",
            "Qmc5m94Gu7z62RC8waSKkZUrCCBJPyHbkpmGzEePxy2oXJ",
            "QmRJHYTNvC3hmd9gJQARxLR1QMEincccBV53bBw524yyq6",
        ]
        .iter()
        .map(|key| {
            let cid = Cid::try_from(*key).unwrap();
            let block = blocks.get_by_str(key).to_vec();
            (cid, block)
        })
        .collect::<Vec<_>>();

        assert_eq!(blocks_received, expected);
    }

    #[test]
    fn three_layers() {
        let content = b"Lorem ipsum dolor sit amet, sit enim montes aliquam. Cras non lorem, \
            rhoncus condimentum, irure et ante. Pulvinar suscipit odio ante, et tellus a enim, \
            wisi ipsum, vel rhoncus eget faucibus varius, luctus turpis nibh vel odio nulla pede.";

        assert!(content.len() > 174 && content.len() < 2 * 174);

        // go-ipfs 0.5 result: QmRQ6NZNUs4JrCT2y7tmCC1wUhjqYuTssB8VXbbN3rMffg, 239 blocks and root
        // root has two links:
        //  - QmXUcuLGKc8SCMEqG4wgct6NKsSRZQfvB2FCfjDow1PfpB (174 links)
        //  - QmeEn8dxWTzGAFKvyXoLj4oWbh9putL4vSw4uhLXJrSZhs (63 links)
        //
        // in future, if we ever add inline Cid generation this test would need to be changed not
        // to use those inline cids or raw leaves
        let adder = FileAdder::builder()
            .with_cid_version(Version::V0)
            .with_chunker(Chunker::Size(1))
            .build();

        let blocks_received = adder.collect_blocks(content, 0);

        assert_eq!(blocks_received.len(), 240);

        assert_eq!(
            blocks_received.last().unwrap().0.to_string(),
            "QmRQ6NZNUs4JrCT2y7tmCC1wUhjqYuTssB8VXbbN3rMffg"
        );
    }

    #[test]
    fn three_layers_all_subchunks() {
        let content = b"Lorem ipsum dolor sit amet, sit enim montes aliquam. Cras non lorem, \
            rhoncus condimentum, irure et ante. Pulvinar suscipit odio ante, et tellus a enim, \
            wisi ipsum, vel rhoncus eget faucibus varius, luctus turpis nibh vel odio nulla pede.";

        for amt in 1..32 {
            let adder = FileAdder::builder()
                .with_cid_version(Version::V0)
                .with_chunker(Chunker::Size(32))
                .build();
            let blocks_received = adder.collect_blocks(content, amt);
            assert_eq!(
                blocks_received.last().unwrap().0.to_string(),
                "QmYSLcVQqxKygiq7x9w1XGYxU29EShB8ZemiaQ8GAAw17h",
                "amt: {amt}"
            );
        }
    }

    #[test]
    fn empty_file() {
        let blocks = FileAdder::builder()
            .with_cid_version(Version::V0)
            .build()
            .collect_blocks(b"", 0);
        assert_eq!(blocks.len(), 1);
        // 0a == field dag-pb body (unixfs)
        // 04 == dag-pb body len, varint, 4 bytes
        // 08 == field type tag, varint, 1 byte
        // 02 == field type (File)
        // 18 == field filesize tag, varint
        // 00 == filesize, varint, 1 byte
        assert_eq!(blocks[0].1.as_slice(), &hex!("0a 04 08 02 18 00"));
        assert_eq!(
            blocks[0].0.to_string(),
            "QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH"
        );
    }

    #[test]
    fn full_link_block_and_a_byte() {
        let buf = vec![0u8; 2];

        // this should produce a root with two links
        //             +----------^---+
        //             |              |
        //  |----------------------| |-| <-- link blocks
        //   ^^^^^^^^^^^^^^^^^^^^^^   ^
        //          174 blocks        \--- 1 block

        let branching_factor = 174;

        let mut adder = FileAdder::builder()
            .with_cid_version(Version::V0)
            .with_chunker(Chunker::Size(2))
            .with_collector(BalancedCollector::with_branching_factor(branching_factor))
            .build();
        let mut blocks_count = 0;

        for _ in 0..branching_factor {
            let (blocks, written) = adder.push(buf.as_slice());
            assert_eq!(written, buf.len());

            blocks_count += blocks.count();
        }

        let (blocks, written) = adder.push(&buf[0..1]);
        assert_eq!(written, 1);
        blocks_count += blocks.count();

        let last_blocks = adder.finish().collect::<Vec<_>>();
        blocks_count += last_blocks.len();

        // chunks == 174
        // one link block for 174
        // one is for the single byte block
        // one is a link block for the singular single byte block
        // other is for the root block
        assert_eq!(blocks_count, branching_factor + 1 + 1 + 1 + 1);

        assert_eq!(
            last_blocks.last().unwrap().0.to_string(),
            "QmcHNWF1d56uCDSfJPA7t9fadZRV9we5HGSTGSmwuqmMP9"
        );
    }

    #[test]
    fn full_link_block() {
        let buf = vec![0u8; 1];

        let branching_factor = 174;

        let mut adder = FileAdder::builder()
            .with_cid_version(Version::V0)
            .with_chunker(Chunker::Size(1))
            .with_collector(BalancedCollector::with_branching_factor(branching_factor))
            .build();
        let mut blocks_count = 0;

        for _ in 0..branching_factor {
            let (blocks, written) = adder.push(buf.as_slice());
            assert_eq!(written, buf.len());

            blocks_count += blocks.count();
        }

        let mut last_blocks = adder.finish();

        // go-ipfs waits until finish to get a single link block, no additional root block

        let last_block = last_blocks.next().expect("must not have flushed yet");
        blocks_count += 1;

        assert_eq!(last_blocks.next(), None);

        assert_eq!(
            last_block.0.to_string(),
            "QmdgQac8c6Bo3MP5bHAg2yQ25KebFUsmkZFvyByYzf8UCB"
        );

        assert_eq!(blocks_count, 175);
    }
}
