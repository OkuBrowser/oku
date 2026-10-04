// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Generic queue statistics
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub struct TcStats {
    /// Number of enqueued bytes
    pub bytes: u64,
    /// Number of enqueued packets
    pub packets: u32,
    /// Packets dropped because of lack of resources
    pub drops: u32,
    /// Number of throttle events when this flow goes out of allocated
    /// bandwidth
    pub overlimits: u32,
    /// Current flow byte rate
    pub bps: u32,
    /// Current flow packet rate
    pub pps: u32,
    pub qlen: u32,
    pub backlog: u32,
}

// The real size is 36, but kernel aligns to 64 bits (8 bytes), hence the
// trailing padding.
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
pub struct TcStatsBuffer {
    bytes: u64,
    packets: u32,
    drops: u32,
    overlimits: u32,
    bps: u32,
    pps: u32,
    qlen: u32,
    backlog: u32,
    _padding: [u8; 4],
}

impl TcStats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            TcStatsBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<TcStatsBuffer>(),
                )
            })?;
        Ok(Self {
            bytes: raw.bytes,
            packets: raw.packets,
            drops: raw.drops,
            overlimits: raw.overlimits,
            bps: raw.bps,
            pps: raw.pps,
            qlen: raw.qlen,
            backlog: raw.backlog,
        })
    }
}

impl From<&TcStats> for TcStatsBuffer {
    fn from(value: &TcStats) -> Self {
        Self {
            bytes: value.bytes,
            packets: value.packets,
            drops: value.drops,
            overlimits: value.overlimits,
            bps: value.bps,
            pps: value.pps,
            qlen: value.qlen,
            backlog: value.backlog,
            _padding: [0; 4],
        }
    }
}

impl Emitable for TcStats {
    fn buffer_len(&self) -> usize {
        size_of::<TcStatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcStatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
