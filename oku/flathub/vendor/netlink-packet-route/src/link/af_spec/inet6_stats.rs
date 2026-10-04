// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{DecodeError, Emitable};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

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
pub struct Inet6StatsBuffer {
    num: i64,
    in_pkts: i64,
    in_octets: i64,
    in_delivers: i64,
    out_forw_datagrams: i64,
    out_requests: i64,
    out_octets: i64,
    in_hdr_errors: i64,
    in_too_big_errors: i64,
    in_no_routes: i64,
    in_addr_errors: i64,
    in_unknown_protos: i64,
    in_truncated_pkts: i64,
    in_discards: i64,
    out_discards: i64,
    out_no_routes: i64,
    reasm_timeout: i64,
    reasm_reqds: i64,
    reasm_oks: i64,
    reasm_fails: i64,
    frag_oks: i64,
    frag_fails: i64,
    frag_creates: i64,
    in_mcast_pkts: i64,
    out_mcast_pkts: i64,
    in_bcast_pkts: i64,
    out_bcast_pkts: i64,
    in_mcast_octets: i64,
    out_mcast_octets: i64,
    in_bcast_octets: i64,
    out_bcast_octets: i64,
    in_csum_errors: i64,
    in_no_ect_pkts: i64,
    in_ect1_pkts: i64,
    in_ect0_pkts: i64,
    in_ce_pkts: i64,
    reasm_overlaps: i64,
    out_pkts: i64,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
#[non_exhaustive]
pub struct Inet6Stats {
    pub num: i64,
    pub in_pkts: i64,
    pub in_octets: i64,
    pub in_delivers: i64,
    pub out_forw_datagrams: i64,
    pub out_requests: i64,
    pub out_octets: i64,
    pub in_hdr_errors: i64,
    pub in_too_big_errors: i64,
    pub in_no_routes: i64,
    pub in_addr_errors: i64,
    pub in_unknown_protos: i64,
    pub in_truncated_pkts: i64,
    pub in_discards: i64,
    pub out_discards: i64,
    pub out_no_routes: i64,
    pub reasm_timeout: i64,
    pub reasm_reqds: i64,
    pub reasm_oks: i64,
    pub reasm_fails: i64,
    pub frag_oks: i64,
    pub frag_fails: i64,
    pub frag_creates: i64,
    pub in_mcast_pkts: i64,
    pub out_mcast_pkts: i64,
    pub in_bcast_pkts: i64,
    pub out_bcast_pkts: i64,
    pub in_mcast_octets: i64,
    pub out_mcast_octets: i64,
    pub in_bcast_octets: i64,
    pub out_bcast_octets: i64,
    pub in_csum_errors: i64,
    pub in_no_ect_pkts: i64,
    pub in_ect1_pkts: i64,
    pub in_ect0_pkts: i64,
    pub in_ce_pkts: i64,
    pub reasm_overlaps: i64,
    pub out_pkts: i64,
}

impl Inet6Stats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            Inet6StatsBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<Inet6StatsBuffer>(),
                )
            })?;
        Ok(Self {
            num: raw.num,
            in_pkts: raw.in_pkts,
            in_octets: raw.in_octets,
            in_delivers: raw.in_delivers,
            out_forw_datagrams: raw.out_forw_datagrams,
            out_requests: raw.out_requests,
            out_octets: raw.out_octets,
            in_hdr_errors: raw.in_hdr_errors,
            in_too_big_errors: raw.in_too_big_errors,
            in_no_routes: raw.in_no_routes,
            in_addr_errors: raw.in_addr_errors,
            in_unknown_protos: raw.in_unknown_protos,
            in_truncated_pkts: raw.in_truncated_pkts,
            in_discards: raw.in_discards,
            out_discards: raw.out_discards,
            out_no_routes: raw.out_no_routes,
            reasm_timeout: raw.reasm_timeout,
            reasm_reqds: raw.reasm_reqds,
            reasm_oks: raw.reasm_oks,
            reasm_fails: raw.reasm_fails,
            frag_oks: raw.frag_oks,
            frag_fails: raw.frag_fails,
            frag_creates: raw.frag_creates,
            in_mcast_pkts: raw.in_mcast_pkts,
            out_mcast_pkts: raw.out_mcast_pkts,
            in_bcast_pkts: raw.in_bcast_pkts,
            out_bcast_pkts: raw.out_bcast_pkts,
            in_mcast_octets: raw.in_mcast_octets,
            out_mcast_octets: raw.out_mcast_octets,
            in_bcast_octets: raw.in_bcast_octets,
            out_bcast_octets: raw.out_bcast_octets,
            in_csum_errors: raw.in_csum_errors,
            in_no_ect_pkts: raw.in_no_ect_pkts,
            in_ect1_pkts: raw.in_ect1_pkts,
            in_ect0_pkts: raw.in_ect0_pkts,
            in_ce_pkts: raw.in_ce_pkts,
            reasm_overlaps: raw.reasm_overlaps,
            out_pkts: raw.out_pkts,
        })
    }
}

impl From<&Inet6Stats> for Inet6StatsBuffer {
    fn from(stats: &Inet6Stats) -> Self {
        Self {
            num: stats.num,
            in_pkts: stats.in_pkts,
            in_octets: stats.in_octets,
            in_delivers: stats.in_delivers,
            out_forw_datagrams: stats.out_forw_datagrams,
            out_requests: stats.out_requests,
            out_octets: stats.out_octets,
            in_hdr_errors: stats.in_hdr_errors,
            in_too_big_errors: stats.in_too_big_errors,
            in_no_routes: stats.in_no_routes,
            in_addr_errors: stats.in_addr_errors,
            in_unknown_protos: stats.in_unknown_protos,
            in_truncated_pkts: stats.in_truncated_pkts,
            in_discards: stats.in_discards,
            out_discards: stats.out_discards,
            out_no_routes: stats.out_no_routes,
            reasm_timeout: stats.reasm_timeout,
            reasm_reqds: stats.reasm_reqds,
            reasm_oks: stats.reasm_oks,
            reasm_fails: stats.reasm_fails,
            frag_oks: stats.frag_oks,
            frag_fails: stats.frag_fails,
            frag_creates: stats.frag_creates,
            in_mcast_pkts: stats.in_mcast_pkts,
            out_mcast_pkts: stats.out_mcast_pkts,
            in_bcast_pkts: stats.in_bcast_pkts,
            out_bcast_pkts: stats.out_bcast_pkts,
            in_mcast_octets: stats.in_mcast_octets,
            out_mcast_octets: stats.out_mcast_octets,
            in_bcast_octets: stats.in_bcast_octets,
            out_bcast_octets: stats.out_bcast_octets,
            in_csum_errors: stats.in_csum_errors,
            in_no_ect_pkts: stats.in_no_ect_pkts,
            in_ect1_pkts: stats.in_ect1_pkts,
            in_ect0_pkts: stats.in_ect0_pkts,
            in_ce_pkts: stats.in_ce_pkts,
            reasm_overlaps: stats.reasm_overlaps,
            out_pkts: stats.out_pkts,
        }
    }
}

impl Emitable for Inet6Stats {
    fn buffer_len(&self) -> usize {
        size_of::<Inet6StatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = Inet6StatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
