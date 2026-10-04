// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Byte/Packet throughput statistics
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub struct TcStatsBasic {
    /// number of seen bytes
    pub bytes: u64,
    /// number of seen packets
    pub packets: u32,
}

// The real size is 12, but kernel aligns to 64 bits (8 bytes), hence the
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
pub struct TcStatsBasicBuffer {
    bytes: u64,
    packets: u32,
    _padding: [u8; 4],
}

impl TcStatsBasic {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            TcStatsBasicBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<TcStatsBasicBuffer>(),
                )
            })?;
        Ok(Self {
            bytes: raw.bytes,
            packets: raw.packets,
        })
    }
}

impl From<&TcStatsBasic> for TcStatsBasicBuffer {
    fn from(value: &TcStatsBasic) -> Self {
        Self {
            bytes: value.bytes,
            packets: value.packets,
            _padding: [0; 4],
        }
    }
}

impl Emitable for TcStatsBasic {
    fn buffer_len(&self) -> usize {
        size_of::<TcStatsBasicBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcStatsBasicBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
