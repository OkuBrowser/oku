// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable, ErrorContext, Parseable};

use super::{
    attribute::VecNsidAttribute, header::NSID_HEADER_LEN, NsidAttribute,
    NsidHeader,
};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct NsidMessage {
    pub header: NsidHeader,
    pub attributes: Vec<NsidAttribute>,
}

impl Parseable<[u8]> for NsidMessage {
    fn parse(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            header: NsidHeader::parse(buf)
                .context("failed to parse nsid message header")?,
            attributes: VecNsidAttribute::parse(&buf[NSID_HEADER_LEN..])
                .context("failed to parse nsid message NLAs")?
                .0,
        })
    }
}

impl Emitable for NsidMessage {
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
