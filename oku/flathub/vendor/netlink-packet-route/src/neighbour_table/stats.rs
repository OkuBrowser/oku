// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct NeighbourTableStats {
    pub allocs: u64,
    pub destroys: u64,
    pub hash_grows: u64,
    pub res_failed: u64,
    pub lookups: u64,
    pub hits: u64,
    pub multicast_probes_received: u64,
    pub unicast_probes_received: u64,
    pub periodic_gc_runs: u64,
    pub forced_gc_runs: u64,
    pub table_fulls: u64,
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
pub struct NeighbourTableStatsBuffer {
    allocs: u64,
    destroys: u64,
    hash_grows: u64,
    res_failed: u64,
    lookups: u64,
    hits: u64,
    multicast_probes_received: u64,
    unicast_probes_received: u64,
    periodic_gc_runs: u64,
    forced_gc_runs: u64,
    table_fulls: u64,
}

impl NeighbourTableStats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = NeighbourTableStatsBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<NeighbourTableStatsBuffer>(),
                )
            })?;
        Ok(Self {
            allocs: raw.allocs,
            destroys: raw.destroys,
            hash_grows: raw.hash_grows,
            res_failed: raw.res_failed,
            lookups: raw.lookups,
            hits: raw.hits,
            multicast_probes_received: raw.multicast_probes_received,
            unicast_probes_received: raw.unicast_probes_received,
            periodic_gc_runs: raw.periodic_gc_runs,
            forced_gc_runs: raw.forced_gc_runs,
            table_fulls: raw.table_fulls,
        })
    }
}

impl From<&NeighbourTableStats> for NeighbourTableStatsBuffer {
    fn from(value: &NeighbourTableStats) -> Self {
        Self {
            allocs: value.allocs,
            destroys: value.destroys,
            hash_grows: value.hash_grows,
            res_failed: value.res_failed,
            lookups: value.lookups,
            hits: value.hits,
            multicast_probes_received: value.multicast_probes_received,
            unicast_probes_received: value.unicast_probes_received,
            periodic_gc_runs: value.periodic_gc_runs,
            forced_gc_runs: value.forced_gc_runs,
            table_fulls: value.table_fulls,
        }
    }
}

impl Emitable for NeighbourTableStats {
    fn buffer_len(&self) -> usize {
        size_of::<NeighbourTableStatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = NeighbourTableStatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
