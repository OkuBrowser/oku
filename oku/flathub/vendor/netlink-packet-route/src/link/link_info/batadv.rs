// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    parse_string, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const IFLA_BATADV_ALGO_NAME: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoBatAdv {
    AlgoName(String),
    Other(DefaultNla),
}

impl Nla for InfoBatAdv {
    fn value_len(&self) -> usize {
        match self {
            Self::AlgoName(v) => v.len() + 1,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::AlgoName(v) => {
                let s = v.as_bytes();
                buffer[..s.len()].copy_from_slice(s);
                buffer[s.len()] = 0;
            }
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::AlgoName(_) => IFLA_BATADV_ALGO_NAME,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoBatAdv {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_BATADV_ALGO_NAME => Self::AlgoName(
                parse_string(payload)
                    .context("invalid IFLA_BATADV_ALGO_NAME value")?,
            ),
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(batadv)"
            ))?),
        })
    }
}
