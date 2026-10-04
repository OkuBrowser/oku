// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer, Parseable,
};

use super::super::{LinkMessage, LinkMessageBuffer};

const VXCAN_INFO_PEER: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
// This data is not for querying/dumping as in kernel 6.19,
// because the `struct rtnl_link_ops vxcan_link_ops` does not have `fill_info`.
// Only for create vxcan
pub enum InfoVxcan {
    Peer(LinkMessage),
    Other(DefaultNla),
}

impl Nla for InfoVxcan {
    fn value_len(&self) -> usize {
        use self::InfoVxcan::*;
        match *self {
            Peer(ref message) => message.buffer_len(),
            Other(ref attr) => attr.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        use self::InfoVxcan::*;
        match *self {
            Peer(ref message) => message.emit(buffer),
            Other(ref attr) => attr.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        use self::InfoVxcan::*;
        match *self {
            Peer(_) => VXCAN_INFO_PEER,
            Other(ref attr) => attr.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoVxcan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        use self::InfoVxcan::*;
        let payload = buf.value();
        Ok(match buf.kind() {
            VXCAN_INFO_PEER => {
                let err = "failed to parse vxcan link info";
                let buffer =
                    LinkMessageBuffer::new_checked(&payload).context(err)?;
                Peer(LinkMessage::parse(&buffer).context(err)?)
            }
            kind => Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}
