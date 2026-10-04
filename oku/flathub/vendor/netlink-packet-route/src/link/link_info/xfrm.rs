// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u32, parse_u32, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const IFLA_XFRM_LINK: u16 = 1;
const IFLA_XFRM_IF_ID: u16 = 2;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoXfrm {
    Link(u32),
    IfId(u32),
    Other(DefaultNla),
}

impl Nla for InfoXfrm {
    fn value_len(&self) -> usize {
        match self {
            Self::Link(_) => 4,
            Self::IfId(_) => 4,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Link(value) => emit_u32(buffer, *value).unwrap(),
            Self::IfId(value) => emit_u32(buffer, *value).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Link(_) => IFLA_XFRM_LINK,
            Self::IfId(_) => IFLA_XFRM_IF_ID,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoXfrm {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_XFRM_LINK => Self::Link(
                parse_u32(payload).context("invalid IFLA_XFRM_LINK value")?,
            ),
            IFLA_XFRM_IF_ID => Self::IfId(
                parse_u32(payload).context("invalid IFLA_XFRM_IF_ID value")?,
            ),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(xfrm)"
            ))?),
        })
    }
}
