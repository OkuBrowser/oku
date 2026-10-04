// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u32, parse_u32, DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer,
    Parseable,
};

const IFLA_VRF_TABLE: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoVrf {
    TableId(u32),
    Other(DefaultNla),
}

impl Nla for InfoVrf {
    fn value_len(&self) -> usize {
        use self::InfoVrf::*;
        match self {
            TableId(_) => 4,
            Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        use self::InfoVrf::*;
        match self {
            TableId(value) => emit_u32(buffer, *value).unwrap(),
            Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        use self::InfoVrf::*;
        match self {
            TableId(_) => IFLA_VRF_TABLE,
            Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoVrf {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        use self::InfoVrf::*;
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_VRF_TABLE => TableId(
                parse_u32(payload).context("invalid IFLA_VRF_TABLE value")?,
            ),
            kind => Other(DefaultNla::parse(buf).context(format!(
                "unknown NLA type {kind} for IFLA_INFO_DATA(vrf)"
            ))?),
        })
    }
}
