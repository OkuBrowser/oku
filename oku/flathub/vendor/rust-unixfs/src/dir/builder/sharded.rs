use super::{CustomFlatUnixFs, Leaf, NamedLeaf, TreeConstructionFailed};
use crate::pb::{UnixFs, UnixFsType};
use crate::Metadata;
use alloc::borrow::Cow;
use ipld_core::cid::{Cid, Version};
use multihash_codetable::Code;

const FANOUT: usize = 256;
const HASH_TYPE_MURMUR3: u64 = 0x22;
const MAX_DEPTH: usize = 8;

pub(super) struct ShardBlock {
    pub(super) cid: Cid,
    pub(super) block: Vec<u8>,
    pub(super) total_size: u64,
}

fn hamt_hash(name: &[u8]) -> [u8; 8] {
    let h = murmur3::murmur3_x64_128(&mut std::io::Cursor::new(name), 0)
        .expect("hashing an in-memory cursor cannot fail");
    (h as u64).to_be_bytes()
}

struct Entry<'a> {
    hash: [u8; 8],
    leaf: &'a NamedLeaf,
}

pub(super) fn build_sharded(
    links: &[Option<NamedLeaf>],
    buffer: &mut Vec<u8>,
    cid_version: Version,
    hasher: Code,
    metadata: &Metadata,
) -> Result<(Leaf, Vec<ShardBlock>), TreeConstructionFailed> {
    let entries = links
        .iter()
        .filter_map(|opt| opt.as_ref())
        .map(|leaf| Entry {
            hash: hamt_hash(leaf.0.as_bytes()),
            leaf,
        })
        .collect::<Vec<_>>();

    let mut interior = Vec::new();
    let root = build_shard(&entries, 0, cid_version, hasher, &mut interior, metadata)?;

    buffer.clear();
    buffer.extend_from_slice(&root.block);

    Ok((
        Leaf {
            link: root.cid,
            total_size: root.total_size,
        },
        interior,
    ))
}

fn build_shard(
    entries: &[Entry<'_>],
    depth: usize,
    cid_version: Version,
    hasher: Code,
    interior: &mut Vec<ShardBlock>,
    metadata: &Metadata,
) -> Result<ShardBlock, TreeConstructionFailed> {
    if depth >= MAX_DEPTH {
        return Err(TreeConstructionFailed::ShardTooDeep(entries.len()));
    }

    let mut buckets: Vec<Vec<&Entry<'_>>> = (0..FANOUT).map(|_| Vec::new()).collect();
    for entry in entries {
        buckets[entry.hash[depth] as usize].push(entry);
    }

    let mut shard_links = Vec::new();
    let mut bitfield = [0u8; FANOUT / 8];

    for (index, bucket) in buckets.iter().enumerate() {
        if bucket.is_empty() {
            continue;
        }

        let bit = (index % 8) as u32;
        bitfield[FANOUT / 8 - 1 - index / 8] |= 1u8 << bit;

        let link = if bucket.len() == 1 {
            let leaf = bucket[0].leaf;
            NamedLeaf(format!("{index:02X}{}", leaf.0), leaf.1, leaf.2)
        } else {
            let child_entries = bucket
                .iter()
                .map(|e| Entry {
                    hash: e.hash,
                    leaf: e.leaf,
                })
                .collect::<Vec<_>>();
            let child = build_shard(
                &child_entries,
                depth + 1,
                cid_version,
                hasher,
                interior,
                &Metadata::default(),
            )?;
            let link = NamedLeaf(format!("{index:02X}"), child.cid, child.total_size);
            interior.push(child);
            link
        };

        shard_links.push(Some(link));
    }

    shard_links.sort_by(|a, b| {
        let a = a.as_ref().expect("just pushed Some");
        let b = b.as_ref().expect("just pushed Some");
        a.0.as_bytes().cmp(b.0.as_bytes())
    });

    let (mode, mtime) = metadata.to_pb();
    let start = bitfield
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bitfield.len() - 1);
    let data = UnixFs {
        Type: UnixFsType::HAMTShard,
        Data: Some(Cow::Owned(bitfield[start..].to_vec())),
        fanout: Some(FANOUT as u64),
        hashType: Some(HASH_TYPE_MURMUR3),
        mode,
        mtime,
        ..Default::default()
    };

    let (cid, block) = render_shard_node(&shard_links, data, cid_version, hasher)?;

    let links_total: u64 = shard_links
        .iter()
        .map(|l| l.as_ref().expect("just pushed Some").2)
        .sum();

    Ok(ShardBlock {
        cid,
        total_size: block.len() as u64 + links_total,
        block,
    })
}

fn render_shard_node(
    links: &[Option<NamedLeaf>],
    data: UnixFs<'_>,
    cid_version: Version,
    hasher: Code,
) -> Result<(Cid, Vec<u8>), TreeConstructionFailed> {
    use quick_protobuf::{BytesWriter, MessageWrite, Writer};

    let node = CustomFlatUnixFs { links, data };
    let size = node.get_size();

    let mut block = vec![0u8; size];
    let mut writer = Writer::new(BytesWriter::new(&mut block[..]));
    node.write_message(&mut writer)
        .map_err(TreeConstructionFailed::Protobuf)?;

    let cid = crate::pb::make_cid(cid_version, hasher, crate::file::DAG_PB_CODEC, &block);

    Ok((cid, block))
}

#[cfg(test)]
mod tests {
    use super::super::NamedLeaf;
    use super::{build_sharded, hamt_hash};
    use crate::dir::MaybeResolved;
    use crate::pb::{FlatUnixFs, UnixFsType};
    use ipld_core::cid::{Cid, Version};
    use multihash::Multihash;
    use multihash_codetable::Code;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;

    fn cid_of(n: u8) -> Cid {
        let mh = Multihash::wrap(Code::Sha2_256.into(), &Sha256::digest([n])).unwrap();
        Cid::new_v0(mh).unwrap()
    }

    fn link(name: &str, n: u8) -> Option<NamedLeaf> {
        Some(NamedLeaf(name.to_string(), cid_of(n), 1))
    }

    fn names(parsed: &FlatUnixFs<'_>) -> Vec<String> {
        parsed
            .links
            .iter()
            .map(|l| l.Name.as_deref().unwrap().to_string())
            .collect()
    }

    #[test]
    fn murmur3_hash_half_and_endianness() {
        let h = |s: &str| u64::from_be_bytes(hamt_hash(s.as_bytes()));
        assert_eq!(h("a"), 0x85555565f6597889);
        assert_eq!(h("b"), 0x7a98a957b1d3d1ee);
        assert_eq!(h("file.txt"), 0xb53875c83534fe3e);
        assert_eq!(h("0"), 0x2ac9debed546a380);
        assert_eq!(h("dir"), 0x2a7c964720718e4f);
    }

    #[test]
    fn shard_without_collision() {
        let links = [link("a", 1), link("b", 2), link("file.txt", 3)];
        let mut buffer = Vec::new();
        let (_leaf, interior) = build_sharded(
            &links,
            &mut buffer,
            Version::V0,
            Code::Sha2_256,
            &crate::Metadata::default(),
        )
        .unwrap();
        assert!(interior.is_empty());

        let parsed = FlatUnixFs::try_parse(&buffer).unwrap();
        assert_eq!(parsed.data.Type, UnixFsType::HAMTShard);
        assert_eq!(parsed.data.fanout, Some(256));
        assert_eq!(parsed.data.hashType, Some(0x22));
        assert!(parsed.data.filesize.is_none());
        assert!(parsed.data.blocksizes.is_empty());

        // sorted by Name bytes: 0x7A 'b', 0x85 'a', 0xB5 'file.txt'
        assert_eq!(names(&parsed), ["7Ab", "85a", "B5file.txt"]);

        let bf = parsed.data.Data.as_deref().unwrap();
        // trimmed big-endian bitfield: highest occupied bucket is 0xB5, so leading zero bytes drop
        assert_eq!(bf.len(), 23);
        assert_eq!(bf.iter().map(|b| b.count_ones()).sum::<u32>(), 3);
        for idx in [0x7Au8, 0x85, 0xB5] {
            let i = idx as usize;
            assert_ne!(
                bf[bf.len() - 1 - i / 8] & (1 << (i % 8)),
                0,
                "bit for {idx:#x}"
            );
        }
    }

    #[test]
    fn shard_with_collision_creates_subshard() {
        // both 0 and dir hash to byte0 0x2A; they split at depth 1 (0xC9 vs 0x7C)
        let links = [link("0", 1), link("dir", 2)];
        let mut buffer = Vec::new();
        let (_leaf, interior) = build_sharded(
            &links,
            &mut buffer,
            Version::V0,
            Code::Sha2_256,
            &crate::Metadata::default(),
        )
        .unwrap();
        assert_eq!(interior.len(), 1);

        let root = FlatUnixFs::try_parse(&buffer).unwrap();
        assert_eq!(names(&root), ["2A"]);
        assert_eq!(root.links[0].Tsize, Some(interior[0].total_size));

        let sub = FlatUnixFs::try_parse(&interior[0].block).unwrap();
        assert_eq!(sub.data.Type, UnixFsType::HAMTShard);
        assert_eq!(names(&sub), ["7Cdir", "C90"]);
    }

    #[test]
    fn shard_resolves_via_read_side() {
        let entries: [(&str, u8); 3] = [("0", 1), ("dir", 2), ("file.txt", 3)];
        let links = entries.map(|(n, c)| link(n, c));
        let mut buffer = Vec::new();
        let (leaf, interior) = build_sharded(
            &links,
            &mut buffer,
            Version::V0,
            Code::Sha2_256,
            &crate::Metadata::default(),
        )
        .unwrap();

        let mut blocks = HashMap::new();
        blocks.insert(leaf.link, buffer);
        for s in &interior {
            blocks.insert(s.cid, s.block.clone());
        }

        for (name, n) in entries {
            assert_eq!(resolve(&blocks, leaf.link, name), Some(cid_of(n)), "{name}");
        }
        assert_eq!(resolve(&blocks, leaf.link, "missing"), None);
    }

    fn resolve(blocks: &HashMap<Cid, Vec<u8>>, root: Cid, name: &str) -> Option<Cid> {
        let mut cache = None;
        let block = blocks.get(&root).unwrap();
        let mut state = crate::dir::resolve(block, name, &mut cache).unwrap();
        loop {
            match state {
                MaybeResolved::Found(cid) => return Some(cid),
                MaybeResolved::NotFound => return None,
                MaybeResolved::NeedToLoadMore(lookup) => {
                    let next = *lookup.pending_links().0;
                    let block = blocks.get(&next).expect("bucket block present");
                    state = lookup.continue_walk(block, &mut cache).unwrap();
                }
            }
        }
    }
}
