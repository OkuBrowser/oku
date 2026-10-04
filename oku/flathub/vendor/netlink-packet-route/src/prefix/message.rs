// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable, ErrorContext, Parseable};

use super::{
    attribute::{PrefixAttribute, VecPrefixAttribute},
    header::{PrefixHeader, PREFIX_HEADER_LEN},
};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct PrefixMessage {
    pub header: PrefixHeader,
    pub attributes: Vec<PrefixAttribute>,
}

impl Emitable for PrefixMessage {
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

impl Parseable<[u8]> for PrefixMessage {
    fn parse(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            header: PrefixHeader::parse(buf)
                .context("failed to parse prefix message header")?,
            attributes: VecPrefixAttribute::parse(&buf[PREFIX_HEADER_LEN..])
                .context("failed to parse prefix message attributes")?
                .0,
        })
    }
}
