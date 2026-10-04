// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const MAX_ADDR_LEN: usize = 32;

const VF_INFO_MAC_LEN: usize = MAX_ADDR_LEN + 4;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoMac {
    pub vf_id: u32,
    pub mac: [u8; MAX_ADDR_LEN],
}

impl VfInfoMac {
    pub fn new(vf_id: u32, mac: &[u8]) -> Self {
        let mut ret = Self {
            vf_id,
            ..Default::default()
        };
        if mac.len() >= MAX_ADDR_LEN {
            ret.mac.copy_from_slice(&mac[..MAX_ADDR_LEN]);
        } else {
            ret.mac[..mac.len()].copy_from_slice(mac);
        }
        ret
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
pub struct VfInfoMacBuffer {
    vf_id: u32,
    mac: [u8; MAX_ADDR_LEN],
}

impl VfInfoMac {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            VfInfoMacBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), VF_INFO_MAC_LEN)
            })?;
        Ok(Self::new(raw.vf_id, &raw.mac))
    }
}

impl From<&VfInfoMac> for VfInfoMacBuffer {
    fn from(mac: &VfInfoMac) -> Self {
        Self {
            vf_id: mac.vf_id,
            mac: mac.mac,
        }
    }
}

impl Emitable for VfInfoMac {
    fn buffer_len(&self) -> usize {
        VF_INFO_MAC_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoMacBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
