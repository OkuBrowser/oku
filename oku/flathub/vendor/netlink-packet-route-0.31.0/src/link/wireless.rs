// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u16, parse_u16, DecodeError, Emitable, Parseable,
};

const IWEVASSOCREQIE: u16 = 0x8C07;
const IWEVASSOCRESPIE: u16 = 0x8C08;

#[cfg(target_pointer_width = "32")]
const IW_HEADER_LEN: usize = 4;
#[cfg(not(target_pointer_width = "32"))]
const IW_HEADER_LEN: usize = 8;

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum WirelessEvent {
    /// IEs used for association request. Please use wl_nl80211 crate to parse.
    AssociateRequest(Vec<u8>),
    /// IEs used for association response. Please use wl_nl80211 crate to
    /// parse.
    AssociateResponse(Vec<u8>),
    /// Unknown variant, holds raw payload including length and command
    Other(Vec<u8>),
}

impl<T: AsRef<[u8]> + ?Sized> Parseable<T> for WirelessEvent {
    fn parse(buf: &T) -> Result<Self, DecodeError> {
        let payload = buf.as_ref();
        let cmd = parse_u16(&payload[2..4])?;
        let data = &payload[IW_HEADER_LEN..];
        Ok(match cmd {
            IWEVASSOCREQIE => Self::AssociateRequest(data.to_vec()),
            IWEVASSOCRESPIE => Self::AssociateResponse(data.to_vec()),
            _ => Self::Other(payload.to_vec()),
        })
    }
}

impl Emitable for WirelessEvent {
    fn buffer_len(&self) -> usize {
        match self {
            Self::AssociateRequest(v) | Self::AssociateResponse(v) => {
                IW_HEADER_LEN + v.len()
            }
            Self::Other(v) => v.len(),
        }
    }

    fn emit(&self, buffer: &mut [u8]) {
        buffer.fill(0);
        match self {
            Self::Other(v) => buffer.copy_from_slice(v),
            Self::AssociateRequest(v) => {
                emit_u16(buffer, self.buffer_len() as u16).unwrap();
                emit_u16(&mut buffer[2..4], IWEVASSOCREQIE).unwrap();
                buffer[IW_HEADER_LEN..v.len() + IW_HEADER_LEN]
                    .copy_from_slice(v.as_slice());
            }
            Self::AssociateResponse(v) => {
                emit_u16(buffer, self.buffer_len() as u16).unwrap();
                emit_u16(&mut buffer[2..4], IWEVASSOCRESPIE).unwrap();
                buffer[IW_HEADER_LEN..v.len() + IW_HEADER_LEN]
                    .copy_from_slice(v.as_slice());
            }
        }
    }
}
