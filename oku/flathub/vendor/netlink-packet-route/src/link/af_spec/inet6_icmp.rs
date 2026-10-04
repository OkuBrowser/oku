// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
#[non_exhaustive]
pub struct Icmp6Stats {
    pub num: i64,
    pub in_msgs: i64,
    pub in_errors: i64,
    pub out_msgs: i64,
    pub out_errors: i64,
    pub csum_errors: i64,
    pub rate_limit_host: i64,
}

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
pub struct Icmp6StatsBuffer {
    num: i64,
    in_msgs: i64,
    in_errors: i64,
    out_msgs: i64,
    out_errors: i64,
    csum_errors: i64,
    rate_limit_host: i64,
}

impl Icmp6Stats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            Icmp6StatsBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<Icmp6StatsBuffer>(),
                )
            })?;
        Ok(Self {
            num: raw.num,
            in_msgs: raw.in_msgs,
            in_errors: raw.in_errors,
            out_msgs: raw.out_msgs,
            out_errors: raw.out_errors,
            csum_errors: raw.csum_errors,
            rate_limit_host: raw.rate_limit_host,
        })
    }
}

impl From<&Icmp6Stats> for Icmp6StatsBuffer {
    fn from(stats: &Icmp6Stats) -> Self {
        Self {
            num: stats.num,
            in_msgs: stats.in_msgs,
            in_errors: stats.in_errors,
            out_msgs: stats.out_msgs,
            out_errors: stats.out_errors,
            csum_errors: stats.csum_errors,
            rate_limit_host: stats.rate_limit_host,
        }
    }
}

impl Emitable for Icmp6Stats {
    fn buffer_len(&self) -> usize {
        size_of::<Icmp6StatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = Icmp6StatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
