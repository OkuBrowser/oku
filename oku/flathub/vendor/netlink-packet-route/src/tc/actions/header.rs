// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::AddressFamily;

pub(crate) const TCA_HEADER_LEN: usize = 4;

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
pub struct TcActionMessageBuffer {
    family: u8,
    pad1: u8,
    pad2: u16,
}

/// Header for a traffic control action message.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct TcActionMessageHeader {
    /// Address family (usually `AddressFamily::Unspec`).
    pub family: AddressFamily,
}

impl TcActionMessageHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = TcActionMessageBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), TCA_HEADER_LEN)
            })?;
        Ok(Self {
            family: raw.family.into(),
        })
    }
}

impl From<&TcActionMessageHeader> for TcActionMessageBuffer {
    fn from(header: &TcActionMessageHeader) -> Self {
        Self {
            family: header.family.into(),
            pad1: 0,
            pad2: 0,
        }
    }
}

impl Emitable for TcActionMessageHeader {
    fn buffer_len(&self) -> usize {
        TCA_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = TcActionMessageBuffer::from(self);
        buffer[..TCA_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}
