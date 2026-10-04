use bytes::Bytes;
use ipld_core::cid::{self, Cid, Version};
use multihash_codetable::{Code, MultihashDigest};
use unsigned_varint::{decode as varint_decode, encode as varint_encode};

/// Prefix represents all metadata of a CID, without the actual content.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Prefix {
    /// The version of CID.
    pub version: Version,
    /// The codec of CID.
    pub codec: u64,
    /// The multihash type of CID.
    pub mh_type: u64,
    /// The multihash length of CID.
    pub mh_len: usize,
}

impl Prefix {
    pub fn new(data: &[u8]) -> Result<Prefix, cid::Error> {
        let (raw_version, remain) = varint_decode::u64(data).map_err(Into::<cid::Error>::into)?;
        let version = Version::try_from(raw_version)?;
        let (codec, remain) = varint_decode::u64(remain).map_err(Into::<cid::Error>::into)?;
        let (mh_type, remain) = varint_decode::u64(remain).map_err(Into::<cid::Error>::into)?;
        let (mh_len, _remain) = varint_decode::usize(remain).map_err(Into::<cid::Error>::into)?;

        Ok(Prefix {
            version,
            codec,
            mh_type,
            mh_len,
        })
    }

    pub fn to_bytes(&self) -> Bytes {
        let mut res = Vec::with_capacity(4);

        let mut buf = varint_encode::u64_buffer();
        let version = varint_encode::u64(self.version.into(), &mut buf);
        res.extend_from_slice(version);
        let mut buf = varint_encode::u64_buffer();
        let codec = varint_encode::u64(self.codec, &mut buf);
        res.extend_from_slice(codec);
        let mut buf = varint_encode::u64_buffer();
        let mh_type = varint_encode::u64(self.mh_type, &mut buf);
        res.extend_from_slice(mh_type);
        let mut buf = varint_encode::u64_buffer();
        let mh_len = varint_encode::u64(self.mh_len as u64, &mut buf);
        res.extend_from_slice(mh_len);

        res.into()
    }

    pub fn to_cid(&self, data: &[u8]) -> Result<Cid, cid::Error> {
        let mh = Code::try_from(self.mh_type)
            .map_err(std::io::Error::other)?
            .digest(data);
        Cid::new(self.version, self.codec, mh)
    }
}

impl From<Cid> for Prefix {
    fn from(cid: Cid) -> Self {
        Self {
            version: cid.version(),
            codec: cid.codec(),
            mh_type: cid.hash().code(),
            mh_len: cid.hash().digest().len(),
        }
    }
}
