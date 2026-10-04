// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use netlink_packet_core::{
    emit_i32, emit_u32, parse_i32, parse_string, parse_u32, parse_u8,
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer, Parseable,
};

use crate::address::{AddressFlags, CacheInfo, CacheInfoBuffer};

const IFA_ADDRESS: u16 = 1;
const IFA_LOCAL: u16 = 2;
const IFA_LABEL: u16 = 3;
const IFA_BROADCAST: u16 = 4;
const IFA_ANYCAST: u16 = 5;
const IFA_CACHEINFO: u16 = 6;
const IFA_MULTICAST: u16 = 7;
const IFA_FLAGS: u16 = 8;
const IFA_RT_PRIORITY: u16 = 9;
const IFA_TARGET_NETNSID: u16 = 10;
const IFA_PROTO: u16 = 11;

// 32 bites
const IPV4_ADDR_LEN: usize = 4;
// 128 bites
const IPV6_ADDR_LEN: usize = 16;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum AddressAttribute {
    Address(IpAddr),
    Local(IpAddr),
    Label(String),
    /// IPv4 only
    Broadcast(Ipv4Addr),
    /// IPv6 only
    Anycast(Ipv6Addr),
    CacheInfo(CacheInfo),
    /// IPv6 only
    Multicast(Ipv6Addr),
    Flags(AddressFlags),
    /// priority/metric for prefix route
    RoutePriority(u32),
    TargetNetNsId(i32),
    Protocol(AddressProtocol),
    Other(DefaultNla),
}

impl Nla for AddressAttribute {
    fn value_len(&self) -> usize {
        match *self {
            Self::Broadcast(_) => IPV4_ADDR_LEN,
            Self::Anycast(_) | Self::Multicast(_) => IPV6_ADDR_LEN,
            Self::Address(ref addr) | Self::Local(ref addr) => {
                if addr.is_ipv6() {
                    IPV6_ADDR_LEN
                } else {
                    IPV4_ADDR_LEN
                }
            }
            Self::Label(ref string) => string.len() + 1,
            Self::Flags(_)
            | Self::RoutePriority(_)
            | Self::TargetNetNsId(_) => 4,
            Self::CacheInfo(ref attr) => attr.buffer_len(),
            Self::Protocol(_) => 1,
            Self::Other(ref attr) => attr.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Broadcast(ref addr) => buffer.copy_from_slice(&addr.octets()),
            Self::Anycast(ref addr) | Self::Multicast(ref addr) => {
                buffer.copy_from_slice(&addr.octets())
            }
            Self::Address(ref addr) | Self::Local(ref addr) => match addr {
                IpAddr::V4(addr4) => buffer.copy_from_slice(&addr4.octets()),
                IpAddr::V6(addr6) => buffer.copy_from_slice(&addr6.octets()),
            },
            Self::Label(ref string) => {
                buffer[..string.len()].copy_from_slice(string.as_bytes());
                buffer[string.len()] = 0;
            }
            Self::Flags(ref value) => emit_u32(buffer, value.bits()).unwrap(),
            Self::CacheInfo(ref attr) => attr.emit(buffer),
            Self::RoutePriority(v) => emit_u32(buffer, *v).unwrap(),
            Self::TargetNetNsId(v) => emit_i32(buffer, *v).unwrap(),
            Self::Protocol(v) => buffer[0] = u8::from(*v),
            Self::Other(ref attr) => attr.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match *self {
            Self::Address(_) => IFA_ADDRESS,
            Self::Local(_) => IFA_LOCAL,
            Self::Label(_) => IFA_LABEL,
            Self::Broadcast(_) => IFA_BROADCAST,
            Self::Anycast(_) => IFA_ANYCAST,
            Self::CacheInfo(_) => IFA_CACHEINFO,
            Self::Multicast(_) => IFA_MULTICAST,
            Self::Flags(_) => IFA_FLAGS,
            Self::RoutePriority(_) => IFA_RT_PRIORITY,
            Self::TargetNetNsId(_) => IFA_TARGET_NETNSID,
            Self::Protocol(_) => IFA_PROTO,
            Self::Other(ref nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for AddressAttribute
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFA_ADDRESS => {
                if payload.len() == IPV4_ADDR_LEN {
                    let mut data = [0u8; IPV4_ADDR_LEN];
                    data.copy_from_slice(&payload[0..IPV4_ADDR_LEN]);
                    Self::Address(IpAddr::from(data))
                } else if payload.len() == IPV6_ADDR_LEN {
                    let mut data = [0u8; IPV6_ADDR_LEN];
                    data.copy_from_slice(&payload[0..IPV6_ADDR_LEN]);
                    Self::Address(IpAddr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFA_LOCAL, got unexpected length of payload \
                         {payload:?}"
                    )));
                }
            }
            IFA_LOCAL => {
                if payload.len() == IPV4_ADDR_LEN {
                    let mut data = [0u8; IPV4_ADDR_LEN];
                    data.copy_from_slice(&payload[0..IPV4_ADDR_LEN]);
                    Self::Local(IpAddr::from(data))
                } else if payload.len() == IPV6_ADDR_LEN {
                    let mut data = [0u8; IPV6_ADDR_LEN];
                    data.copy_from_slice(&payload[0..IPV6_ADDR_LEN]);
                    Self::Local(IpAddr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFA_LOCAL, got unexpected length of payload \
                         {payload:?}"
                    )));
                }
            }
            IFA_LABEL => Self::Label(
                parse_string(payload).context("invalid IFA_LABEL value")?,
            ),
            IFA_BROADCAST => {
                if payload.len() == IPV4_ADDR_LEN {
                    let mut data = [0u8; IPV4_ADDR_LEN];
                    data.copy_from_slice(&payload[0..IPV4_ADDR_LEN]);
                    Self::Broadcast(Ipv4Addr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFA_BROADCAST, got unexpected length of IPv4 \
                         address payload {payload:?}"
                    )));
                }
            }
            IFA_ANYCAST => {
                if payload.len() == IPV6_ADDR_LEN {
                    let mut data = [0u8; IPV6_ADDR_LEN];
                    data.copy_from_slice(&payload[0..IPV6_ADDR_LEN]);
                    Self::Anycast(Ipv6Addr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFA_ANYCAST, got unexpected length of IPv6 \
                         address payload {payload:?}"
                    )));
                }
            }
            IFA_CACHEINFO => Self::CacheInfo(
                CacheInfo::parse(&CacheInfoBuffer::new(payload))
                    .context(format!("Invalid IFA_CACHEINFO {payload:?}"))?,
            ),
            IFA_MULTICAST => {
                if payload.len() == IPV6_ADDR_LEN {
                    let mut data = [0u8; IPV6_ADDR_LEN];
                    data.copy_from_slice(&payload[0..IPV6_ADDR_LEN]);
                    Self::Multicast(Ipv6Addr::from(data))
                } else {
                    return Err(DecodeError::from(format!(
                        "Invalid IFA_MULTICAST, got unexpected length of IPv6 \
                         address payload {payload:?}"
                    )));
                }
            }
            IFA_FLAGS => Self::Flags(AddressFlags::from_bits_retain(
                parse_u32(payload).context("invalid IFA_FLAGS value")?,
            )),
            IFA_RT_PRIORITY => Self::RoutePriority(
                parse_u32(payload).context("invalid IFA_RT_PRIORITY value")?,
            ),
            IFA_TARGET_NETNSID => Self::TargetNetNsId(
                parse_i32(payload)
                    .context("invalid IFA_TARGET_NETNSID value")?,
            ),
            IFA_PROTO => Self::Protocol(
                parse_u8(payload).context("invalid IFA_PROTO value")?.into(),
            ),
            kind => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}

const IFAPROT_KERNEL_LO: u8 = 1;
const IFAPROT_KERNEL_RA: u8 = 2;
const IFAPROT_KERNEL_LL: u8 = 3;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum AddressProtocol {
    Loopback,
    RouterAnnouncement,
    LinkLocal,
    Other(u8),
}

impl From<u8> for AddressProtocol {
    fn from(d: u8) -> Self {
        match d {
            IFAPROT_KERNEL_LO => Self::Loopback,
            IFAPROT_KERNEL_RA => Self::RouterAnnouncement,
            IFAPROT_KERNEL_LL => Self::LinkLocal,
            _ => Self::Other(d),
        }
    }
}

impl From<AddressProtocol> for u8 {
    fn from(d: AddressProtocol) -> Self {
        match d {
            AddressProtocol::Loopback => IFAPROT_KERNEL_LO,
            AddressProtocol::RouterAnnouncement => IFAPROT_KERNEL_RA,
            AddressProtocol::LinkLocal => IFAPROT_KERNEL_LL,
            AddressProtocol::Other(d) => d,
        }
    }
}
