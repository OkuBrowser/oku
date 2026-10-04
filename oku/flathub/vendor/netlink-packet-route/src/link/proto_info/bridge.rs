// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    parse_u8, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    NlasIterator, Parseable,
};

const IFLA_BRPORT_NEIGH_FORWARD_GRAT: u16 = 45;

#[derive(Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum LinkProtoInfoBridge {
    NeighForwardGrat(bool),
    Other(DefaultNla),
}

pub(crate) struct VecLinkProtoInfoBridge(pub(crate) Vec<LinkProtoInfoBridge>);

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for VecLinkProtoInfoBridge
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let mut nlas = vec![];
        for nla in NlasIterator::new(buf.into_inner()) {
            let nla = nla.context(format!(
                "invalid bridge IFLA_PROTINFO {:?}",
                buf.value()
            ))?;
            nlas.push(LinkProtoInfoBridge::parse(&nla)?);
        }
        Ok(Self(nlas))
    }
}

impl Nla for LinkProtoInfoBridge {
    fn value_len(&self) -> usize {
        match *self {
            Self::NeighForwardGrat(_) => 1,
            Self::Other(ref nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match *self {
            Self::NeighForwardGrat(value) => {
                buffer[0] = if value { 1 } else { 0 }
            }
            Self::Other(ref nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match *self {
            Self::NeighForwardGrat(_) => IFLA_BRPORT_NEIGH_FORWARD_GRAT,
            Self::Other(ref nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for LinkProtoInfoBridge
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        Ok(match buf.kind() {
            IFLA_BRPORT_NEIGH_FORWARD_GRAT => Self::NeighForwardGrat(
                parse_u8(buf.value())
                    .context("invalid IFLA_BRPORT_NEIGH_FORWARD_GRAT value")?
                    > 0,
            ),
            _ => Self::Other(DefaultNla::parse(buf).context(format!(
                "invalid bridge IFLA_PROTINFO {:?}",
                buf.value()
            ))?),
        })
    }
}
