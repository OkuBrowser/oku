use std::{borrow::Borrow, fmt};

use bao_tree::io::mixed::EncodedItem;
use bytes::Bytes;
use derive_more::{From, Into};
use n0_future::time::SystemTime;

mod sparse_mem_file;
use irpc::channel::mpsc;
use range_collections::{range_set::RangeSetEntry, RangeSetRef};
use ref_cast::RefCast;
use serde::{Deserialize, Serialize};
pub use sparse_mem_file::SparseMemFile;
pub mod observer;
mod size_info;
pub use size_info::SizeInfo;
mod partial_mem_storage;
pub use partial_mem_storage::PartialMemStorage;

#[cfg(feature = "fs-store")]
mod mem_or_file;
#[cfg(feature = "fs-store")]
#[cfg_attr(iroh_blobs_docsrs, doc(cfg(feature = "fs-store")))]
pub use mem_or_file::{FixedSize, MemOrFile};

/// A named, persistent tag.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, From, Into)]
pub struct Tag(pub Bytes);

impl From<&[u8]> for Tag {
    fn from(value: &[u8]) -> Self {
        Self(Bytes::copy_from_slice(value))
    }
}

impl AsRef<[u8]> for Tag {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Borrow<[u8]> for Tag {
    fn borrow(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<String> for Tag {
    fn from(value: String) -> Self {
        Self(Bytes::from(value))
    }
}

impl From<&str> for Tag {
    fn from(value: &str) -> Self {
        Self(Bytes::from(value.to_owned()))
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0.as_ref();
        match std::str::from_utf8(bytes) {
            Ok(s) => write!(f, "\"{s}\""),
            Err(_) => write!(f, "{}", hex::encode(bytes)),
        }
    }
}

impl Tag {
    /// Create a new tag that does not exist yet.
    pub fn auto(time: SystemTime, exists: impl Fn(&[u8]) -> bool) -> Self {
        // On wasm, SystemTime is web_time::SystemTime, but we need a std system time
        // to convert to chrono.
        // TODO: Upstream to n0-future or expose SystemTimeExt on wasm
        #[cfg(wasm_browser)]
        let time = std::time::SystemTime::UNIX_EPOCH
            + time.duration_since(SystemTime::UNIX_EPOCH).unwrap();

        let now = chrono::DateTime::<chrono::Utc>::from(time);
        let mut i = 0;
        loop {
            let mut text = format!("auto-{}", now.format("%Y-%m-%dT%H:%M:%S%.3fZ"));
            if i != 0 {
                text.push_str(&format!("-{i}"));
            }
            if !exists(text.as_bytes()) {
                return Self::from(text);
            }
            i += 1;
        }
    }

    /// The successor of this tag in lexicographic order.
    pub fn successor(&self) -> Self {
        let mut bytes = self.0.to_vec();
        // increment_vec(&mut bytes);
        bytes.push(0);
        Self(bytes.into())
    }

    /// If this is a prefix, get the next prefix.
    ///
    /// This is like successor, except that it will return None if the prefix is all 0xFF instead of appending a 0 byte.
    pub fn next_prefix(&self) -> Option<Self> {
        let mut bytes = self.0.to_vec();
        if next_prefix(&mut bytes) {
            Some(Self(bytes.into()))
        } else {
            None
        }
    }
}

pub struct DD<T: fmt::Display>(pub T);

impl<T: fmt::Display> fmt::Debug for DD<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tag").field(&DD(self)).finish()
    }
}

pub(crate) fn limited_range(offset: u64, len: usize, buf_len: usize) -> std::ops::Range<usize> {
    if offset < buf_len as u64 {
        let start = offset as usize;
        let end = start.saturating_add(len).min(buf_len);
        start..end
    } else {
        0..0
    }
}

/// zero copy get a limited slice from a `Bytes` as a `Bytes`.
#[allow(dead_code)]
pub(crate) fn get_limited_slice(bytes: &Bytes, offset: u64, len: usize) -> Bytes {
    bytes.slice(limited_range(offset, len, bytes.len()))
}

pub trait RangeSetExt<T> {
    fn upper_bound(&self) -> Option<T>;
}

impl<T: RangeSetEntry + Clone> RangeSetExt<T> for RangeSetRef<T> {
    /// The upper (exclusive) bound of the bitfield
    fn upper_bound(&self) -> Option<T> {
        let boundaries = self.boundaries();
        if boundaries.is_empty() {
            Some(RangeSetEntry::min_value())
        } else if boundaries.len().is_multiple_of(2) {
            Some(boundaries[boundaries.len() - 1].clone())
        } else {
            None
        }
    }
}

#[cfg(feature = "fs-store")]
mod fs {
    use std::{
        fmt,
        fs::{File, OpenOptions},
        io::{self, Read, Write},
        path::Path,
    };

    use arrayvec::ArrayString;
    use bao_tree::blake3;
    use serde::{de::DeserializeOwned, Serialize};

    mod redb_support {
        use bytes::Bytes;
        use redb::{Key as RedbKey, Value as RedbValue};

        use super::super::Tag;

        impl RedbValue for Tag {
            type SelfType<'a> = Self;

            type AsBytes<'a> = bytes::Bytes;

            fn fixed_width() -> Option<usize> {
                None
            }

            fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
            where
                Self: 'a,
            {
                Self(Bytes::copy_from_slice(data))
            }

            fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
            where
                Self: 'a,
                Self: 'b,
            {
                value.0.clone()
            }

            fn type_name() -> redb::TypeName {
                redb::TypeName::new("Tag")
            }
        }

        impl RedbKey for Tag {
            fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
                data1.cmp(data2)
            }
        }
    }

    pub fn write_checksummed<P: AsRef<Path>, T: Serialize>(path: P, data: &T) -> io::Result<()> {
        // Build Vec with space for hash
        let mut buffer = Vec::with_capacity(32 + 128);
        buffer.extend_from_slice(&[0u8; 32]);

        // Serialize directly into buffer
        postcard::to_io(data, &mut buffer).map_err(io::Error::other)?;

        // Compute hash over data (skip first 32 bytes)
        let data_slice = &buffer[32..];
        let hash = blake3::hash(data_slice);
        buffer[..32].copy_from_slice(hash.as_bytes());

        // Write all at once
        let mut file = File::create(&path)?;
        file.write_all(&buffer)?;
        file.sync_all()?;

        Ok(())
    }

    pub fn read_checksummed_and_truncate<T: DeserializeOwned>(
        path: impl AsRef<Path>,
    ) -> io::Result<T> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        file.set_len(0)?;
        file.sync_all()?;

        if buffer.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File marked dirty",
            ));
        }

        if buffer.len() < 32 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "File too short"));
        }

        let stored_hash = &buffer[..32];
        let data = &buffer[32..];

        let computed_hash = blake3::hash(data);
        if computed_hash.as_bytes() != stored_hash {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Hash mismatch"));
        }

        let deserialized = postcard::from_bytes(data).map_err(io::Error::other)?;

        Ok(deserialized)
    }

    #[cfg(test)]
    pub fn read_checksummed<T: DeserializeOwned>(path: impl AsRef<Path>) -> io::Result<T> {
        use std::{fs::File, io::Read};

        use bao_tree::blake3;
        use tracing::info;

        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        info!("{} {}", path.display(), hex::encode(&buffer));

        if buffer.is_empty() {
            use std::io;

            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "File marked dirty",
            ));
        }

        if buffer.len() < 32 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "File too short"));
        }

        let stored_hash = &buffer[..32];
        let data = &buffer[32..];

        let computed_hash = blake3::hash(data);
        if computed_hash.as_bytes() != stored_hash {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Hash mismatch"));
        }

        let deserialized = postcard::from_bytes(data).map_err(io::Error::other)?;

        Ok(deserialized)
    }

    /// Helper trait for bytes for debugging
    pub trait SliceInfoExt: AsRef<[u8]> {
        // get the addr of the actual data, to check if data was copied
        fn addr(&self) -> usize;

        // a short symbol string for the address
        fn addr_short(&self) -> ArrayString<12> {
            let addr = self.addr().to_le_bytes();
            symbol_string(&addr)
        }

        #[allow(dead_code)]
        fn hash_short(&self) -> ArrayString<10> {
            crate::Hash::new(self.as_ref()).fmt_short()
        }
    }

    impl<T: AsRef<[u8]>> SliceInfoExt for T {
        fn addr(&self) -> usize {
            self.as_ref() as *const [u8] as *const u8 as usize
        }

        fn hash_short(&self) -> ArrayString<10> {
            crate::Hash::new(self.as_ref()).fmt_short()
        }
    }

    pub fn symbol_string(data: &[u8]) -> ArrayString<12> {
        const SYMBOLS: &[char] = &[
            '😀', '😂', '😍', '😎', '😢', '😡', '😱', '😴', '🤓', '🤔', '🤗', '🤢', '🤡', '🤖',
            '👽', '👾', '👻', '💀', '💩', '♥', '💥', '💦', '💨', '💫', '💬', '💭', '💰', '💳',
            '💼', '📈', '📉', '📍', '📢', '📦', '📱', '📷', '📺', '🎃', '🎄', '🎉', '🎋', '🎍',
            '🎒', '🎓', '🎖', '🎤', '🎧', '🎮', '🎰', '🎲', '🎳', '🎴', '🎵', '🎷', '🎸', '🎹',
            '🎺', '🎻', '🎼', '🏀', '🏁', '🏆', '🏈',
        ];
        const BASE: usize = SYMBOLS.len(); // 64

        // Hash the input with BLAKE3
        let hash = blake3::hash(data);
        let bytes = hash.as_bytes(); // 32-byte hash

        // Create an ArrayString with capacity 12 (bytes)
        let mut result = ArrayString::<12>::new();

        // Fill with 3 symbols
        for byte in bytes.iter().take(3) {
            let byte = *byte as usize;
            let index = byte % BASE;
            result.push(SYMBOLS[index]); // Each char can be up to 4 bytes
        }

        result
    }

    pub struct ValueOrPoisioned<T>(pub Option<T>);

    impl<T: fmt::Debug> fmt::Debug for ValueOrPoisioned<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.0 {
                Some(x) => x.fmt(f),
                None => f.debug_tuple("Poisoned").finish(),
            }
        }
    }
}
#[cfg(feature = "fs-store")]
pub use fs::*;

/// Given a prefix, increment it lexographically.
///
/// If the prefix is all FF, this will return false because there is no
/// higher prefix than that.
#[allow(dead_code)]
pub(crate) fn next_prefix(bytes: &mut [u8]) -> bool {
    for byte in bytes.iter_mut().rev() {
        if *byte < 255 {
            *byte += 1;
            return true;
        }
        *byte = 0;
    }
    false
}

#[derive(ref_cast::RefCast)]
#[repr(transparent)]
pub struct BaoTreeSender(mpsc::Sender<EncodedItem>);

impl BaoTreeSender {
    pub fn new(sender: &mut mpsc::Sender<EncodedItem>) -> &mut Self {
        BaoTreeSender::ref_cast_mut(sender)
    }
}

impl bao_tree::io::mixed::Sender for BaoTreeSender {
    type Error = irpc::channel::SendError;
    async fn send(&mut self, item: EncodedItem) -> std::result::Result<(), Self::Error> {
        self.0.send(item).await
    }
}

#[cfg(test)]
#[cfg(feature = "fs-store")]
pub mod tests {
    use bao_tree::{io::outboard::PreOrderMemOutboard, ChunkRanges};
    use n0_error::{Result, StdResultExt};

    use crate::{hash::Hash, store::IROH_BLOCK_SIZE};

    /// Create n0 flavoured bao. Note that this can be used to request ranges below a chunk group size,
    /// which can not be exported via bao because we don't store hashes below the chunk group level.
    pub fn create_n0_bao(data: &[u8], ranges: &ChunkRanges) -> Result<(Hash, Vec<u8>)> {
        let outboard = PreOrderMemOutboard::create(data, IROH_BLOCK_SIZE);
        let mut encoded = Vec::new();
        let size = data.len() as u64;
        encoded.extend_from_slice(&size.to_le_bytes());
        bao_tree::io::sync::encode_ranges_validated(data, &outboard, ranges, &mut encoded)
            .anyerr()?;
        Ok((outboard.root.into(), encoded))
    }
}
