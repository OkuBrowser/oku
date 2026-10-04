// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer, Parseable,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::link::VlanProtocol;

const IFLA_VF_VLAN_INFO: u16 = 1;

#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum VfVlan {
    Info(VfVlanInfo),
    Other(DefaultNla),
}

impl Nla for VfVlan {
    fn value_len(&self) -> usize {
        match self {
            Self::Info(v) => v.buffer_len(),
            Self::Other(v) => v.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Info(v) => v.emit(buffer),
            Self::Other(attr) => attr.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Info(_) => IFLA_VF_VLAN_INFO,
            Self::Other(v) => v.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for VfVlan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_VF_VLAN_INFO => {
                Self::Info(VfVlanInfo::parse(payload).context(format!(
                    "invalid IFLA_VF_VLAN_INFO {payload:?}"
                ))?)
            }
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "failed to parse {kind} as DefaultNla: {payload:?}"
            ))?),
        })
    }
}

const VF_VLAN_INFO_LEN: usize = 16; // with 2 bytes padding

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct VfVlanInfo {
    pub vf_id: u32,
    pub vlan_id: u32,
    pub qos: u32,
    pub protocol: VlanProtocol,
}

impl VfVlanInfo {
    pub fn new(
        vf_id: u32,
        vlan_id: u32,
        qos: u32,
        protocol: VlanProtocol,
    ) -> Self {
        Self {
            vf_id,
            vlan_id,
            qos,
            protocol,
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
pub struct VfVlanInfoBuffer {
    vf_id: u32,
    vlan_id: u32,
    qos: u32,
    protocol: u16,
    _padding: [u8; 2],
}

impl VfVlanInfo {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            VfVlanInfoBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), VF_VLAN_INFO_LEN)
            })?;
        Ok(Self {
            vf_id: raw.vf_id,
            vlan_id: raw.vlan_id,
            qos: raw.qos,
            protocol: u16::from_be(raw.protocol).into(),
        })
    }
}

impl From<&VfVlanInfo> for VfVlanInfoBuffer {
    fn from(vlan_info: &VfVlanInfo) -> Self {
        Self {
            vf_id: vlan_info.vf_id,
            vlan_id: vlan_info.vlan_id,
            qos: vlan_info.qos,
            protocol: u16::from(vlan_info.protocol).to_be(),
            _padding: [0; 2],
        }
    }
}

impl Emitable for VfVlanInfo {
    fn buffer_len(&self) -> usize {
        VF_VLAN_INFO_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = VfVlanInfoBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
