// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable, ErrorContext, Parseable};

use super::{
    attribute::VecTcAttribute, header::TC_HEADER_LEN, TcAttribute, TcHeader,
};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct TcMessage {
    pub header: TcHeader,
    pub attributes: Vec<TcAttribute>,
}

impl TcMessage {
    pub fn into_parts(self) -> (TcHeader, Vec<TcAttribute>) {
        (self.header, self.attributes)
    }

    pub fn from_parts(header: TcHeader, attributes: Vec<TcAttribute>) -> Self {
        TcMessage { header, attributes }
    }

    /// Create a new `TcMessage` with the given index
    pub fn with_index(index: i32) -> Self {
        Self {
            header: TcHeader {
                index,
                ..Default::default()
            },
            attributes: Vec::new(),
        }
    }
}

impl Parseable<[u8]> for TcMessage {
    fn parse(buf: &[u8]) -> Result<Self, DecodeError> {
        let header = TcHeader::parse(buf)
            .context("failed to parse tc message header")?;
        Ok(Self {
            header,
            attributes: VecTcAttribute::parse(&buf[TC_HEADER_LEN..])
                .context("failed to parse tc message NLAs")?
                .0,
        })
    }
}

impl Emitable for TcMessage {
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
