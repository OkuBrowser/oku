// SPDX-License-Identifier: MIT

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use super::{super::AddressFamily, flags::RuleFlags, RuleAction};

pub(crate) const RULE_HEADER_LEN: usize = 12;

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    FromBytes,
    IntoBytes,
    KnownLayout,
    Immutable,
    Unaligned,
)]
#[repr(C, packed)]
pub struct RuleMessageBuffer {
    family: u8,
    dst_len: u8,
    src_len: u8,
    tos: u8,
    table: u8,
    reserve_1: u8,
    reserve_2: u8,
    action: u8,
    flags: u32,
}

// Linux kernel code `struct fib_rule_hdr`
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct RuleHeader {
    pub family: AddressFamily,
    pub dst_len: u8,
    pub src_len: u8,
    pub tos: u8,
    pub table: u8,
    pub action: RuleAction,
    pub flags: RuleFlags,
}

impl Emitable for RuleHeader {
    fn buffer_len(&self) -> usize {
        RULE_HEADER_LEN
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = RuleMessageBuffer::from(self);
        buffer[..RULE_HEADER_LEN].copy_from_slice(raw.as_bytes());
    }
}

impl RuleHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            RuleMessageBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(payload.len(), RULE_HEADER_LEN)
            })?;
        Ok(RuleHeader {
            family: raw.family.into(),
            dst_len: raw.dst_len,
            src_len: raw.src_len,
            tos: raw.tos,
            table: raw.table,
            action: raw.action.into(),
            flags: RuleFlags::from_bits_retain(raw.flags),
        })
    }
}

impl From<&RuleHeader> for RuleMessageBuffer {
    fn from(header: &RuleHeader) -> Self {
        Self {
            family: header.family.into(),
            dst_len: header.dst_len,
            src_len: header.src_len,
            tos: header.tos,
            table: header.table,
            reserve_1: 0,
            reserve_2: 0,
            action: header.action.into(),
            flags: header.flags.bits(),
        }
    }
}
