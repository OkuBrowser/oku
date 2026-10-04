// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const VF_INFO_SPOOFCHK_LEN: usize = 8;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoSpoofCheck {
    pub vf_id: u32,
    pub enabled: bool,
}

impl VfInfoSpoofCheck {
    pub fn new(vf_id: u32, enabled: bool) -> Self {
        Self { vf_id, enabled }
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
pub struct VfInfoSpoofCheckBuffer {
    vf_id: u32,
    setting: u32,
}

impl VfInfoSpoofCheck {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = VfInfoSpoofCheckBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    VF_INFO_SPOOFCHK_LEN,
                )
            })?;
        Ok(Self::new(
            raw.vf_id,
            raw.setting > 0 && raw.setting != u32::MAX,
        ))
    }
}

impl From<&VfInfoSpoofCheck> for VfInfoSpoofCheckBuffer {
    fn from(spoofchk: &VfInfoSpoofCheck) -> Self {
        Self {
            vf_id: spoofchk.vf_id,
            setting: spoofchk.enabled as u32,
        }
    }
}

impl Emitable for VfInfoSpoofCheck {
    fn buffer_len(&self) -> usize {
        VF_INFO_SPOOFCHK_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoSpoofCheckBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
