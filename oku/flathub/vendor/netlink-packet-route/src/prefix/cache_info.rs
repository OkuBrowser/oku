// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct CacheInfo {
    pub preferred_time: u32,
    pub valid_time: u32,
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
pub struct CacheInfoBuffer {
    preferred_time: u32,
    valid_time: u32,
}

impl CacheInfo {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            CacheInfoBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<CacheInfoBuffer>(),
                )
            })?;
        Ok(CacheInfo {
            preferred_time: raw.preferred_time,
            valid_time: raw.valid_time,
        })
    }
}

impl From<&CacheInfo> for CacheInfoBuffer {
    fn from(value: &CacheInfo) -> Self {
        Self {
            preferred_time: value.preferred_time,
            valid_time: value.valid_time,
        }
    }
}

impl Emitable for CacheInfo {
    fn buffer_len(&self) -> usize {
        size_of::<CacheInfoBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = CacheInfoBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
