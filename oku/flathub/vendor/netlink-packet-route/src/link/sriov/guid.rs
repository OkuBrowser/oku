// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const VF_INFO_GUID_LEN: usize = 12;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoGuid {
    pub vf_id: u32,
    pub guid: u64,
}

impl VfInfoGuid {
    pub fn new(vf_id: u32, guid: u64) -> Self {
        Self { vf_id, guid }
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
pub struct VfInfoGuidBuffer {
    vf_id: u32,
    guid: u64,
}

impl VfInfoGuid {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            VfInfoGuidBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), VF_INFO_GUID_LEN)
            })?;
        Ok(Self::new(raw.vf_id, raw.guid))
    }
}

impl From<&VfInfoGuid> for VfInfoGuidBuffer {
    fn from(guid: &VfInfoGuid) -> Self {
        Self {
            vf_id: guid.vf_id,
            guid: guid.guid,
        }
    }
}

impl Emitable for VfInfoGuid {
    fn buffer_len(&self) -> usize {
        VF_INFO_GUID_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoGuidBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
