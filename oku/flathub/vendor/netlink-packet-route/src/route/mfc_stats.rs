// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct RouteMfcStats {
    pub packets: u64,
    pub bytes: u64,
    pub wrong_if: u64,
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
pub struct RouteMfcStatsBuffer {
    packets: u64,
    bytes: u64,
    wrong_if: u64,
}

impl RouteMfcStats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            RouteMfcStatsBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<RouteMfcStatsBuffer>(),
                )
            })?;
        Ok(Self {
            packets: raw.packets,
            bytes: raw.bytes,
            wrong_if: raw.wrong_if,
        })
    }
}

impl From<&RouteMfcStats> for RouteMfcStatsBuffer {
    fn from(value: &RouteMfcStats) -> Self {
        Self {
            packets: value.packets,
            bytes: value.bytes,
            wrong_if: value.wrong_if,
        }
    }
}

impl Emitable for RouteMfcStats {
    fn buffer_len(&self) -> usize {
        size_of::<RouteMfcStatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = RouteMfcStatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
