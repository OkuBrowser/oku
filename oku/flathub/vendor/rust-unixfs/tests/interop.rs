use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use ipld_core::cid::{Cid, Version};
use rust_unixfs::dir::builder::{BufferingTreeBuilder, TreeOptions};
use rust_unixfs::file::adder::{Chunker, FileAdder};
use rust_unixfs::symlink::symlink_block;
use rust_unixfs::Metadata;

const MODE: u32 = 0o644;
const MTIME: i64 = 1_700_000_000;

fn ipfs_available() -> bool {
    Command::new("ipfs")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn kubo_add_bytes(content: &[u8], args: &[String]) -> String {
    let mut child = Command::new("ipfs")
        .args(["add", "--only-hash", "-Q"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ipfs add");

    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(content)
        .expect("write content to ipfs");

    let out = child.wait_with_output().expect("wait for ipfs add");
    assert!(
        out.status.success(),
        "ipfs add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf8 cid")
        .trim()
        .to_string()
}

fn kubo_add_path(path: &Path, args: &[String]) -> String {
    let out = Command::new("ipfs")
        .args(["add", "--only-hash", "-Q", "-r"])
        .args(args)
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run ipfs add -r");
    assert!(
        out.status.success(),
        "ipfs add -r failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf8 cid")
        .trim()
        .to_string()
}

fn scratch_dir(tag: &str) -> std::path::PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "rust-unixfs-interop-{}-{}-{tag}",
        std::process::id(),
        n
    ));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn build_file(
    content: &[u8],
    chunk: Option<usize>,
    version: Version,
    metadata: Option<Metadata>,
) -> Vec<(Cid, Vec<u8>)> {
    let mut builder = FileAdder::builder().with_cid_version(version);
    if let Some(size) = chunk {
        builder = builder.with_chunker(Chunker::Size(size));
    }
    if let Some(md) = metadata {
        builder = builder.with_metadata(md);
    }
    let mut adder = builder.build();

    let mut blocks = Vec::new();
    let mut pushed = 0;
    while pushed < content.len() {
        let (produced, consumed) = adder.push(&content[pushed..]);
        blocks.extend(produced);
        pushed += consumed;
    }
    blocks.extend(adder.finish());
    blocks
}

/// Root CID of a file plus its cumulative dag-pb size (the `Tsize` a parent link records, equal to
/// the sum of every block's serialized length).
fn file_root(
    content: &[u8],
    chunk: Option<usize>,
    version: Version,
    metadata: Option<Metadata>,
) -> (Cid, u64) {
    let blocks = build_file(content, chunk, version, metadata);
    let total_size = blocks.iter().map(|(_, b)| b.len() as u64).sum();
    (blocks.last().expect("at least one block").0, total_size)
}

/// Child link CIDs referenced by a dag-pb node (PBLink.Hash, field 1 of each Links entry).
fn child_cids(block: &[u8]) -> Vec<Cid> {
    fn varint(b: &[u8], i: &mut usize) -> u64 {
        let (mut v, mut shift) = (0u64, 0);
        loop {
            let byte = b[*i];
            *i += 1;
            v |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                return v;
            }
            shift += 7;
        }
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < block.len() {
        let field = varint(block, &mut i) >> 3;
        let len = varint(block, &mut i) as usize;
        let chunk = &block[i..i + len];
        i += len;
        if field == 2 {
            let mut j = 0;
            while j < chunk.len() {
                let ltag = varint(chunk, &mut j);
                let llen = varint(chunk, &mut j) as usize;
                if ltag >> 3 == 1 {
                    out.push(Cid::try_from(&chunk[j..j + llen]).unwrap());
                }
                j += llen;
            }
        }
    }
    out
}

/// The directory root is the one node referenced by no other node. The iterator emits a sharded
/// root before its interior shard blocks, so the root is not always the last node.
fn dir_root(builder: BufferingTreeBuilder) -> Cid {
    let nodes: Vec<_> = builder
        .build()
        .map(|r| r.expect("dir construction"))
        .collect();
    let children: HashSet<Cid> = nodes.iter().flat_map(|n| child_cids(&n.block)).collect();
    nodes
        .into_iter()
        .map(|n| n.cid)
        .find(|cid| !children.contains(cid))
        .expect("a root node referenced by nobody")
}

fn small_dir(version: Version) -> Cid {
    let mut opts = TreeOptions::default();
    opts.cid_version(version);
    opts.wrap_with_directory();
    let mut builder = BufferingTreeBuilder::new(opts);
    for (name, content) in [("a.txt", &b"a"[..]), ("b.txt", &b"bb"[..])] {
        let (cid, total_size) = file_root(content, None, version, None);
        builder.put_link(name, cid, total_size).unwrap();
    }
    dir_root(builder)
}

const HAMT_FILES: usize = 6000;

fn hamt_dir(version: Version) -> Cid {
    let mut opts = TreeOptions::default();
    opts.cid_version(version);
    opts.wrap_with_directory();
    let mut builder = BufferingTreeBuilder::new(opts);
    let (cid, total_size) = file_root(b"", None, version, None);
    for i in 0..HAMT_FILES {
        builder
            .put_link(&format!("file-{i:05}"), cid, total_size)
            .unwrap();
    }
    dir_root(builder)
}

/// A sharded subdir ("sub") next to a normal file, inside the wrapped root. Exercises the
/// iterator's deferred-emission path for a shard that is NOT the root.
fn nested_subdir_dir(version: Version) -> Cid {
    let mut opts = TreeOptions::default();
    opts.cid_version(version);
    opts.wrap_with_directory();
    let mut builder = BufferingTreeBuilder::new(opts);
    let (readme, rt) = file_root(b"hello", None, version, None);
    builder.put_link("readme.txt", readme, rt).unwrap();
    let (empty, et) = file_root(b"", None, version, None);
    for i in 0..HAMT_FILES {
        builder
            .put_link(&format!("sub/file-{i:05}"), empty, et)
            .unwrap();
    }
    dir_root(builder)
}

/// Two sibling sharded subdirs under the wrapped root.
fn sibling_shards_dir(version: Version) -> Cid {
    let mut opts = TreeOptions::default();
    opts.cid_version(version);
    opts.wrap_with_directory();
    let mut builder = BufferingTreeBuilder::new(opts);
    let (empty, et) = file_root(b"", None, version, None);
    for i in 0..HAMT_FILES {
        builder
            .put_link(&format!("alpha/a-{i:05}"), empty, et)
            .unwrap();
        builder
            .put_link(&format!("beta/b-{i:05}"), empty, et)
            .unwrap();
    }
    dir_root(builder)
}

fn version_args(version: Version) -> Vec<String> {
    match version {
        Version::V0 => Vec::new(),
        Version::V1 => vec!["--cid-version=1".to_string()],
    }
}

fn chunk_args(chunk: Option<usize>) -> Vec<String> {
    chunk
        .map(|c| vec![format!("--chunker=size-{c}")])
        .unwrap_or_default()
}

// Captured from kubo 0.40.1 `ipfs add --only-hash`.

const EMPTY_V0: &str = "QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH";
const EMPTY_V1: &str = "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku";
const HELLO_V0: &str = "QmT78zSuBmuS4z925WZfrqQ1qHaJ56DQaTfyMUF7F8ff5o";
const HELLO_V1: &str = "bafkreifjjcie6lypi6ny7amxnfftagclbuxndqonfipmb64f2km2devei4";
const FOOBAR_C2_V0: &str = "QmRJHYTNvC3hmd9gJQARxLR1QMEincccBV53bBw524yyq6";
const FOOBAR_C2_V1: &str = "bafybeiakabo5d5e25jzw2i32mymsnwyuoaqozkjfkqibnvodyxna6nuanm";
const ZEROS1000_C7_V0: &str = "QmNgDVWgrTpUEmxAeutigwykfU2bKjjEGFzvknzptoqAUN";
const ZEROS1000_C7_V1: &str = "bafybeia3u4lcsbc6ijwivdukvlougvli4rm73r65chivjsl5l7vemqclhy";
const A400_C1_V0: &str = "QmccZ7Tar96JCFFSXcBQwjP847Zt2ezVWmDb5rFj7wPL1U";
const A400_C1_V1: &str = "bafybeiajemq5ovepy7fc56ifhtn6aoctbkgxcqav44zg2bjkrfwvlbnewu";

const FOOBAR_META_V0: &str = "QmSKJ5FTGeacMnX1M69wak9wtDXE1gipL5pNnfnZu36vKb";
const FOOBAR_META_V1: &str = "bafybeib3cxkzkogkexj4bnyx5qrbui7dulj6cudejwffwqh2fk6agbjcsi";

const SYMLINK_V0: &str = "QmNgQEdXVdLw79nH2bnxLMxnyWMaXrijfqMTiDVat3iyuz";
const SYMLINK_V1: &str = "bafybeiafb5ac2gbschcrzuqj3ewdam5vbqs7xtzbpgow5mkab7s5ilzxre";

const SMALL_DIR_V0: &str = "QmYeCfYxiAzFV6TAgzuLDouQqGeJqG4WxGA1KYS8HncSU9";
const SMALL_DIR_V1: &str = "bafybeic7au5c3eydqdymxffpwtuahdr3saj2dxptw34y6uvkyoybxpuhte";

const HAMT_DIR_V0: &str = "QmXNw274pqF5fjJkgZBJV5Hob8dzH8SUPiEMdTAeMb7492";
const HAMT_DIR_V1: &str = "bafybeie43ouwdxahhv64kcn47jmknnquqpv4jo3jaejmv3uqgomkey4giu";

const NESTED_DIR_V0: &str = "QmVfqsn13Lwu2ZUvcfsxenwBSxDNo1h3RTvDdbJGqM537J";
const NESTED_DIR_V1: &str = "bafybeidoputpooro7qarpdimkkaagrtmy2qynkqinn5hakjms6lnb32hqi";
const SIBLING_DIR_V0: &str = "QmNmFcLAFyJ9qBc2dGGDPLTf9ozkaVfJU3pH9yXjxEGSKk";
const SIBLING_DIR_V1: &str = "bafybeie76nplpl5g2rvf5in3rntjqki5aoctciwr6k2k53hg6suydd6cza";

/// `(content, chunk, version, expected_cid)` for plain file leaves and balanced layouts.
fn file_cases() -> Vec<(Vec<u8>, Option<usize>, Version, &'static str)> {
    vec![
        (b"".to_vec(), None, Version::V0, EMPTY_V0),
        (b"".to_vec(), None, Version::V1, EMPTY_V1),
        (b"hello world\n".to_vec(), None, Version::V0, HELLO_V0),
        (b"hello world\n".to_vec(), None, Version::V1, HELLO_V1),
        (b"foobar\n".to_vec(), Some(2), Version::V0, FOOBAR_C2_V0),
        (b"foobar\n".to_vec(), Some(2), Version::V1, FOOBAR_C2_V1),
        (vec![0u8; 1000], Some(7), Version::V0, ZEROS1000_C7_V0),
        (vec![0u8; 1000], Some(7), Version::V1, ZEROS1000_C7_V1),
        (vec![b'a'; 400], Some(1), Version::V0, A400_C1_V0),
        (vec![b'a'; 400], Some(1), Version::V1, A400_C1_V1),
    ]
}

#[test]
fn pinned_files() {
    for (content, chunk, version, expected) in file_cases() {
        let (cid, _) = file_root(&content, chunk, version, None);
        assert_eq!(
            cid.to_string(),
            expected,
            "file len={} chunk={chunk:?} {version:?}",
            content.len()
        );
    }
}

#[test]
fn pinned_metadata_file() {
    let md = Metadata::new(Some(MODE), Some((MTIME, 0)));
    let (v0, _) = file_root(b"foobar\n", None, Version::V0, Some(md.clone()));
    assert_eq!(v0.to_string(), FOOBAR_META_V0);
    assert_eq!(v0.codec(), 0x70);

    let (v1, _) = file_root(b"foobar\n", None, Version::V1, Some(md));
    assert_eq!(
        v1.to_string(),
        FOOBAR_META_V1,
        "metadata disables raw leaves"
    );
    assert_eq!(v1.codec(), 0x70);
}

#[test]
fn pinned_symlink() {
    let md = Metadata::default();
    assert_eq!(
        symlink_block(
            "foobar",
            &md,
            Version::V0,
            multihash_codetable::Code::Sha2_256
        )
        .0
        .to_string(),
        SYMLINK_V0
    );
    assert_eq!(
        symlink_block(
            "foobar",
            &md,
            Version::V1,
            multihash_codetable::Code::Sha2_256
        )
        .0
        .to_string(),
        SYMLINK_V1
    );
}

#[test]
fn pinned_small_dir() {
    assert_eq!(small_dir(Version::V0).to_string(), SMALL_DIR_V0);
    assert_eq!(small_dir(Version::V1).to_string(), SMALL_DIR_V1);
}

#[test]
fn pinned_hamt_dir() {
    assert_eq!(hamt_dir(Version::V0).to_string(), HAMT_DIR_V0);
    assert_eq!(hamt_dir(Version::V1).to_string(), HAMT_DIR_V1);
}

#[test]
fn pinned_nested_dirs() {
    assert_eq!(nested_subdir_dir(Version::V0).to_string(), NESTED_DIR_V0);
    assert_eq!(nested_subdir_dir(Version::V1).to_string(), NESTED_DIR_V1);
    assert_eq!(sibling_shards_dir(Version::V0).to_string(), SIBLING_DIR_V0);
    assert_eq!(sibling_shards_dir(Version::V1).to_string(), SIBLING_DIR_V1);
}

#[ignore]
#[test]
fn live_files() {
    if !ipfs_available() {
        eprintln!("skipping live_files: no `ipfs` on PATH");
        return;
    }
    for (content, chunk, version, pinned) in file_cases() {
        let mut args = version_args(version);
        args.extend(chunk_args(chunk));
        let kubo = kubo_add_bytes(&content, &args);
        let (cid, _) = file_root(&content, chunk, version, None);
        assert_eq!(cid.to_string(), kubo, "rust vs kubo, len={}", content.len());
        assert_eq!(kubo, pinned, "pinned vector drifted from kubo");
    }
}

#[test]
fn live_metadata_file() {
    if !ipfs_available() {
        eprintln!("skipping live_metadata_file: no `ipfs` on PATH");
        return;
    }
    let meta_args = [format!("--mode=0{MODE:o}"), format!("--mtime={MTIME}")];
    for (version, pinned) in [(Version::V0, FOOBAR_META_V0), (Version::V1, FOOBAR_META_V1)] {
        let mut args = version_args(version);
        args.extend_from_slice(&meta_args);
        let kubo = kubo_add_bytes(b"foobar\n", &args);
        let md = Metadata::new(Some(MODE), Some((MTIME, 0)));
        let (cid, _) = file_root(b"foobar\n", None, version, Some(md));
        assert_eq!(cid.to_string(), kubo, "rust vs kubo metadata {version:?}");
        assert_eq!(kubo, pinned, "pinned vector drifted from kubo");
    }
}

#[ignore]
#[cfg(unix)]
#[test]
fn live_symlink() {
    if !ipfs_available() {
        eprintln!("skipping live_symlink: no `ipfs` on PATH");
        return;
    }
    let dir = scratch_dir("symlink");
    let link = dir.join("mylink");
    std::os::unix::fs::symlink("foobar", &link).expect("create symlink");

    for (version, pinned) in [(Version::V0, SYMLINK_V0), (Version::V1, SYMLINK_V1)] {
        let kubo = kubo_add_path(&link, &version_args(version));
        let cid = symlink_block(
            "foobar",
            &Metadata::default(),
            version,
            multihash_codetable::Code::Sha2_256,
        )
        .0;
        assert_eq!(cid.to_string(), kubo, "rust vs kubo symlink {version:?}");
        assert_eq!(kubo, pinned, "pinned vector drifted from kubo");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[ignore]
#[test]
fn live_small_dir() {
    if !ipfs_available() {
        eprintln!("skipping live_small_dir: no `ipfs` on PATH");
        return;
    }
    let dir = scratch_dir("small");
    std::fs::write(dir.join("a.txt"), b"a").unwrap();
    std::fs::write(dir.join("b.txt"), b"bb").unwrap();

    for (version, pinned) in [(Version::V0, SMALL_DIR_V0), (Version::V1, SMALL_DIR_V1)] {
        let kubo = kubo_add_path(&dir, &version_args(version));
        assert_eq!(
            small_dir(version).to_string(),
            kubo,
            "rust vs kubo dir {version:?}"
        );
        assert_eq!(kubo, pinned, "pinned vector drifted from kubo");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[ignore]
#[test]
fn live_hamt_dir() {
    if !ipfs_available() {
        eprintln!("skipping live_hamt_dir: no `ipfs` on PATH");
        return;
    }
    let dir = scratch_dir("hamt");
    for i in 0..HAMT_FILES {
        std::fs::write(dir.join(format!("file-{i:05}")), b"").unwrap();
    }

    for (version, pinned) in [(Version::V0, HAMT_DIR_V0), (Version::V1, HAMT_DIR_V1)] {
        let kubo = kubo_add_path(&dir, &version_args(version));
        assert_eq!(
            hamt_dir(version).to_string(),
            kubo,
            "rust vs kubo HAMT {version:?}"
        );
        assert_eq!(kubo, pinned, "pinned vector drifted from kubo");
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[ignore]
#[test]
fn live_nested_dirs() {
    if !ipfs_available() {
        eprintln!("skipping live_nested_dirs: no `ipfs` on PATH");
        return;
    }

    let dir = scratch_dir("nested");
    std::fs::write(dir.join("readme.txt"), b"hello").unwrap();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    for i in 0..HAMT_FILES {
        std::fs::write(dir.join("sub").join(format!("file-{i:05}")), b"").unwrap();
    }
    for (version, pinned) in [(Version::V0, NESTED_DIR_V0), (Version::V1, NESTED_DIR_V1)] {
        let kubo = kubo_add_path(&dir, &version_args(version));
        assert_eq!(
            nested_subdir_dir(version).to_string(),
            kubo,
            "rust vs kubo nested {version:?}"
        );
        assert_eq!(kubo, pinned, "pinned vector drifted from kubo");
    }
    std::fs::remove_dir_all(&dir).ok();

    let dir = scratch_dir("siblings");
    std::fs::create_dir_all(dir.join("alpha")).unwrap();
    std::fs::create_dir_all(dir.join("beta")).unwrap();
    for i in 0..HAMT_FILES {
        std::fs::write(dir.join("alpha").join(format!("a-{i:05}")), b"").unwrap();
        std::fs::write(dir.join("beta").join(format!("b-{i:05}")), b"").unwrap();
    }
    for (version, pinned) in [(Version::V0, SIBLING_DIR_V0), (Version::V1, SIBLING_DIR_V1)] {
        let kubo = kubo_add_path(&dir, &version_args(version));
        assert_eq!(
            sibling_shards_dir(version).to_string(),
            kubo,
            "rust vs kubo siblings {version:?}"
        );
        assert_eq!(kubo, pinned, "pinned vector drifted from kubo");
    }
    std::fs::remove_dir_all(&dir).ok();
}
