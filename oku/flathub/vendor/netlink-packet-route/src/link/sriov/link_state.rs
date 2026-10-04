// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const VF_INFO_LINK_STATE_LEN: usize = 8;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfInfoLinkState {
    pub vf_id: u32,
    pub state: VfLinkState,
}

impl VfInfoLinkState {
    pub fn new(vf_id: u32, state: VfLinkState) -> Self {
        Self { vf_id, state }
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
pub struct VfInfoLinkStateBuffer {
    vf_id: u32,
    state: u32,
}

impl VfInfoLinkState {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = VfInfoLinkStateBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    VF_INFO_LINK_STATE_LEN,
                )
            })?;
        Ok(Self::new(raw.vf_id, raw.state.into()))
    }
}

impl From<&VfInfoLinkState> for VfInfoLinkStateBuffer {
    fn from(link_state: &VfInfoLinkState) -> Self {
        Self {
            vf_id: link_state.vf_id,
            state: link_state.state.into(),
        }
    }
}

impl Emitable for VfInfoLinkState {
    fn buffer_len(&self) -> usize {
        VF_INFO_LINK_STATE_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfInfoLinkStateBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}

const IFLA_VF_LINK_STATE_AUTO: u32 = 0;
const IFLA_VF_LINK_STATE_ENABLE: u32 = 1;
const IFLA_VF_LINK_STATE_DISABLE: u32 = 2;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub enum VfLinkState {
    #[default]
    Auto,
    Enable,
    Disable,
    Other(u32),
}

impl From<u32> for VfLinkState {
    fn from(d: u32) -> Self {
        match d {
            IFLA_VF_LINK_STATE_AUTO => Self::Auto,
            IFLA_VF_LINK_STATE_ENABLE => Self::Enable,
            IFLA_VF_LINK_STATE_DISABLE => Self::Disable,
            _ => Self::Other(d),
        }
    }
}

impl From<VfLinkState> for u32 {
    fn from(v: VfLinkState) -> u32 {
        match v {
            VfLinkState::Auto => IFLA_VF_LINK_STATE_AUTO,
            VfLinkState::Enable => IFLA_VF_LINK_STATE_ENABLE,
            VfLinkState::Disable => IFLA_VF_LINK_STATE_DISABLE,
            VfLinkState::Other(d) => d,
        }
    }
}
