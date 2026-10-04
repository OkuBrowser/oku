// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct NeighbourCacheInfo {
    pub confirmed: u32,
    pub used: u32,
    pub updated: u32,
    pub refcnt: u32,
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
pub struct NeighbourCacheInfoBuffer {
    confirmed: u32,
    used: u32,
    updated: u32,
    refcnt: u32,
}

impl NeighbourCacheInfo {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = NeighbourCacheInfoBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<NeighbourCacheInfoBuffer>(),
                )
            })?;
        Ok(Self {
            confirmed: raw.confirmed,
            used: raw.used,
            updated: raw.updated,
            refcnt: raw.refcnt,
        })
    }
}

impl From<&NeighbourCacheInfo> for NeighbourCacheInfoBuffer {
    fn from(value: &NeighbourCacheInfo) -> Self {
        Self {
            confirmed: value.confirmed,
            used: value.used,
            updated: value.updated,
            refcnt: value.refcnt,
        }
    }
}

impl Emitable for NeighbourCacheInfo {
    fn buffer_len(&self) -> usize {
        size_of::<NeighbourCacheInfoBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = NeighbourCacheInfoBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
