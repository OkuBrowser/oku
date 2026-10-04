// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::AddressFamily;

pub(crate) const STATS_HEADER_LEN: usize = 12;

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
pub struct StatsMessageBuffer {
    family: u8,
    pad1: u8,
    pad2: u16,
    ifindex: u32,
    filter_mask: u32,
}

// The kernel uses IFLA_STATS_FILTER_BIT(attr) = 1 << (attr - 1)
// (see include/uapi/linux/if_link.h)
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[non_exhaustive]
    pub struct StatsFilterMask: u32 {
        const Link64 =
            1 << (super::attribute::IFLA_STATS_LINK_64 as u32 - 1);
        const LinkXstats =
            1 << (super::attribute::IFLA_STATS_LINK_XSTATS as u32 - 1);
        const LinkXstatsPort =
            1 << (super::attribute::IFLA_STATS_LINK_XSTATS_SLAVE as u32 - 1);
        const LinkOffloadXstats =
            1 << (super::attribute::IFLA_STATS_LINK_OFFLOAD_XSTATS as u32 - 1);
        const AfSpec =
            1 << (super::attribute::IFLA_STATS_AF_SPEC as u32 - 1);
        const _ = !0;
    }
}

impl Default for StatsFilterMask {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct StatsHeader {
    pub family: AddressFamily,
    pub ifindex: u32,
    pub filter_mask: StatsFilterMask,
}

impl StatsHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            StatsMessageBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), STATS_HEADER_LEN)
            })?;
        Ok(StatsHeader {
            family: raw.family.into(),
            ifindex: raw.ifindex,
            filter_mask: StatsFilterMask::from_bits_retain(raw.filter_mask),
        })
    }
}

impl From<&StatsHeader> for StatsMessageBuffer {
    fn from(header: &StatsHeader) -> Self {
        Self {
            family: header.family.into(),
            pad1: 0,
            pad2: 0,
            ifindex: header.ifindex,
            filter_mask: header.filter_mask.bits(),
        }
    }
}

impl Emitable for StatsHeader {
    fn buffer_len(&self) -> usize {
        STATS_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = StatsMessageBuffer::from(self);
        buffer[..STATS_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}
