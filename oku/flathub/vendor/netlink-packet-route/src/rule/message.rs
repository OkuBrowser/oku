// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable, ErrorContext, Parseable};

use super::{
    attribute::VecRuleAttribute, header::RULE_HEADER_LEN, RuleAttribute,
    RuleHeader,
};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct RuleMessage {
    pub header: RuleHeader,
    pub attributes: Vec<RuleAttribute>,
}

impl Emitable for RuleMessage {
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

impl Parseable<[u8]> for RuleMessage {
    fn parse(buf: &[u8]) -> Result<Self, DecodeError> {
        let header = RuleHeader::parse(buf)
            .context("failed to parse rule message header")?;
        let attributes = VecRuleAttribute::parse(&buf[RULE_HEADER_LEN..])
            .context("failed to parse rule message NLAs")?
            .0;
        Ok(RuleMessage { header, attributes })
    }
}
