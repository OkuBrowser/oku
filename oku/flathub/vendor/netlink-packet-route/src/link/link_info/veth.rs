// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer, Parseable,
};

use super::super::LinkMessage;

const VETH_INFO_PEER: u16 = 1;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
// This data is not for querying/dumping as in kernel 6.5.8,
// because the `struct rtnl_link_ops veth_link_ops` does not have `fill_info`.
// Only for create veth
pub enum InfoVeth {
    Peer(LinkMessage),
    Other(DefaultNla),
}

impl Nla for InfoVeth {
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
            Self::Peer(_) => VETH_INFO_PEER,
            Self::Other(ref attr) => attr.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoVeth {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            VETH_INFO_PEER => {
                let err = "failed to parse veth link info";
                Self::Peer(LinkMessage::parse(payload).context(err)?)
            }
            kind => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}
