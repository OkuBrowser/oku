// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use netlink_packet_core::{
    emit_u16_be, emit_u32, emit_u32_be, parse_ip, parse_u16_be, parse_u32,
    parse_u32_be, parse_u8, DecodeError, DefaultNla, ErrorContext, Nla,
    NlaBuffer, Parseable,
};

const IFLA_GENEVE_ID: u16 = 1;
const IFLA_GENEVE_REMOTE: u16 = 2;
const IFLA_GENEVE_TTL: u16 = 3;
const IFLA_GENEVE_TOS: u16 = 4;
const IFLA_GENEVE_PORT: u16 = 5;
const IFLA_GENEVE_COLLECT_METADATA: u16 = 6;
const IFLA_GENEVE_REMOTE6: u16 = 7;
const IFLA_GENEVE_UDP_CSUM: u16 = 8;
const IFLA_GENEVE_UDP_ZERO_CSUM6_TX: u16 = 9;
const IFLA_GENEVE_UDP_ZERO_CSUM6_RX: u16 = 10;
const IFLA_GENEVE_LABEL: u16 = 11;
const IFLA_GENEVE_TTL_INHERIT: u16 = 12;
const IFLA_GENEVE_DF: u16 = 13;
const IFLA_GENEVE_INNER_PROTO_INHERIT: u16 = 14;
const IFLA_GENEVE_LOCAL: u16 = 17;
const IFLA_GENEVE_LOCAL6: u16 = 18;

const GENEVE_DF_UNSET: u8 = 0;
const GENEVE_DF_SET: u8 = 1;
const GENEVE_DF_INHERIT: u8 = 2;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum GeneveDf {
    Unset,
    Set,
    Inherit,
    Other(u8),
}

impl From<u8> for GeneveDf {
    fn from(d: u8) -> Self {
        match d {
            GENEVE_DF_UNSET => Self::Unset,
            GENEVE_DF_SET => Self::Set,
            GENEVE_DF_INHERIT => Self::Inherit,
            _ => Self::Other(d),
        }
    }
}

impl From<GeneveDf> for u8 {
    fn from(d: GeneveDf) -> Self {
        match d {
            GeneveDf::Unset => GENEVE_DF_UNSET,
            GeneveDf::Set => GENEVE_DF_SET,
            GeneveDf::Inherit => GENEVE_DF_INHERIT,
            GeneveDf::Other(value) => value,
        }
    }
}

impl std::fmt::Display for GeneveDf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unset => f.write_str("unset"),
            Self::Set => f.write_str("set"),
            Self::Inherit => f.write_str("inherit"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

impl std::str::FromStr for GeneveDf {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "unset" => Self::Unset,
            "set" => Self::Set,
            "inherit" => Self::Inherit,
            _ => {
                return Err(DecodeError::from(format!(
                    "unknown GENEVE DF: {s}"
                )))
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoGeneve {
    Id(u32),
    Remote(Ipv4Addr),
    Remote6(Ipv6Addr),
    Local(Ipv4Addr),
    Local6(Ipv6Addr),
    Ttl(u8),
    Tos(u8),
    Port(u16),
    CollectMetadata,
    UdpCsum(bool),
    UdpZeroCsum6Tx(bool),
    UdpZeroCsum6Rx(bool),
    Label(u32),
    TtlInherit(bool),
    Df(GeneveDf),
    InnerProtoInherit,
    Other(DefaultNla),
}

impl Nla for InfoGeneve {
    fn value_len(&self) -> usize {
        match self {
            Self::Id(_) | Self::Remote(_) | Self::Local(_) | Self::Label(_) => {
                4
            }
            Self::Remote6(_) | Self::Local6(_) => 16,
            Self::Ttl(_)
            | Self::Tos(_)
            | Self::UdpCsum(_)
            | Self::UdpZeroCsum6Tx(_)
            | Self::UdpZeroCsum6Rx(_)
            | Self::TtlInherit(_)
            | Self::Df(_) => 1,
            Self::Port(_) => 2,
            Self::CollectMetadata | Self::InnerProtoInherit => 0,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Id(value) => emit_u32(buffer, *value).unwrap(),
            Self::Remote(value) => buffer.copy_from_slice(&value.octets()),
            Self::Remote6(value) => buffer.copy_from_slice(&value.octets()),
            Self::Local(value) => buffer.copy_from_slice(&value.octets()),
            Self::Local6(value) => buffer.copy_from_slice(&value.octets()),
            Self::Ttl(value) | Self::Tos(value) => buffer[0] = *value,
            Self::Port(value) => emit_u16_be(buffer, *value).unwrap(),
            Self::CollectMetadata | Self::InnerProtoInherit => (),
            Self::UdpCsum(value)
            | Self::UdpZeroCsum6Tx(value)
            | Self::UdpZeroCsum6Rx(value)
            | Self::TtlInherit(value) => buffer[0] = *value as u8,
            Self::Label(value) => emit_u32_be(buffer, *value).unwrap(),
            Self::Df(value) => buffer[0] = (*value).into(),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Id(_) => IFLA_GENEVE_ID,
            Self::Remote(_) => IFLA_GENEVE_REMOTE,
            Self::Remote6(_) => IFLA_GENEVE_REMOTE6,
            Self::Local(_) => IFLA_GENEVE_LOCAL,
            Self::Local6(_) => IFLA_GENEVE_LOCAL6,
            Self::Ttl(_) => IFLA_GENEVE_TTL,
            Self::Tos(_) => IFLA_GENEVE_TOS,
            Self::Port(_) => IFLA_GENEVE_PORT,
            Self::CollectMetadata => IFLA_GENEVE_COLLECT_METADATA,
            Self::UdpCsum(_) => IFLA_GENEVE_UDP_CSUM,
            Self::UdpZeroCsum6Tx(_) => IFLA_GENEVE_UDP_ZERO_CSUM6_TX,
            Self::UdpZeroCsum6Rx(_) => IFLA_GENEVE_UDP_ZERO_CSUM6_RX,
            Self::Label(_) => IFLA_GENEVE_LABEL,
            Self::TtlInherit(_) => IFLA_GENEVE_TTL_INHERIT,
            Self::Df(_) => IFLA_GENEVE_DF,
            Self::InnerProtoInherit => IFLA_GENEVE_INNER_PROTO_INHERIT,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoGeneve {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_GENEVE_ID => Self::Id(
                parse_u32(payload).context("invalid IFLA_GENEVE_ID value")?,
            ),
            IFLA_GENEVE_REMOTE => match parse_ip(payload) {
                Ok(IpAddr::V4(addr)) => Self::Remote(addr),
                Ok(v) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_REMOTE, expecting IPv4 address, \
                         but got {v}"
                    )))
                }
                Err(e) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_REMOTE {e}"
                    )))
                }
            },
            IFLA_GENEVE_REMOTE6 => match parse_ip(payload) {
                Ok(IpAddr::V6(addr)) => Self::Remote6(addr),
                Ok(v) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_REMOTE6, expecting IPv6 address, \
                         but got {v}"
                    )))
                }
                Err(e) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_REMOTE6 {e}"
                    )))
                }
            },
            IFLA_GENEVE_LOCAL => match parse_ip(payload) {
                Ok(IpAddr::V4(addr)) => Self::Local(addr),
                Ok(v) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_LOCAL, expecting IPv4 address, \
                         but got {v}"
                    )))
                }
                Err(e) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_LOCAL {e}"
                    )))
                }
            },
            IFLA_GENEVE_LOCAL6 => match parse_ip(payload) {
                Ok(IpAddr::V6(addr)) => Self::Local6(addr),
                Ok(v) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_LOCAL6, expecting IPv6 address, \
                         but got {v}"
                    )))
                }
                Err(e) => {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GENEVE_LOCAL6 {e}"
                    )))
                }
            },
            IFLA_GENEVE_TTL => Self::Ttl(
                parse_u8(payload).context("invalid IFLA_GENEVE_TTL value")?,
            ),
            IFLA_GENEVE_TOS => Self::Tos(
                parse_u8(payload).context("invalid IFLA_GENEVE_TOS value")?,
            ),
            IFLA_GENEVE_PORT => Self::Port(
                parse_u16_be(payload)
                    .context("invalid IFLA_GENEVE_PORT value")?,
            ),
            IFLA_GENEVE_COLLECT_METADATA => Self::CollectMetadata,
            IFLA_GENEVE_UDP_CSUM => Self::UdpCsum(
                parse_u8(payload)
                    .context("invalid IFLA_GENEVE_UDP_CSUM value")?
                    > 0,
            ),
            IFLA_GENEVE_UDP_ZERO_CSUM6_TX => Self::UdpZeroCsum6Tx(
                parse_u8(payload)
                    .context("invalid IFLA_GENEVE_UDP_ZERO_CSUM6_TX value")?
                    > 0,
            ),
            IFLA_GENEVE_UDP_ZERO_CSUM6_RX => Self::UdpZeroCsum6Rx(
                parse_u8(payload)
                    .context("invalid IFLA_GENEVE_UDP_ZERO_CSUM6_RX value")?
                    > 0,
            ),
            IFLA_GENEVE_LABEL => Self::Label(
                parse_u32_be(payload)
                    .context("invalid IFLA_GENEVE_LABEL value")?,
            ),
            IFLA_GENEVE_TTL_INHERIT => Self::TtlInherit(
                parse_u8(payload)
                    .context("invalid IFLA_GENEVE_TTL_INHERIT value")?
                    > 0,
            ),
            IFLA_GENEVE_DF => Self::Df(
                parse_u8(payload)
                    .context("invalid IFLA_GENEVE_DF value")?
                    .into(),
            ),
            IFLA_GENEVE_INNER_PROTO_INHERIT => Self::InnerProtoInherit,
            kind => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}
