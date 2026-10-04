// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u32, parse_u32, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const IFLA_WWAN_LINK_ID: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoWwan {
    LinkId(u32),
    Other(DefaultNla),
}

impl Nla for InfoWwan {
    fn value_len(&self) -> usize {
        match self {
            Self::LinkId(_) => 4,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::LinkId(value) => emit_u32(buffer, *value).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::LinkId(_) => IFLA_WWAN_LINK_ID,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoWwan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_WWAN_LINK_ID => Self::LinkId(
                parse_u32(payload)
                    .context("invalid IFLA_WWAN_LINK_ID value")?,
            ),
            _ => Self::Other(
                DefaultNla::parse(buf).context("unknown NLA type for wwan")?,
            ),
        })
    }
}
