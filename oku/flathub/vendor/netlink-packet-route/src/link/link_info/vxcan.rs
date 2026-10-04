// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer, Parseable,
};

use super::super::LinkMessage;

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
        match *self {
            Self::Peer(ref message) => message.buffer_len(),
            Self::Other(ref attr) => attr.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match *self {
            Self::Peer(ref message) => message.emit(buffer),
            Self::Other(ref attr) => attr.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match *self {
            Self::Peer(_) => VXCAN_INFO_PEER,
            Self::Other(ref attr) => attr.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoVxcan {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            VXCAN_INFO_PEER => {
                let err = "failed to parse vxcan link info";
                Self::Peer(LinkMessage::parse(payload).context(err)?)
            }
            kind => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}
