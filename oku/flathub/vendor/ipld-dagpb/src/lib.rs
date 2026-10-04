//! Protobuf codec.
#![deny(missing_docs)]

use std::io::{BufRead, Write};

mod codec;
mod error;

use bytes::Bytes;
use ipld_core::{
    cid::Cid,
    codec::{Codec, Links},
    ipld::Ipld,
};

use crate::codec::PbNodeRef;
pub use crate::{
    codec::{PbLink, PbNode},
    error::Error,
};

/// The multicodec code for DAG-PB.
pub const DAG_PB_CODE: u64 = 0x70;

/// Convert from [`ipld_core::ipld::Ipld`] into serialized DAG-PB.
pub fn from_ipld(ipld: &Ipld) -> Result<Vec<u8>, Error> {
    let node: PbNodeRef = ipld.try_into()?;
    Ok(node.into_bytes())
}

/// Convert from serialized DAG-PB to [`ipld_core::ipld::Ipld`].
pub fn to_ipld(bytes: &[u8]) -> Result<Ipld, Error> {
    let node = PbNode::from_bytes(Bytes::copy_from_slice(bytes))?;
    Ok(node.into())
}

/// Extract all the links from a serialize DAG-PB object.
pub fn links(bytes: &[u8], links: &mut impl Extend<Cid>) -> Result<(), Error> {
    let node = PbNode::from_bytes(Bytes::copy_from_slice(bytes))?;
    for link in node.links {
        links.extend(Some(link.cid));
    }
    Ok(())
}

/// DAG-PB implementation of ipld-core's `Codec` trait.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DagPbCodec;

impl Codec<Ipld> for DagPbCodec {
    const CODE: u64 = DAG_PB_CODE;
    type Error = Error;

    fn decode<R: BufRead>(mut reader: R) -> Result<Ipld, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        crate::to_ipld(&bytes)
    }

    fn encode<W: Write>(mut writer: W, data: &Ipld) -> Result<(), Self::Error> {
        let bytes = crate::from_ipld(data)?;
        Ok(writer.write_all(&bytes)?)
    }
}

impl Links for DagPbCodec {
    type LinksError = Error;

    fn links(data: &[u8]) -> Result<impl Iterator<Item = Cid>, Self::LinksError> {
        let mut links = Vec::new();
        crate::links(data, &mut links)?;
        Ok(links.into_iter())
    }
}

impl Codec<PbNode> for DagPbCodec {
    const CODE: u64 = DAG_PB_CODE;
    type Error = Error;

    fn decode<R: BufRead>(mut reader: R) -> Result<PbNode, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        PbNode::from_bytes(Bytes::copy_from_slice(&bytes))
    }

    fn encode<W: Write>(mut writer: W, data: &PbNode) -> Result<(), Self::Error> {
        let bytes = data.clone().into_bytes();
        Ok(writer.write_all(&bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    #[test]
    fn test_encode_decode() {
        let cid =
            Cid::try_from("bafkreie74tgmnxqwojhtumgh5dzfj46gi4mynlfr7dmm7duwzyvnpw7h7m").unwrap();
        let mut pb_link = BTreeMap::<String, Ipld>::new();
        pb_link.insert("Hash".to_string(), cid.into());
        pb_link.insert("Name".to_string(), "block".to_string().into());
        pb_link.insert("Tsize".to_string(), 13.into());

        let links: Vec<Ipld> = vec![pb_link.into()];
        let mut pb_node = BTreeMap::<String, Ipld>::new();
        pb_node.insert("Data".to_string(), b"Here is some data\n".to_vec().into());
        pb_node.insert("Links".to_string(), links.into());
        let data: Ipld = pb_node.into();

        let bytes = from_ipld(&data).unwrap();
        let data2 = to_ipld(&bytes).unwrap();
        assert_eq!(data, data2);
    }
}
