// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct NeighbourTableConfig {
    pub key_len: u16,
    pub entry_size: u16,
    pub entries: u32,
    pub last_flush: u32,
    pub last_rand: u32,
    pub hash_rand: u32,
    pub hash_mask: u32,
    pub hash_chain_gc: u32,
    pub proxy_qlen: u32,
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
pub struct NeighbourTableConfigBuffer {
    key_len: u16,
    entry_size: u16,
    entries: u32,
    last_flush: u32,
    last_rand: u32,
    hash_rand: u32,
    hash_mask: u32,
    hash_chain_gc: u32,
    proxy_qlen: u32,
}

impl NeighbourTableConfig {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = NeighbourTableConfigBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<NeighbourTableConfigBuffer>(),
                )
            })?;
        Ok(Self {
            key_len: raw.key_len,
            entry_size: raw.entry_size,
            entries: raw.entries,
            last_flush: raw.last_flush,
            last_rand: raw.last_rand,
            hash_rand: raw.hash_rand,
            hash_mask: raw.hash_mask,
            hash_chain_gc: raw.hash_chain_gc,
            proxy_qlen: raw.proxy_qlen,
        })
    }
}

impl From<&NeighbourTableConfig> for NeighbourTableConfigBuffer {
    fn from(value: &NeighbourTableConfig) -> Self {
        Self {
            key_len: value.key_len,
            entry_size: value.entry_size,
            entries: value.entries,
            last_flush: value.last_flush,
            last_rand: value.last_rand,
            hash_rand: value.hash_rand,
            hash_mask: value.hash_mask,
            hash_chain_gc: value.hash_chain_gc,
            proxy_qlen: value.proxy_qlen,
        }
    }
}

impl Emitable for NeighbourTableConfig {
    fn buffer_len(&self) -> usize {
        size_of::<NeighbourTableConfigBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = NeighbourTableConfigBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
