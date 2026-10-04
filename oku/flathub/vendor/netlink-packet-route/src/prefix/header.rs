// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

pub(crate) const PREFIX_HEADER_LEN: usize = 12;

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
pub struct PrefixMessageBuffer {
    prefix_family: u8,
    pad1: u8,
    pad2: u16,
    ifindex: i32,
    prefix_type: u8,
    prefix_len: u8,
    flags: u8,
    pad3: u8,
}

// Linux kernel code `struct prefixmsg`
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct PrefixHeader {
    pub prefix_family: u8,
    pub ifindex: i32,
    pub prefix_type: u8,
    pub prefix_len: u8,
    pub flags: u8,
}

impl PrefixHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            PrefixMessageBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), PREFIX_HEADER_LEN)
            })?;
        Ok(Self {
            prefix_family: raw.prefix_family,
            ifindex: raw.ifindex,
            prefix_type: raw.prefix_type,
            prefix_len: raw.prefix_len,
            flags: raw.flags,
        })
    }
}

impl From<&PrefixHeader> for PrefixMessageBuffer {
    fn from(header: &PrefixHeader) -> Self {
        Self {
            prefix_family: header.prefix_family,
            pad1: 0,
            pad2: 0,
            ifindex: header.ifindex,
            prefix_type: header.prefix_type,
            prefix_len: header.prefix_len,
            flags: header.flags,
            pad3: 0,
        }
    }
}

impl Emitable for PrefixHeader {
    fn buffer_len(&self) -> usize {
        PREFIX_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = PrefixMessageBuffer::from(self);
        buffer[..PREFIX_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}
