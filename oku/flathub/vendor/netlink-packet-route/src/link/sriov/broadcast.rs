// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const VF_INFO_BROADCAST_LEN: usize = 32;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoBroadcast {
    pub addr: [u8; VF_INFO_BROADCAST_LEN],
}

impl VfInfoBroadcast {
    pub fn new(addr: &[u8]) -> Self {
        let mut ret = Self::default();
        if addr.len() > VF_INFO_BROADCAST_LEN {
            ret.addr.copy_from_slice(&addr[..VF_INFO_BROADCAST_LEN])
        } else {
            ret.addr[..addr.len()].copy_from_slice(addr)
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
pub struct VfInfoBroadcastBuffer {
    addr: [u8; VF_INFO_BROADCAST_LEN],
}

impl VfInfoBroadcast {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = VfInfoBroadcastBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    VF_INFO_BROADCAST_LEN,
                )
            })?;
        Ok(Self::new(&raw.addr))
    }
}

impl From<&VfInfoBroadcast> for VfInfoBroadcastBuffer {
    fn from(broadcast: &VfInfoBroadcast) -> Self {
        Self {
            addr: broadcast.addr,
        }
    }
}

impl Emitable for VfInfoBroadcast {
    fn buffer_len(&self) -> usize {
        VF_INFO_BROADCAST_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoBroadcastBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
