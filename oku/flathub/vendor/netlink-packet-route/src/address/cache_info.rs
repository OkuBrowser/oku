// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct CacheInfo {
    pub ifa_preferred: u32,
    pub ifa_valid: u32,
    pub cstamp: u32,
    pub tstamp: u32,
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
    ifa_preferred: u32,
    ifa_valid: u32,
    cstamp: u32,
    tstamp: u32,
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
        Ok(Self {
            ifa_preferred: raw.ifa_preferred,
            ifa_valid: raw.ifa_valid,
            cstamp: raw.cstamp,
            tstamp: raw.tstamp,
        })
    }
}

impl From<&CacheInfo> for CacheInfoBuffer {
    fn from(value: &CacheInfo) -> Self {
        Self {
            ifa_preferred: value.ifa_preferred,
            ifa_valid: value.ifa_valid,
            cstamp: value.cstamp,
            tstamp: value.tstamp,
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
