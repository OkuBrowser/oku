// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const LINK_MAP_LEN: usize = 32;

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
pub struct MapBuffer {
    memory_start: u64,
    memory_end: u64,
    base_address: u64,
    irq: u16,
    dma: u8,
    port: u8,
    _padding: [u8; 4],
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct Map {
    pub memory_start: u64,
    pub memory_end: u64,
    pub base_address: u64,
    pub irq: u16,
    pub dma: u8,
    pub port: u8,
}

impl Map {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = MapBuffer::ref_from_prefix(payload).map_err(|_| {
            DecodeError::buffer_too_small(payload.len(), LINK_MAP_LEN)
        })?;
        Ok(Self {
            memory_start: raw.memory_start,
            memory_end: raw.memory_end,
            base_address: raw.base_address,
            irq: raw.irq,
            dma: raw.dma,
            port: raw.port,
        })
    }
}

impl From<&Map> for MapBuffer {
    fn from(map: &Map) -> Self {
        Self {
            memory_start: map.memory_start,
            memory_end: map.memory_end,
            base_address: map.base_address,
            irq: map.irq,
            dma: map.dma,
            port: map.port,
            _padding: [0; 4],
        }
    }
}

impl Emitable for Map {
    fn buffer_len(&self) -> usize {
        LINK_MAP_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = MapBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
