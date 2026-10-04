// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::{
    ip::{parse_ipv4_addr, parse_ipv6_addr, IPV4_ADDR_LEN, IPV6_ADDR_LEN},
    AddressFamily,
};

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
// Kernel representative is `struct rtvia`
// In Linux kernel 6.18, MPLS route also use `AF_PACKET`
// and MPLS route. Even the MPLS is using AF_PACKET, so we cannot simply
// treat `RouteVia` as `IpAddr`.
pub enum RouteVia {
    Inet(Ipv4Addr),
    Inet6(Ipv6Addr),
    #[cfg(any(target_os = "linux", target_os = "fuchsia"))]
    Packet(Vec<u8>),
    Other((AddressFamily, Vec<u8>)),
}

const RTVIA_LEN: usize = 2;

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
    Unaligned,
)]
#[repr(C, packed)]
pub struct RouteViaBuffer {
    address_family: u16,
}

impl RouteVia {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, address) =
            RouteViaBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), RTVIA_LEN)
            })?;
        let address_family: AddressFamily = (raw.address_family as u8).into();
        Ok(match address_family {
            AddressFamily::Inet => Self::Inet(parse_ipv4_addr(address)?),
            AddressFamily::Inet6 => Self::Inet6(parse_ipv6_addr(address)?),
            #[cfg(any(target_os = "linux", target_os = "fuchsia"))]
            AddressFamily::Packet => Self::Packet(address.to_vec()),
            _ => Self::Other((address_family, address.to_vec())),
        })
    }
}

impl Emitable for RouteVia {
    fn buffer_len(&self) -> usize {
        match self {
            Self::Inet(_) => IPV4_ADDR_LEN + 2,
            Self::Inet6(_) => IPV6_ADDR_LEN + 2,
            #[cfg(any(target_os = "linux", target_os = "fuchsia"))]
            Self::Packet(a) => a.len() + 2,
            Self::Other((_, a)) => a.len() + 2,
        }
    }

    fn emit(&self, buffer: &mut [u8]) {
        let (address_family, addr) = match self {
            Self::Inet(ip) => (AddressFamily::Inet, ip.octets().to_vec()),
            Self::Inet6(ip) => (AddressFamily::Inet6, ip.octets().to_vec()),
            #[cfg(any(target_os = "linux", target_os = "fuchsia"))]
            Self::Packet(a) => (AddressFamily::Packet, a.to_vec()),
            Self::Other((f, a)) => (*f, a.to_vec()),
        };
        let raw = RouteViaBuffer {
            address_family: u16::from(u8::from(address_family)),
        };
        buffer[..RTVIA_LEN].copy_from_slice(raw.as_bytes());
        buffer[RTVIA_LEN..].copy_from_slice(addr.as_slice());
    }
}

impl From<Ipv4Addr> for RouteVia {
    fn from(v: Ipv4Addr) -> Self {
        Self::Inet(v)
    }
}

impl From<Ipv6Addr> for RouteVia {
    fn from(v: Ipv6Addr) -> Self {
        Self::Inet6(v)
    }
}

impl From<IpAddr> for RouteVia {
    fn from(ip: IpAddr) -> Self {
        match ip {
            IpAddr::V4(ipv4) => Self::Inet(ipv4),
            IpAddr::V6(ipv6) => Self::Inet6(ipv6),
        }
    }
}
