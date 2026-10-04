// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    parse_string, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const IFLA_ALT_IFNAME: u16 = 53;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum Prop {
    AltIfName(String),
    Other(DefaultNla),
}

impl Nla for Prop {
    fn value_len(&self) -> usize {
        match self {
            Self::AltIfName(ref string) => string.len() + 1,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::AltIfName(ref string) => {
                buffer[..string.len()].copy_from_slice(string.as_bytes());
                buffer[string.len()] = 0;
            }
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::AltIfName(_) => IFLA_ALT_IFNAME,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for Prop {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_ALT_IFNAME => Prop::AltIfName(
                parse_string(payload)
                    .context("invalid IFLA_ALT_IFNAME value")?,
            ),
            kind => Prop::Other(
                DefaultNla::parse(buf)
                    .context(format!("Unknown NLA type {kind}"))?,
            ),
        })
    }
}
