// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub struct Inet6CacheInfo {
    pub max_reasm_len: i32,
    pub tstamp: i32,
    pub reachable_time: i32,
    pub retrans_time: i32,
}

const LINK_INET6_CACHE_INFO_LEN: usize = 16;

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
pub struct Inet6CacheInfoBuffer {
    max_reasm_len: i32,
    tstamp: i32,
    reachable_time: i32,
    retrans_time: i32,
}

impl Inet6CacheInfo {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            Inet6CacheInfoBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    LINK_INET6_CACHE_INFO_LEN,
                )
            })?;
        Ok(Self {
            max_reasm_len: raw.max_reasm_len,
            tstamp: raw.tstamp,
            reachable_time: raw.reachable_time,
            retrans_time: raw.retrans_time,
        })
    }
}

impl From<&Inet6CacheInfo> for Inet6CacheInfoBuffer {
    fn from(cache_info: &Inet6CacheInfo) -> Self {
        Self {
            max_reasm_len: cache_info.max_reasm_len,
            tstamp: cache_info.tstamp,
            reachable_time: cache_info.reachable_time,
            retrans_time: cache_info.retrans_time,
        }
    }
}

impl Emitable for Inet6CacheInfo {
    fn buffer_len(&self) -> usize {
        LINK_INET6_CACHE_INFO_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = Inet6CacheInfoBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
