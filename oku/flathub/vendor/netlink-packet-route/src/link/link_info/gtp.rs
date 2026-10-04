// SPDX-License-Identifier: MIT

use std::net::{Ipv4Addr, Ipv6Addr};

use netlink_packet_core::{
    emit_u32, parse_u32, parse_u8, DecodeError, DefaultNla, ErrorContext, Nla,
    NlaBuffer, Parseable,
};

const IFLA_GTP_FD0: u16 = 1;
const IFLA_GTP_FD1: u16 = 2;
const IFLA_GTP_PDP_HASHSIZE: u16 = 3;
const IFLA_GTP_ROLE: u16 = 4;
const IFLA_GTP_CREATE_SOCKETS: u16 = 5;
const IFLA_GTP_RESTART_COUNT: u16 = 6;
const IFLA_GTP_LOCAL: u16 = 7;
const IFLA_GTP_LOCAL6: u16 = 8;

/// GTP role: GGSN or SGSN
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[non_exhaustive]
pub enum GtpRole {
    #[default]
    Ggsn,
    Sgsn,
    Other(u32),
}

impl From<u32> for GtpRole {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::Ggsn,
            1 => Self::Sgsn,
            _ => Self::Other(v),
        }
    }
}

impl From<GtpRole> for u32 {
    fn from(r: GtpRole) -> Self {
        match r {
            GtpRole::Ggsn => 0,
            GtpRole::Sgsn => 1,
            GtpRole::Other(v) => v,
        }
    }
}

impl std::fmt::Display for GtpRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtpRole::Ggsn => write!(f, "ggsn"),
            GtpRole::Sgsn => write!(f, "sgsn"),
            GtpRole::Other(v) => write!(f, "unknown({v})"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoGtp {
    Fd0(u32),
    Fd1(u32),
    PdpHashsize(u32),
    Role(GtpRole),
    CreateSockets(bool),
    RestartCount(u8),
    Local(Ipv4Addr),
    Local6(Ipv6Addr),
    Other(DefaultNla),
}

impl Nla for InfoGtp {
    fn value_len(&self) -> usize {
        match self {
            Self::Fd0(_)
            | Self::Fd1(_)
            | Self::PdpHashsize(_)
            | Self::Role(_) => 4,
            Self::CreateSockets(_) | Self::RestartCount(_) => 1,
            Self::Local(_) => 4,
            Self::Local6(_) => 16,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Fd0(value) | Self::Fd1(value) | Self::PdpHashsize(value) => {
                emit_u32(buffer, *value).unwrap()
            }
            Self::Role(role) => emit_u32(buffer, (*role).into()).unwrap(),
            Self::CreateSockets(value) => buffer[0] = *value as u8,
            Self::RestartCount(value) => buffer[0] = *value,
            Self::Local(value) => buffer.copy_from_slice(&value.octets()),
            Self::Local6(value) => buffer.copy_from_slice(&value.octets()),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Fd0(_) => IFLA_GTP_FD0,
            Self::Fd1(_) => IFLA_GTP_FD1,
            Self::PdpHashsize(_) => IFLA_GTP_PDP_HASHSIZE,
            Self::Role(_) => IFLA_GTP_ROLE,
            Self::CreateSockets(_) => IFLA_GTP_CREATE_SOCKETS,
            Self::RestartCount(_) => IFLA_GTP_RESTART_COUNT,
            Self::Local(_) => IFLA_GTP_LOCAL,
            Self::Local6(_) => IFLA_GTP_LOCAL6,
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoGtp {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_GTP_FD0 => Self::Fd0(
                parse_u32(payload).context("invalid IFLA_GTP_FD0 value")?,
            ),
            IFLA_GTP_FD1 => Self::Fd1(
                parse_u32(payload).context("invalid IFLA_GTP_FD1 value")?,
            ),
            IFLA_GTP_PDP_HASHSIZE => Self::PdpHashsize(
                parse_u32(payload)
                    .context("invalid IFLA_GTP_PDP_HASHSIZE value")?,
            ),
            IFLA_GTP_ROLE => Self::Role(GtpRole::from(
                parse_u32(payload).context("invalid IFLA_GTP_ROLE value")?,
            )),
            IFLA_GTP_CREATE_SOCKETS => Self::CreateSockets(
                parse_u8(payload)
                    .context("invalid IFLA_GTP_CREATE_SOCKETS value")?
                    > 0,
            ),
            IFLA_GTP_RESTART_COUNT => Self::RestartCount(
                parse_u8(payload)
                    .context("invalid IFLA_GTP_RESTART_COUNT value")?,
            ),
            IFLA_GTP_LOCAL => {
                if payload.len() == 4 {
                    let mut data = [0u8; 4];
                    data.copy_from_slice(&payload[0..4]);
                    Self::Local(Ipv4Addr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GTP_LOCAL, got unexpected length of \
                         IPv4 address payload {payload:?}"
                    )));
                }
            }
            IFLA_GTP_LOCAL6 => {
                if payload.len() == 16 {
                    let mut data = [0u8; 16];
                    data.copy_from_slice(&payload[0..16]);
                    Self::Local6(Ipv6Addr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFLA_GTP_LOCAL6, got unexpected length of \
                         IPv6 address payload {payload:?}"
                    )));
                }
            }
            unknown_kind => {
                Self::Other(DefaultNla::parse(buf).context(format!(
                    "unknown NLA type {unknown_kind} for gtp"
                ))?)
            }
        })
    }
}
