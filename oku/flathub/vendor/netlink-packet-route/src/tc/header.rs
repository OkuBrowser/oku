// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::AddressFamily;

pub(crate) const TC_HEADER_LEN: usize = 20;

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
pub struct TcMessageBuffer {
    family: u8,
    pad1: u8,
    pad2: u16,
    index: i32,
    handle: u32,
    parent: u32,
    info: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct TcHeader {
    pub family: AddressFamily,
    // Interface index
    pub index: i32,
    // Qdisc handle
    pub handle: TcHandle,
    // Parent Qdisc
    pub parent: TcHandle,
    pub info: u32,
}

impl TcHeader {
    pub const TCM_IFINDEX_MAGIC_BLOCK: u32 = 0xFFFFFFFF;

    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            TcMessageBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), TC_HEADER_LEN)
            })?;
        Ok(Self {
            family: raw.family.into(),
            index: raw.index,
            handle: raw.handle.into(),
            parent: raw.parent.into(),
            info: raw.info,
        })
    }
}

impl From<&TcHeader> for TcMessageBuffer {
    fn from(header: &TcHeader) -> Self {
        Self {
            family: header.family.into(),
            pad1: 0,
            pad2: 0,
            index: header.index,
            handle: header.handle.into(),
            parent: header.parent.into(),
            info: header.info,
        }
    }
}

impl Emitable for TcHeader {
    fn buffer_len(&self) -> usize {
        TC_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcMessageBuffer::from(self);
        buffer[..TC_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct TcHandle {
    pub major: u16,
    pub minor: u16,
}

impl TcHandle {
    pub const UNSPEC: Self = Self { major: 0, minor: 0 };
    pub const ROOT: Self = Self {
        major: u16::MAX,
        minor: u16::MAX,
    };
    pub const INGRESS: Self = Self {
        major: u16::MAX,
        minor: 0xfff1,
    };

    pub const CLSACT: Self = Self::INGRESS;

    pub const MIN_PRIORITY: u16 = 0xFFE0;
    pub const MIN_INGRESS: u16 = 0xFFF2;
    pub const MIN_EGRESS: u16 = 0xFFF3;
}

impl From<u32> for TcHandle {
    fn from(d: u32) -> Self {
        let bytes = d.to_be_bytes();
        Self {
            major: u16::from_be_bytes([bytes[0], bytes[1]]),
            minor: u16::from_be_bytes([bytes[2], bytes[3]]),
        }
    }
}

impl From<TcHandle> for u32 {
    fn from(v: TcHandle) -> u32 {
        let major_bytes = v.major.to_be_bytes();
        let minor_bytes = v.minor.to_be_bytes();
        u32::from_be_bytes([
            major_bytes[0],
            major_bytes[1],
            minor_bytes[0],
            minor_bytes[1],
        ])
    }
}

impl std::fmt::Display for TcHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.major, self.minor)
    }
}
