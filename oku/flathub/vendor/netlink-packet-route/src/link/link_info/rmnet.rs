// SPDX-License-Identifier: MIT

use bitflags::bitflags;
use netlink_packet_core::{
    emit_u16, emit_u32, parse_u16, parse_u32, DecodeError, DefaultNla,
    ErrorContext, Nla, NlaBuffer, Parseable,
};

const IFLA_RMNET_MUX_ID: u16 = 1;
const IFLA_RMNET_FLAGS: u16 = 2;

const RMNET_FLAGS_INGRESS_DEAGGREGATION: u32 = 1 << 0;
const RMNET_FLAGS_INGRESS_MAP_COMMANDS: u32 = 1 << 1;
const RMNET_FLAGS_INGRESS_MAP_CKSUMV4: u32 = 1 << 2;
const RMNET_FLAGS_EGRESS_MAP_CKSUMV4: u32 = 1 << 3;
const RMNET_FLAGS_INGRESS_MAP_CKSUMV5: u32 = 1 << 4;
const RMNET_FLAGS_EGRESS_MAP_CKSUMV5: u32 = 1 << 5;

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[non_exhaustive]
    pub struct RmNetFlags: u32 {
        const IngressDeaggregation = RMNET_FLAGS_INGRESS_DEAGGREGATION;
        const IngressCommands = RMNET_FLAGS_INGRESS_MAP_COMMANDS;
        const IngressMapCksumV4 = RMNET_FLAGS_INGRESS_MAP_CKSUMV4;
        const EgressMapCksumV4 = RMNET_FLAGS_EGRESS_MAP_CKSUMV4;
        const IngressMapCksumV5 = RMNET_FLAGS_INGRESS_MAP_CKSUMV5;
        const EgressMapCksumV5 = RMNET_FLAGS_EGRESS_MAP_CKSUMV5;
        const _ = !0;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct InfoRmNetFlags {
    pub flags: RmNetFlags,
    pub mask: RmNetFlags,
}

impl InfoRmNetFlags {
    pub fn new(flags: RmNetFlags, mask: RmNetFlags) -> Self {
        Self { flags, mask }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoRmNet {
    MuxId(u16),
    Flags(InfoRmNetFlags),
    Other(DefaultNla),
}

impl Nla for InfoRmNet {
    fn value_len(&self) -> usize {
        match self {
            Self::MuxId(_) => 2,
            Self::Flags(_) => 8,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::MuxId(value) => emit_u16(buffer, *value).unwrap(),
            Self::Flags(flags) => {
                emit_u32(buffer, flags.flags.bits()).unwrap();
                emit_u32(&mut buffer[4..], flags.mask.bits()).unwrap();
            }
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::MuxId(_) => IFLA_RMNET_MUX_ID,
            Self::Flags(_) => IFLA_RMNET_FLAGS,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoRmNet {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_RMNET_MUX_ID => Self::MuxId(
                parse_u16(payload)
                    .context("invalid IFLA_RMNET_MUX_ID value")?,
            ),
            IFLA_RMNET_FLAGS => {
                if payload.len() < 8 {
                    return Err(format!(
                        "invalid IFLA_RMNET_FLAGS: got {} bytes, expected at \
                         least 8",
                        payload.len()
                    )
                    .into());
                }
                Self::Flags(InfoRmNetFlags {
                    flags: RmNetFlags::from_bits_retain(
                        parse_u32(&payload[..4])
                            .context("invalid IFLA_RMNET_FLAGS value")?,
                    ),
                    mask: RmNetFlags::from_bits_retain(
                        parse_u32(&payload[4..])
                            .context("invalid IFLA_RMNET_FLAGS mask value")?,
                    ),
                })
            }
            _ => Self::Other(
                DefaultNla::parse(buf).context("unknown NLA type for rmnet")?,
            ),
        })
    }
}
