//! Helpers for blobs that contain a sequence of hashes.
use std::fmt::Debug;

use bytes::Bytes;
use n0_error::{anyerr, AnyError};

use crate::Hash;

/// A sequence of links, backed by a [`Bytes`] object.
#[derive(Clone, derive_more::Into)]
pub struct HashSeq(Bytes);

impl Debug for HashSeq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a> FromIterator<&'a Hash> for HashSeq {
    fn from_iter<T: IntoIterator<Item = &'a Hash>>(iter: T) -> Self {
        iter.into_iter().copied().collect()
    }
}

impl FromIterator<Hash> for HashSeq {
    fn from_iter<T: IntoIterator<Item = Hash>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let (lower, _upper) = iter.size_hint();
        let mut bytes = Vec::with_capacity(lower * 32);
        for hash in iter {
            bytes.extend_from_slice(hash.as_ref());
        }
        Self(bytes.into())
    }
}

impl TryFrom<Bytes> for HashSeq {
    type Error = AnyError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        Self::new(bytes).ok_or_else(|| anyerr!("invalid hash sequence"))
    }
}

impl IntoIterator for HashSeq {
    type Item = Hash;
    type IntoIter = HashSeqIter;

    fn into_iter(self) -> Self::IntoIter {
        HashSeqIter(self)
    }
}

impl HashSeq {
    /// Create a new sequence of hashes.
    pub fn new(bytes: Bytes) -> Option<Self> {
        if bytes.len().is_multiple_of(32) {
            Some(Self(bytes))
        } else {
            None
        }
    }

    /// Iterate over the hashes in this sequence.
    pub fn iter(&self) -> impl Iterator<Item = Hash> + '_ {
        self.0.chunks_exact(32).map(|chunk| {
            let hash: [u8; 32] = chunk.try_into().unwrap();
            hash.into()
        })
    }

    /// Get the number of hashes in this sequence.
    pub fn len(&self) -> usize {
        self.0.len() / 32
    }

    /// Check if this sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the hash at the given index.
    pub fn get(&self, index: usize) -> Option<Hash> {
        if index < self.len() {
            let hash: [u8; 32] = self.0[index * 32..(index + 1) * 32].try_into().unwrap();
            Some(hash.into())
        } else {
            None
        }
    }

    /// Get and remove the first hash in this sequence.
    pub fn pop_front(&mut self) -> Option<Hash> {
        if self.is_empty() {
            None
        } else {
            let hash = self.get(0).unwrap();
            self.0 = self.0.slice(32..);
            Some(hash)
        }
    }

    /// Get the underlying bytes.
    pub fn into_inner(self) -> Bytes {
        self.0
    }
}

/// Iterator over the hashes in a [`HashSeq`].
#[derive(Debug, Clone)]
pub struct HashSeqIter(HashSeq);

impl Iterator for HashSeqIter {
    type Item = Hash;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}
