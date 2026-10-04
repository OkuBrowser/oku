// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const VF_INFO_RATE_LEN: usize = 12;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoRate {
    pub vf_id: u32,
    pub min_tx_rate: u32,
    pub max_tx_rate: u32,
}

impl VfInfoRate {
    pub fn new(vf_id: u32, min_tx_rate: u32, max_tx_rate: u32) -> Self {
        Self {
            vf_id,
            min_tx_rate,
            max_tx_rate,
        }
    }
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
pub struct VfInfoRateBuffer {
    vf_id: u32,
    min_tx_rate: u32,
    max_tx_rate: u32,
}

impl VfInfoRate {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            VfInfoRateBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), VF_INFO_RATE_LEN)
            })?;
        Ok(Self {
            vf_id: raw.vf_id,
            min_tx_rate: raw.min_tx_rate,
            max_tx_rate: raw.max_tx_rate,
        })
    }
}

impl From<&VfInfoRate> for VfInfoRateBuffer {
    fn from(rate: &VfInfoRate) -> Self {
        Self {
            vf_id: rate.vf_id,
            min_tx_rate: rate.min_tx_rate,
            max_tx_rate: rate.max_tx_rate,
        }
    }
}

impl Emitable for VfInfoRate {
    fn buffer_len(&self) -> usize {
        VF_INFO_RATE_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoRateBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
