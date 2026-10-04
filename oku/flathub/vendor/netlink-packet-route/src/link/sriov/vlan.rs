// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const VF_INFO_VLAN_LEN: usize = 12;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoVlan {
    pub vf_id: u32,
    pub vlan_id: u32,
    pub qos: u32,
}

impl VfInfoVlan {
    pub fn new(vf_id: u32, vlan_id: u32, qos: u32) -> Self {
        Self {
            vf_id,
            vlan_id,
            qos,
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
pub struct VfInfoVlanBuffer {
    vf_id: u32,
    vlan_id: u32,
    qos: u32,
}

impl VfInfoVlan {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            VfInfoVlanBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), VF_INFO_VLAN_LEN)
            })?;
        Ok(Self {
            vf_id: raw.vf_id,
            vlan_id: raw.vlan_id,
            qos: raw.qos,
        })
    }
}

impl From<&VfInfoVlan> for VfInfoVlanBuffer {
    fn from(vlan: &VfInfoVlan) -> Self {
        Self {
            vf_id: vlan.vf_id,
            vlan_id: vlan.vlan_id,
            qos: vlan.qos,
        }
    }
}

impl Emitable for VfInfoVlan {
    fn buffer_len(&self) -> usize {
        VF_INFO_VLAN_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoVlanBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
