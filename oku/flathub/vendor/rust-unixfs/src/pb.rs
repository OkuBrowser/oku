use alloc::borrow::Cow;
use core::convert::TryFrom;
use core::fmt;
use core::ops::Range;
use quick_protobuf::{errors::Result as ProtobufResult, Writer, WriterBackend};

pub(crate) mod merkledag;
pub(crate) use merkledag::PBLink;
pub(crate) use merkledag::PBNode;

pub(crate) mod unixfs;
pub(crate) use unixfs::mod_Data::DataType as UnixFsType;
pub(crate) use unixfs::Data as UnixFs;
pub(crate) use unixfs::UnixTime;

/// Failure cases for nested serialization, which allows recovery of the outer `PBNode` when desired.
#[derive(Debug)]
pub(crate) enum ParsingFailed<'a> {
    InvalidDagPb(quick_protobuf::Error),
    NoData(PBNode<'a>),
    InvalidUnixFs(quick_protobuf::Error, PBNode<'a>),
}

impl fmt::Display for ParsingFailed<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParsingFailed::*;
        match self {
            InvalidDagPb(e) => write!(fmt, "failed to read the block as dag-pb: {e}"),
            InvalidUnixFs(e, _) => write!(
                fmt,
                "failed to read the dag-pb PBNode::Data as UnixFS message: {e}"
            ),
            NoData(_) => write!(fmt, "dag-pb PBNode::Data was missing or empty"),
        }
    }
}

impl std::error::Error for ParsingFailed<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use ParsingFailed::*;

        match self {
            InvalidDagPb(e) => Some(e),
            InvalidUnixFs(e, _) => Some(e),
            NoData(_) => None,
        }
    }
}

// This has been aliased as UnixFs<'a>
impl<'a> TryFrom<&'a merkledag::PBNode<'a>> for unixfs::Data<'a> {
    type Error = quick_protobuf::Error;

    fn try_from(node: &'a merkledag::PBNode<'a>) -> Result<Self, Self::Error> {
        UnixFs::try_from(node.Data.as_deref())
    }
}

// This has been aliased as UnixFs<'a>
impl<'a> TryFrom<Option<&'a [u8]>> for unixfs::Data<'a> {
    type Error = quick_protobuf::Error;

    fn try_from(data: Option<&'a [u8]>) -> Result<Self, Self::Error> {
        use quick_protobuf::{BytesReader, MessageRead};

        let data = data.unwrap_or_default();
        let mut reader = BytesReader::from_bytes(data);
        UnixFs::from_reader(&mut reader, data)
    }
}

// These should be derived by the pb-rs
impl<'a> TryFrom<&'a [u8]> for merkledag::PBNode<'a> {
    type Error = quick_protobuf::Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        use quick_protobuf::{BytesReader, MessageRead};
        merkledag::PBNode::from_reader(&mut BytesReader::from_bytes(data), data)
    }
}

/// Combined dag-pb (or MerkleDAG) with UnixFs payload.
#[derive(Debug)]
pub(crate) struct FlatUnixFs<'a> {
    pub(crate) links: Vec<PBLink<'a>>,
    pub(crate) data: UnixFs<'a>,
}

impl quick_protobuf::message::MessageWrite for FlatUnixFs<'_> {
    fn get_size(&self) -> usize {
        use quick_protobuf::sizeofs::sizeof_len;
        let links = self
            .links
            .iter()
            .map(|s| 1 + sizeof_len(s.get_size()))
            .sum::<usize>();

        let body = 1 + sizeof_len(self.data.get_size());

        links + body
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> ProtobufResult<()> {
        // this has been monkeyd after PBNode::write_message
        //
        // important to note that while protobuf isn't so picky when reading on field order, dag-pb
        // is, at least to produce the same Cids.
        for link in &self.links {
            w.write_with_tag(18, |w| w.write_message(link))?;
        }
        // writing the self.data directly saves us the trouble of serializing it first to a vec,
        // then using the vec to write this field.
        w.write_with_tag(10, |w| w.write_message(&self.data))?;
        Ok(())
    }
}

/// Hashes `bytes` with `hasher` and wraps the digest in a `Cid` of the given `codec`. CIDv0 is
/// defined only for dag-pb with sha2-256, so any other hash yields a CIDv1 regardless of
/// `cid_version`.
pub(crate) fn make_cid(
    cid_version: ipld_core::cid::Version,
    hasher: multihash_codetable::Code,
    codec: u64,
    bytes: &[u8],
) -> ipld_core::cid::Cid {
    use ipld_core::cid::{Cid, Version};
    use multihash_codetable::{Code, MultihashDigest};

    let mh = hasher.digest(bytes);
    match cid_version {
        Version::V0 if hasher == Code::Sha2_256 => {
            Cid::new_v0(mh).expect("sha2-256 is the correct multihash for cidv0")
        }
        _ => Cid::new_v1(codec, mh),
    }
}

/// Serializes a `Cid` as the bytes of a dag-pb `PBLink::Hash`, streaming the multihash directly
/// (`code | size | digest`) so no intermediate byte vector is allocated.
pub(crate) struct WriteableCid<'a>(pub(crate) &'a ipld_core::cid::Cid);

impl quick_protobuf::message::MessageWrite for WriteableCid<'_> {
    fn get_size(&self) -> usize {
        use ipld_core::cid::Version::*;
        use quick_protobuf::sizeofs::sizeof_varint;

        let mh = self.0.hash();
        let hash_len =
            sizeof_varint(mh.code()) + sizeof_varint(mh.size() as u64) + mh.size() as usize;

        match self.0.version() {
            V0 => hash_len,
            V1 => 1 + sizeof_varint(self.0.codec()) + hash_len,
        }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> ProtobufResult<()> {
        use ipld_core::cid::Version::*;

        if let V1 = self.0.version() {
            w.write_u8(1)?;
            w.write_varint(self.0.codec())?;
        }

        let mh = self.0.hash();
        w.write_varint(mh.code())?;
        w.write_varint(mh.size() as u64)?;
        mh.digest()[..mh.size() as usize]
            .iter()
            .try_for_each(|b| w.write_u8(*b))
    }
}

impl<'a> FlatUnixFs<'a> {
    pub(crate) fn try_parse(data: &'a [u8]) -> Result<Self, ParsingFailed<'a>> {
        Self::try_from(data)
    }
}

impl<'a> TryFrom<&'a [u8]> for FlatUnixFs<'a> {
    type Error = ParsingFailed<'a>;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let node = merkledag::PBNode::try_from(data).map_err(ParsingFailed::InvalidDagPb)?;

        let data = match node.Data {
            Some(Cow::Borrowed(bytes)) if !bytes.is_empty() => Some(bytes),
            Some(Cow::Owned(_)) => unreachable!(),
            Some(Cow::Borrowed(_)) | None => return Err(ParsingFailed::NoData(node)),
        };

        match UnixFs::try_from(data) {
            Ok(data) => Ok(FlatUnixFs {
                links: node.Links,
                data,
            }),
            Err(e) => Err(ParsingFailed::InvalidUnixFs(e, node)),
        }
    }
}

#[cfg(test)]
impl<'a> FlatUnixFs<'a> {
    pub fn range_links(&'a self) -> impl Iterator<Item = (PBLink<'a>, Range<u64>)> {
        assert_eq!(self.links.len(), self.data.blocksizes.len());

        let zipped = self
            .links
            .clone()
            .into_iter()
            .zip(self.data.blocksizes.iter().copied());

        // important: we have validated links.len() == blocksizes.len()
        RangeLinks::from_links_and_blocksizes(zipped, Some(0))
    }
}

pub(crate) struct RangeLinks<I> {
    inner: I,
    base: u64,
}

impl<'a, I> RangeLinks<I>
where
    I: Iterator<Item = (PBLink<'a>, u64)>,
{
    /// `start_offset` is the offset of the current tree when walking the graph.
    pub fn from_links_and_blocksizes(zipped: I, start_offset: Option<u64>) -> RangeLinks<I> {
        RangeLinks {
            inner: zipped,
            base: start_offset.unwrap_or(0),
        }
    }
}

impl<'a, I> Iterator for RangeLinks<I>
where
    I: Iterator<Item = (PBLink<'a>, u64)>,
{
    type Item = (PBLink<'a>, Range<u64>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(link, blocksize)| {
            let returned_base = self.base;
            self.base += blocksize;
            (link, returned_base..(returned_base + blocksize))
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[cfg(test)]
mod test {
    use super::{FlatUnixFs, PBNode, UnixFs, UnixFsType};
    use alloc::borrow::Cow;
    use core::convert::TryFrom;
    use hex_literal::hex;

    #[test]
    fn parse_content() {
        use quick_protobuf::{BytesReader, MessageRead};
        let input = hex!("0a0d08021207636f6e74656e741807");

        let mut reader = BytesReader::from_bytes(&input);
        let dagnode =
            PBNode::from_reader(&mut reader, &input).expect("parse outer merkledag::PBNode");
        assert!(dagnode.Links.is_empty());

        let unixfs_data = UnixFs::try_from(&dagnode).expect("parse inner unixfs::Data");
        assert_eq!(unixfs_data.Type, UnixFsType::File);
        assert_eq!(unixfs_data.Data, Some(Cow::Borrowed(&b"content"[..])));
        assert_eq!(unixfs_data.filesize, Some(7));
        println!("{unixfs_data:?}");
    }

    #[test]
    fn linux_tarxz_range_links() {
        let input = hex!("122b0a2212203822560f945fd3c74522de3448512a7e45cb53f0a9a1e12161da4667531ec12e120018aed4e015122b0a2212208594eb4dd5d67e573d506cd950ac59863b9afb024a590d7fe49b42fbcb44af43120018aed4e015122b0a221220745a70b6cd7ec3e46d16fb15b5e1e5db256f6a7a52d0b359f8f49b242665e17b120018b4e7e8090a1608021888c1a835208080e015208080e0152088c1e809");

        let flat = FlatUnixFs::try_from(&input[..]).unwrap();

        let mut expected_ranges = vec![
            0..45_613_056,
            45_613_056..91_226_112,
            91_226_112..111_812_744,
        ];

        expected_ranges.reverse();

        for (link, range) in flat.range_links() {
            assert_eq!(link.Name, Some(Cow::Borrowed("")));
            // Tsize is the subtree size, which must always be larger than the file segments
            // because of encoding
            assert!(link.Tsize >= Some(range.end - range.start));
            assert_eq!(Some(range), expected_ranges.pop());
        }
    }
}
