// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const VF_INFO_TX_RATE_LEN: usize = 8;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoTxRate {
    pub vf_id: u32,
    pub rate: u32,
}

impl VfInfoTxRate {
    pub fn new(vf_id: u32, rate: u32) -> Self {
        Self { vf_id, rate }
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
pub struct VfInfoTxRateBuffer {
    vf_id: u32,
    rate: u32,
}

impl VfInfoTxRate {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            VfInfoTxRateBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    VF_INFO_TX_RATE_LEN,
                )
            })?;
        Ok(Self {
            vf_id: raw.vf_id,
            rate: raw.rate,
        })
    }
}

impl From<&VfInfoTxRate> for VfInfoTxRateBuffer {
    fn from(rate: &VfInfoTxRate) -> Self {
        Self {
            vf_id: rate.vf_id,
            rate: rate.rate,
        }
    }
}

impl Emitable for VfInfoTxRate {
    fn buffer_len(&self) -> usize {
        VF_INFO_TX_RATE_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoTxRateBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
