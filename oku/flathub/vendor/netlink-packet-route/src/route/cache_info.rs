// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct RouteCacheInfo {
    pub clntref: u32,
    pub last_use: u32,
    pub expires: u32,
    pub error: u32,
    pub used: u32,
    pub id: u32,
    pub ts: u32,
    pub ts_age: u32,
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
pub struct RouteCacheInfoBuffer {
    clntref: u32,
    last_use: u32,
    expires: u32,
    error: u32,
    used: u32,
    id: u32,
    ts: u32,
    ts_age: u32,
}

impl RouteCacheInfo {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            RouteCacheInfoBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<RouteCacheInfoBuffer>(),
                )
            })?;
        Ok(Self {
            clntref: raw.clntref,
            last_use: raw.last_use,
            expires: raw.expires,
            error: raw.error,
            used: raw.used,
            id: raw.id,
            ts: raw.ts,
            ts_age: raw.ts_age,
        })
    }
}

impl From<&RouteCacheInfo> for RouteCacheInfoBuffer {
    fn from(value: &RouteCacheInfo) -> Self {
        Self {
            clntref: value.clntref,
            last_use: value.last_use,
            expires: value.expires,
            error: value.error,
            used: value.used,
            id: value.id,
            ts: value.ts,
            ts_age: value.ts_age,
        }
    }
}

impl Emitable for RouteCacheInfo {
    fn buffer_len(&self) -> usize {
        size_of::<RouteCacheInfoBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = RouteCacheInfoBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
