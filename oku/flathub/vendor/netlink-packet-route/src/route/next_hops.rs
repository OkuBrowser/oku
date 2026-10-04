// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{
    DecodeError, Emitable, ErrorContext, ParseableParametrized,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::{
    super::AddressFamily, attribute::VecRouteAttribute, RouteAttribute,
    RouteLwEnCapType, RouteType,
};

pub(crate) const RTNH_F_DEAD: u8 = 1;
pub(crate) const RTNH_F_PERVASIVE: u8 = 2;
pub(crate) const RTNH_F_ONLINK: u8 = 4;
pub(crate) const RTNH_F_OFFLOAD: u8 = 8;
pub(crate) const RTNH_F_LINKDOWN: u8 = 16;
pub(crate) const RTNH_F_UNRESOLVED: u8 = 32;
pub(crate) const RTNH_F_TRAP: u8 = 64;

bitflags! {
    #[derive(Clone, Eq, PartialEq, Debug, Copy, Default)]
    #[non_exhaustive]
    pub struct RouteNextHopFlags: u8 {
        const Dead = RTNH_F_DEAD;
        const Pervasive = RTNH_F_PERVASIVE;
        const Onlink = RTNH_F_ONLINK;
        const Offload = RTNH_F_OFFLOAD;
        const Linkdown = RTNH_F_LINKDOWN;
        const Unresolved = RTNH_F_UNRESOLVED;
        const Trap = RTNH_F_TRAP;
        const _ = !0;
    }
}

impl std::fmt::Display for RouteNextHopFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        if self.contains(Self::Dead) {
            write!(f, "dead")?;
            first = false;
        }
        if self.contains(Self::Onlink) {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "onlink")?;
            first = false;
        }
        if self.contains(Self::Pervasive) {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "pervasive")?;
            first = false;
        }
        if self.contains(Self::Offload) {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "offload")?;
            first = false;
        }
        if self.contains(Self::Linkdown) {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "linkdown")?;
            first = false;
        }
        if self.contains(Self::Unresolved) {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "unresolved")?;
            first = false;
        }
        if self.contains(Self::Trap) {
            if !first {
                write!(f, " ")?;
            }
            write!(f, "trap")?;
        }
        Ok(())
    }
}

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
pub struct RouteNextHopBuffer {
    length: u16,
    flags: u8,
    hops: u8,
    interface_index: u32,
}

impl RouteNextHopBuffer {
    pub fn new_checked(buffer: &[u8]) -> Result<&Self, DecodeError> {
        let len = buffer.len();
        let header_len = size_of::<Self>();
        if len < header_len {
            return Err(format!(
                "invalid RouteNextHopBuffer: length {len} < {header_len}"
            )
            .into());
        }
        let raw = Self::ref_from_prefix(buffer)
            .map_err(|_| DecodeError::buffer_too_small(len, header_len))?
            .0;
        let length = raw.length;
        if len < length as usize {
            return Err(format!(
                "invalid RouteNextHopBuffer: length {len} < {length}"
            )
            .into());
        }
        if (length as usize) < header_len {
            return Err(format!(
                "invalid RouteNextHopBuffer: length {length} < {header_len}"
            )
            .into());
        }
        Ok(raw)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct RouteNextHop {
    /// Next-hop flags
    pub flags: RouteNextHopFlags,
    /// Next-hop priority
    pub hops: u8,
    /// Interface index for the next-hop
    pub interface_index: u32,
    /// Attributes
    pub attributes: Vec<RouteAttribute>,
}

impl ParseableParametrized<[u8], (AddressFamily, RouteType, RouteLwEnCapType)>
    for RouteNextHop
{
    fn parse_with_param(
        buf: &[u8],
        (address_family, route_type, encap_type): (
            AddressFamily,
            RouteType,
            RouteLwEnCapType,
        ),
    ) -> Result<RouteNextHop, DecodeError> {
        let err = "cannot parse route attributes in next-hop";
        let nh_buf = RouteNextHopBuffer::new_checked(buf).context(err)?;
        let length = nh_buf.length as usize;
        let attributes = VecRouteAttribute::parse_with_param(
            &buf[size_of::<RouteNextHopBuffer>()..length],
            (address_family, route_type, encap_type),
        )
        .context(err)?
        .0;
        Ok(RouteNextHop {
            flags: RouteNextHopFlags::from_bits_retain(nh_buf.flags),
            hops: nh_buf.hops,
            interface_index: nh_buf.interface_index,
            attributes,
        })
    }
}

impl Emitable for RouteNextHop {
    fn buffer_len(&self) -> usize {
        // len, flags, hops and interface id fields
        size_of::<RouteNextHopBuffer>()
            + self.attributes.as_slice().buffer_len()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = RouteNextHopBuffer::from(self);
        buffer[..size_of::<RouteNextHopBuffer>()]
            .copy_from_slice(raw.as_bytes());
        self.attributes
            .as_slice()
            .emit(&mut buffer[size_of::<RouteNextHopBuffer>()..]);
    }
}

impl From<&RouteNextHop> for RouteNextHopBuffer {
    fn from(nh: &RouteNextHop) -> Self {
        Self {
            length: nh.buffer_len() as u16,
            flags: nh.flags.bits(),
            hops: nh.hops,
            interface_index: nh.interface_index,
        }
    }
}

// Parse a `RTA_MULTIPATH` attribute value: a sequence of variable-length
// `rtnexthop` entries, each carrying its own total length.
pub(crate) fn parse_multipath_next_hops(
    buf: &[u8],
    (address_family, route_type, encap_type): (
        AddressFamily,
        RouteType,
        RouteLwEnCapType,
    ),
) -> Result<Vec<RouteNextHop>, DecodeError> {
    let mut next_hops = vec![];
    let mut buf = buf;
    loop {
        let length = RouteNextHopBuffer::new_checked(buf)?.length as usize;
        next_hops.push(RouteNextHop::parse_with_param(
            buf,
            (address_family, route_type, encap_type),
        )?);
        if buf.len() == length {
            break;
        }
        buf = &buf[length..];
    }
    Ok(next_hops)
}
