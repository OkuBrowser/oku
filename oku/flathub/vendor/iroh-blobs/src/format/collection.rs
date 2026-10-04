//! The collection type used by iroh
use std::{collections::BTreeMap, future::Future};

// n0_error::Context is no longer exported; use explicit mapping instead.
use bao_tree::blake3;
use bytes::Bytes;
use n0_error::{Result, StdResultExt};
use serde::{Deserialize, Serialize};

use crate::{
    api::{blobs::AddBytesOptions, Store},
    get::{fsm, Stats},
    hashseq::HashSeq,
    util::temp_tag::TempTag,
    BlobFormat, Hash,
};

/// A collection of blobs
///
/// Note that the format is subject to change.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct Collection {
    /// Links to the blobs in this collection
    blobs: Vec<(String, Hash)>,
}

impl std::ops::Index<usize> for Collection {
    type Output = (String, Hash);

    fn index(&self, index: usize) -> &Self::Output {
        &self.blobs[index]
    }
}

impl<K, V> Extend<(K, V)> for Collection
where
    K: Into<String>,
    V: Into<Hash>,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.blobs
            .extend(iter.into_iter().map(|(k, v)| (k.into(), v.into())));
    }
}

impl<K, V> FromIterator<(K, V)> for Collection
where
    K: Into<String>,
    V: Into<Hash>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut res = Self::default();
        res.extend(iter);
        res
    }
}

impl IntoIterator for Collection {
    type Item = (String, Hash);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.blobs.into_iter()
    }
}

/// A simple store trait for loading blobs
pub trait SimpleStore {
    /// Load a blob from the store
    fn load(&self, hash: Hash) -> impl Future<Output = Result<Bytes>> + Send + '_;
}

impl SimpleStore for crate::api::Store {
    async fn load(&self, hash: Hash) -> Result<Bytes> {
        Ok(self.get_bytes(hash).await?)
    }
}

/// Metadata for a collection
///
/// This is the wire format for the metadata blob.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CollectionMeta {
    header: [u8; 13], // Must contain "CollectionV0."
    names: Vec<String>,
}

impl Default for CollectionMeta {
    fn default() -> Self {
        Self {
            header: *Collection::HEADER,
            names: Vec::default(),
        }
    }
}

impl CollectionMeta {
    /// Verifies the header is the only valid collection header.
    pub fn check_header(&self) -> bool {
        &self.header == Collection::HEADER
    }

    /// Returns the names in this collection.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Returns a mutable reference to the names in this collection.
    pub fn names_mut(&mut self) -> &mut Vec<String> {
        &mut self.names
    }
}

impl Collection {
    /// The header for the collection format.
    ///
    /// This is the start of the metadata blob.
    pub const HEADER: &'static [u8; 13] = b"CollectionV0.";

    /// Convert the collection to an iterator of blobs, with the last being the
    /// root blob.
    ///
    /// To persist the collection, write all the blobs to storage, and use the
    /// hash of the last blob as the collection hash.
    pub fn to_blobs(&self) -> impl DoubleEndedIterator<Item = Bytes> {
        let meta = CollectionMeta {
            header: *Self::HEADER,
            names: self.names(),
        };
        let meta_bytes = postcard::to_stdvec(&meta).unwrap();
        let meta_bytes_hash = blake3::hash(&meta_bytes).into();
        let links = std::iter::once(meta_bytes_hash)
            .chain(self.links())
            .collect::<HashSeq>();
        let links_bytes = links.into_inner();
        [meta_bytes.into(), links_bytes].into_iter()
    }

    /// Read the collection from a get fsm.
    ///
    /// Returns the fsm at the start of the first child blob (if any),
    /// the links array, and the collection.
    pub async fn read_fsm(
        fsm_at_start_root: fsm::AtStartRoot,
    ) -> Result<(fsm::EndBlobNext, HashSeq, Collection)> {
        let (next, links) = {
            let curr = fsm_at_start_root.next();
            let (curr, data) = curr.concatenate_into_vec().await?;
            let links = HashSeq::new(data.into())
                .ok_or_else(|| n0_error::anyerr!("links could not be parsed"))?;
            (curr.next(), links)
        };
        let fsm::EndBlobNext::MoreChildren(at_meta) = next else {
            n0_error::bail_any!("expected meta");
        };
        let (next, collection) = {
            let mut children = links.clone();
            let meta_link = children
                .pop_front()
                .ok_or_else(|| n0_error::anyerr!("meta link not found"))?;
            let curr = at_meta.next(meta_link);
            let (curr, names) = curr.concatenate_into_vec().await?;
            let names = postcard::from_bytes::<CollectionMeta>(&names).anyerr()?;
            n0_error::ensure_any!(
                names.check_header(),
                "expected header {:?}, got {:?}",
                Self::HEADER,
                names.header
            );
            let collection = Collection::from_parts(children, names);
            (curr.next(), collection)
        };
        Ok((next, links, collection))
    }

    /// Read the collection and all it's children from a get fsm.
    ///
    /// Returns the collection, a map from blob offsets to bytes, and the stats.
    pub async fn read_fsm_all(
        fsm_at_start_root: crate::get::fsm::AtStartRoot,
    ) -> Result<(Collection, BTreeMap<u64, Bytes>, Stats)> {
        let (next, links, collection) = Self::read_fsm(fsm_at_start_root).await?;
        let mut res = BTreeMap::new();
        let mut curr = next;
        let end = loop {
            match curr {
                fsm::EndBlobNext::MoreChildren(more) => {
                    let child_offset = more.offset() - 1;
                    let Some(hash) = links.get(usize::try_from(child_offset).anyerr()?) else {
                        break more.finish();
                    };
                    let header = more.next(hash);
                    let (next, blob) = header.concatenate_into_vec().await?;
                    res.insert(child_offset - 1, blob.into());
                    curr = next.next();
                }
                fsm::EndBlobNext::Closing(closing) => break closing,
            }
        };
        let stats = end.next().await?;
        Ok((collection, res, stats))
    }

    /// Create a new collection from a hash sequence and metadata.
    pub async fn load(root: Hash, store: &impl SimpleStore) -> Result<Self> {
        let hs = store.load(root).await?;
        let hs = HashSeq::try_from(hs)?;
        let meta_hash = hs
            .iter()
            .next()
            .ok_or_else(|| n0_error::anyerr!("empty hash seq"))?;
        let meta = store.load(meta_hash).await?;
        let meta: CollectionMeta = postcard::from_bytes(&meta).anyerr()?;
        n0_error::ensure_any!(
            meta.names.len() + 1 == hs.len(),
            "names and links length mismatch"
        );
        Ok(Self::from_parts(hs.into_iter().skip(1), meta))
    }

    /// Store a collection in a store. returns the root hash of the collection
    /// as a TempTag.
    pub async fn store(self, db: &Store) -> Result<TempTag> {
        let (links, meta) = self.into_parts();
        let meta_bytes = postcard::to_stdvec(&meta).anyerr()?;
        let meta_tag = db.add_bytes(meta_bytes).temp_tag().await?;
        let links_bytes = std::iter::once(meta_tag.hash())
            .chain(links)
            .collect::<HashSeq>();
        let links_tag = db
            .add_bytes_with_opts(AddBytesOptions {
                data: links_bytes.into(),
                format: BlobFormat::HashSeq,
            })
            .temp_tag()
            .await?;
        Ok(links_tag)
    }

    /// Split a collection into a sequence of links and metadata
    fn into_parts(self) -> (Vec<Hash>, CollectionMeta) {
        let mut names = Vec::with_capacity(self.blobs.len());
        let mut links = Vec::with_capacity(self.blobs.len());
        for (name, hash) in self.blobs {
            names.push(name);
            links.push(hash);
        }
        let meta = CollectionMeta {
            header: *Self::HEADER,
            names,
        };
        (links, meta)
    }

    /// Create a new collection from a list of hashes and metadata
    fn from_parts(links: impl IntoIterator<Item = Hash>, meta: CollectionMeta) -> Self {
        meta.names.into_iter().zip(links).collect()
    }

    /// Get the links to the blobs in this collection
    fn links(&self) -> impl Iterator<Item = Hash> + '_ {
        self.blobs.iter().map(|(_name, hash)| *hash)
    }

    /// Get the names of the blobs in this collection
    fn names(&self) -> Vec<String> {
        self.blobs.iter().map(|(name, _)| name.clone()).collect()
    }

    /// Iterate over the blobs in this collection
    pub fn iter(&self) -> impl Iterator<Item = &(String, Hash)> {
        self.blobs.iter()
    }

    /// Get the number of blobs in this collection
    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    /// Check if this collection is empty
    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    /// Add the given blob to the collection.
    pub fn push(&mut self, name: String, hash: Hash) {
        self.blobs.push((name, hash));
    }
}

#[cfg(test)]
mod tests {
    use n0_error::{Result, StackResultExt};

    use super::*;

    #[test]
    fn roundtrip_blob() {
        let b = (
            "test".to_string(),
            blake3::Hash::from_hex(
                "3aa61c409fd7717c9d9c639202af2fae470c0ef669be7ba2caea5779cb534e9d",
            )
            .unwrap()
            .into(),
        );

        let mut buf = bytes::BytesMut::zeroed(1024);
        postcard::to_slice(&b, &mut buf).unwrap();
        let deserialize_b: (String, Hash) = postcard::from_bytes(&buf).unwrap();
        assert_eq!(b, deserialize_b);
    }

    #[test]
    fn roundtrip_collection_meta() {
        let expected = CollectionMeta {
            header: *Collection::HEADER,
            names: vec!["test".to_string(), "a".to_string(), "b".to_string()],
        };
        let mut buf = bytes::BytesMut::zeroed(1024);
        postcard::to_slice(&expected, &mut buf).unwrap();
        let actual: CollectionMeta = postcard::from_bytes(&buf).unwrap();
        assert_eq!(expected, actual);
    }

    #[tokio::test]
    async fn collection_store_load() -> testresult::TestResult {
        let collection = (0..3)
            .map(|i| {
                (
                    format!("blob{i}"),
                    crate::Hash::from(blake3::hash(&[i as u8])),
                )
            })
            .collect::<Collection>();
        let mut root = None;
        let store = collection
            .to_blobs()
            .map(|data| {
                let hash = crate::Hash::from(blake3::hash(&data));
                root = Some(hash);
                (hash, data)
            })
            .collect::<TestStore>();
        let collection2 = Collection::load(root.unwrap(), &store).await?;
        assert_eq!(collection, collection2);
        Ok(())
    }

    /// An implementation of a [SimpleStore] for testing
    struct TestStore(BTreeMap<Hash, Bytes>);

    impl FromIterator<(Hash, Bytes)> for TestStore {
        fn from_iter<T: IntoIterator<Item = (Hash, Bytes)>>(iter: T) -> Self {
            Self(iter.into_iter().collect())
        }
    }

    impl SimpleStore for TestStore {
        async fn load(&self, hash: Hash) -> Result<Bytes> {
            self.0.get(&hash).cloned().context("not found")
        }
    }
}
