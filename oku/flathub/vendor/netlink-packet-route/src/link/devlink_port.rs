// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u32, parse_string, parse_u32, DecodeError, DefaultNla, ErrorContext,
    Nla, NlaBuffer, Parseable,
};

const DEVLINK_ATTR_BUS_NAME: u16 = 1;
const DEVLINK_ATTR_DEV_NAME: u16 = 2;
const DEVLINK_ATTR_PORT_INDEX: u16 = 3;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum DevlinkPort {
    BusName(String),
    DevName(String),
    PortIndex(u32),
    Other(DefaultNla),
}

impl Nla for DevlinkPort {
    fn value_len(&self) -> usize {
        match self {
            Self::BusName(s) | Self::DevName(s) => s.len() + 1,
            Self::PortIndex(_) => 4,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::BusName(s) | Self::DevName(s) => {
                buffer[..s.len()].copy_from_slice(s.as_bytes());
                buffer[s.len()] = 0;
            }
            Self::PortIndex(v) => emit_u32(buffer, *v).unwrap(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::BusName(_) => DEVLINK_ATTR_BUS_NAME,
            Self::DevName(_) => DEVLINK_ATTR_DEV_NAME,
            Self::PortIndex(_) => DEVLINK_ATTR_PORT_INDEX,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for DevlinkPort {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            DEVLINK_ATTR_BUS_NAME => Self::BusName(
                parse_string(payload)
                    .context("invalid DEVLINK_ATTR_BUS_NAME value")?,
            ),
            DEVLINK_ATTR_DEV_NAME => Self::DevName(
                parse_string(payload)
                    .context("invalid DEVLINK_ATTR_DEV_NAME value")?,
            ),
            DEVLINK_ATTR_PORT_INDEX => Self::PortIndex(
                parse_u32(payload)
                    .context("invalid DEVLINK_ATTR_PORT_INDEX value")?,
            ),
            _ => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown DEVLINK_ATTR type {}",
                buf.kind()
            ))?),
        })
    }
}
