// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable, ErrorContext, Parseable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::{
    address::{
        attribute::VecAddressAttribute, AddressAttribute, AddressHeaderFlags,
        AddressScope,
    },
    AddressFamily,
};

const ADDRESS_HEADER_LEN: usize = 8;

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
pub struct AddressMessageBuffer {
    family: u8,
    prefix_len: u8,
    flags: u8,
    scope: u8,
    index: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct AddressMessage {
    pub header: AddressHeader,
    pub attributes: Vec<AddressAttribute>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct AddressHeader {
    pub family: AddressFamily,
    pub prefix_len: u8,
    pub flags: AddressHeaderFlags,
    pub scope: AddressScope,
    pub index: u32,
}

impl AddressHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            AddressMessageBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), ADDRESS_HEADER_LEN)
            })?;
        Ok(Self {
            family: raw.family.into(),
            prefix_len: raw.prefix_len,
            flags: AddressHeaderFlags::from_bits_retain(raw.flags),
            scope: raw.scope.into(),
            index: raw.index,
        })
    }
}

impl From<&AddressHeader> for AddressMessageBuffer {
    fn from(header: &AddressHeader) -> Self {
        Self {
            family: header.family.into(),
            prefix_len: header.prefix_len,
            flags: header.flags.bits(),
            scope: header.scope.into(),
            index: header.index,
        }
    }
}

impl Emitable for AddressHeader {
    fn buffer_len(&self) -> usize {
        ADDRESS_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = AddressMessageBuffer::from(self);
        buffer[..ADDRESS_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}

impl Emitable for AddressMessage {
    fn buffer_len(&self) -> usize {
        self.header.buffer_len() + self.attributes.as_slice().buffer_len()
    }

    fn emit(&self, buffer: &mut [u8]) {
        self.header.emit(buffer);
        self.attributes
            .as_slice()
            .emit(&mut buffer[self.header.buffer_len()..]);
    }
}

impl Parseable<[u8]> for AddressMessage {
    fn parse(buf: &[u8]) -> Result<Self, DecodeError> {
        let header = AddressHeader::parse(buf)
            .context("failed to parse address message header")?;
        Ok(AddressMessage {
            header,
            attributes: VecAddressAttribute::parse(&buf[ADDRESS_HEADER_LEN..])
                .context("failed to parse address message NLAs")?
                .0,
        })
    }
}
