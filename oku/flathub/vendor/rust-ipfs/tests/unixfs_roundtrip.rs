use rust_ipfs::Node;
use rust_ipfs::unixfs::ll::file::adder::Chunker;

/// Deterministic, non-repeating payload so a wrong-order reassembly is detectable.
fn payload(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u32).wrapping_mul(2_654_435_761).rotate_left(13) as u8)
        .collect()
}

/// A tiny chunker turns a modest payload into a many-leaf, multi-level DAG, which forces the
/// prefetch window and reorder buffer to handle blocks completing out of consumption order.
async fn add_multiblock(node: &Node, data: &[u8]) -> rust_ipfs::IpfsPath {
    node.add_unixfs(data.to_vec())
        .chunk(Chunker::Size(64))
        .await
        .expect("add succeeds")
}

#[tokio::test]
async fn cat_roundtrip_multiblock() {
    let node = Node::new("cat_roundtrip_multiblock").await;
    let data = payload(40_000);

    let path = add_multiblock(&node, &data).await;
    let out = node.cat_unixfs(path).await.expect("cat succeeds");

    assert_eq!(out.len(), data.len());
    assert_eq!(out.as_ref(), data.as_slice());

    node.shutdown().await;
}

#[tokio::test]
async fn get_roundtrip_multiblock() {
    let node = Node::new("get_roundtrip_multiblock").await;
    let data = payload(40_000);

    let path = add_multiblock(&node, &data).await;

    let dest =
        std::env::temp_dir().join(format!("rust_ipfs_unixfs_get_{}.bin", std::process::id()));
    let _ = std::fs::remove_file(&dest);

    node.get_unixfs(path, &dest).await.expect("get succeeds");

    let written = std::fs::read(&dest).expect("read back");
    let _ = std::fs::remove_file(&dest);

    assert_eq!(written, data);

    node.shutdown().await;
}

#[tokio::test]
async fn cat_roundtrip_single_block() {
    let node = Node::new("cat_roundtrip_single_block").await;
    let data = payload(40);

    let path = add_multiblock(&node, &data).await;
    let out = node.cat_unixfs(path).await.expect("cat succeeds");

    assert_eq!(out.as_ref(), data.as_slice());

    node.shutdown().await;
}

#[tokio::test]
async fn ls_roundtrip_multiblock() {
    let node = Node::new("ls_roundtrip_multiblock").await;

    let path = add_multiblock(&node, &payload(40_000)).await;

    let entries = node.unixfs().ls(path).await.expect("ls walk succeeds");

    assert!(
        entries
            .iter()
            .any(|e| matches!(e, rust_ipfs::unixfs::Entry::File { .. })),
        "{entries:?}"
    );

    node.shutdown().await;
}

#[tokio::test]
async fn ls_lists_only_current_directory() {
    use rust_ipfs::Block;
    use rust_ipfs::unixfs::ll::dir::builder::{BufferingTreeBuilder, TreeOptions};

    let node = Node::new("ls_lists_only_current_directory").await;

    let top = payload(40_000);
    let nested = payload(2_000);
    let nested_len = nested.len() as u64;
    let top_cid = *add_multiblock(&node, &top)
        .await
        .root()
        .cid()
        .expect("ipld root");
    let nested_cid = *node
        .add_unixfs(nested)
        .await
        .expect("add nested")
        .root()
        .cid()
        .expect("ipld root");

    // Build root/ { a.txt, sub/ { inner.txt } } and store the directory nodes; the file blocks are
    // already in the repo from the adds above.
    let mut opts = TreeOptions::default();
    opts.wrap_with_directory();
    let mut builder = BufferingTreeBuilder::new(opts);
    builder
        .put_link("a.txt", top_cid, top.len() as u64)
        .expect("link a");
    builder
        .put_link("sub/inner.txt", nested_cid, nested_len)
        .expect("link inner");

    let mut root = None;
    for tree_node in builder.build() {
        let tree_node = tree_node.expect("tree node");
        let block = Block::new(tree_node.cid, tree_node.block.to_vec()).expect("valid block");
        node.repo().put_block(&block).await.expect("put dir block");
        root = Some(tree_node.cid);
    }
    let root = root.expect("at least one directory node");

    let entries = node.unixfs().ls(root).await.expect("ls walk succeeds");

    let files: Vec<&str> = entries
        .iter()
        .filter_map(|e| match e {
            rust_ipfs::unixfs::Entry::File { file, .. } => Some(file.as_str()),
            _ => None,
        })
        .collect();
    let dirs: Vec<&str> = entries
        .iter()
        .filter_map(|e| match e {
            rust_ipfs::unixfs::Entry::Directory { path, .. } => Some(path.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        files.iter().any(|f| f.ends_with("a.txt")),
        "files: {files:?}"
    );
    assert!(dirs.iter().any(|d| d.ends_with("sub")), "dirs: {dirs:?}");
    // The nested file lives under sub/ so a shallow ls of the root must not surface it.
    assert!(
        !files.iter().any(|f| f.ends_with("inner.txt")),
        "inner.txt leaked into a shallow ls: {files:?}"
    );

    node.shutdown().await;
}

#[tokio::test]
async fn add_default_is_cidv1() {
    use ipld_core::cid::Version;

    let node = Node::new("add_default_is_cidv1").await;

    // A small file's default root is now a raw-leaf CIDv1, not a CIDv0 dag-pb node.
    let path = node.add_unixfs(payload(40)).await.expect("add small");
    let cid = path.root().cid().copied().expect("ipld root");
    assert_eq!(cid.version(), Version::V1);
    assert_eq!(cid.codec(), 0x55, "small file root is a raw leaf");

    // A multi-block file's default root is a CIDv1 dag-pb node over raw leaves.
    let path = node
        .add_unixfs(payload(40_000))
        .chunk(Chunker::Size(64))
        .await
        .expect("add multiblock");
    let cid = path.root().cid().copied().expect("ipld root");
    assert_eq!(cid.version(), Version::V1);
    assert_eq!(cid.codec(), 0x70, "multi-block root is dag-pb");

    node.shutdown().await;
}

#[tokio::test]
async fn add_with_blake3_hasher_roundtrips() {
    use multihash_codetable::Code;

    let node = Node::new("add_with_blake3_hasher").await;
    let data = payload(40_000);

    let path = node
        .add_unixfs(data.clone())
        .chunk(Chunker::Size(64))
        .hasher(Code::Blake3_256)
        .await
        .expect("add succeeds");

    let blake3: u64 = Code::Blake3_256.into();
    let cid = path.root().cid().copied().expect("ipld root");
    assert_eq!(cid.hash().code(), blake3, "root must use blake3");

    // The prefetching cat path is hash-agnostic and must still round-trip the bytes.
    let out = node.cat_unixfs(path).await.expect("cat succeeds");
    assert_eq!(out.as_ref(), data.as_slice());

    node.shutdown().await;
}

#[tokio::test]
async fn cat_roundtrip_default_chunk() {
    let node = Node::new("cat_roundtrip_default_chunk").await;
    // Larger than one default 256 KiB leaf so the standard chunker also spans multiple blocks.
    let data = payload(700_000);

    let path = node.add_unixfs(data.clone()).await.expect("add succeeds");
    let out = node.cat_unixfs(path).await.expect("cat succeeds");

    assert_eq!(out.as_ref(), data.as_slice());

    node.shutdown().await;
}
