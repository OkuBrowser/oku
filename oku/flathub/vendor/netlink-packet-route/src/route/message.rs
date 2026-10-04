// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, Emitable, ErrorContext, Parseable, ParseableParametrized,
};

use super::{
    attribute::VecRouteAttribute, header::ROUTE_HEADER_LEN, RouteAttribute,
    RouteHeader,
};

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct RouteMessage {
    pub header: RouteHeader,
    pub attributes: Vec<RouteAttribute>,
}

impl Emitable for RouteMessage {
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

impl Parseable<[u8]> for RouteMessage {
    fn parse(buf: &[u8]) -> Result<Self, DecodeError> {
        let header = RouteHeader::parse(buf)
            .context("failed to parse route message header")?;
        let address_family = header.address_family;
        let route_type = header.kind;
        Ok(RouteMessage {
            header,
            attributes: VecRouteAttribute::parse_with_param(
                &buf[ROUTE_HEADER_LEN..],
                (address_family, route_type),
            )
            .context("failed to parse route message NLAs")?
            .0,
        })
    }
}
