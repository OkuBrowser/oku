// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, DefaultNla, Nla, NlaBuffer, Parseable};

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoTeamPort {
    Other(DefaultNla),
}

impl Nla for InfoTeamPort {
    fn value_len(&self) -> usize {
        match self {
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoTeamPort {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        Ok(Self::Other(DefaultNla::parse(buf)?))
    }
}
