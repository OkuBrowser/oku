// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::AddressFamily;

pub(crate) const NSID_HEADER_LEN: usize = 4;

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
pub struct NsidMessageBuffer {
    family: u8,
    _pad: [u8; 3],
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct NsidHeader {
    pub family: AddressFamily,
}

impl NsidHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            NsidMessageBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), NSID_HEADER_LEN)
            })?;
        Ok(NsidHeader {
            family: raw.family.into(),
        })
    }
}

impl From<&NsidHeader> for NsidMessageBuffer {
    fn from(header: &NsidHeader) -> Self {
        Self {
            family: header.family.into(),
            _pad: [0; 3],
        }
    }
}

impl Emitable for NsidHeader {
    fn buffer_len(&self) -> usize {
        NSID_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = NsidMessageBuffer::from(self);
        buffer[..NSID_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}
