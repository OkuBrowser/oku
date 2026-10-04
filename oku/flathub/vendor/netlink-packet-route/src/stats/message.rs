// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable, ErrorContext, Parseable};

use crate::stats::{
    attribute::VecStatsAttribute, header::STATS_HEADER_LEN, StatsAttribute,
    StatsHeader,
};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct StatsMessage {
    pub header: StatsHeader,
    pub attributes: Vec<StatsAttribute>,
}

impl Parseable<[u8]> for StatsMessage {
    fn parse(buf: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            header: StatsHeader::parse(buf)
                .context("failed to parse stats message header")?,
            attributes: VecStatsAttribute::parse(&buf[STATS_HEADER_LEN..])
                .context("failed to parse stats message NLAs")?
                .0,
        })
    }
}

impl Emitable for StatsMessage {
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
