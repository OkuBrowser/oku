// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlasIterator,
    Parseable, NLA_F_NESTED,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

const AF_MPLS: u16 = 28;
const MPLS_STATS_LINK: u16 = 1;

/// Parsed `struct mpls_link_stats` (72 bytes).
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct MplsLinkStats {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
    pub rx_noroute: u64,
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
pub struct MplsLinkStatsBuffer {
    rx_packets: u64,
    tx_packets: u64,
    rx_bytes: u64,
    tx_bytes: u64,
    rx_errors: u64,
    tx_errors: u64,
    rx_dropped: u64,
    tx_dropped: u64,
    rx_noroute: u64,
}

impl MplsLinkStats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) =
            MplsLinkStatsBuffer::ref_from_prefix(payload).map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<MplsLinkStatsBuffer>(),
                )
            })?;
        Ok(Self {
            rx_packets: raw.rx_packets,
            tx_packets: raw.tx_packets,
            rx_bytes: raw.rx_bytes,
            tx_bytes: raw.tx_bytes,
            rx_errors: raw.rx_errors,
            tx_errors: raw.tx_errors,
            rx_dropped: raw.rx_dropped,
            tx_dropped: raw.tx_dropped,
            rx_noroute: raw.rx_noroute,
        })
    }
}

impl From<&MplsLinkStats> for MplsLinkStatsBuffer {
    fn from(value: &MplsLinkStats) -> Self {
        Self {
            rx_packets: value.rx_packets,
            tx_packets: value.tx_packets,
            rx_bytes: value.rx_bytes,
            tx_bytes: value.tx_bytes,
            rx_errors: value.rx_errors,
            tx_errors: value.tx_errors,
            rx_dropped: value.rx_dropped,
            tx_dropped: value.tx_dropped,
            rx_noroute: value.rx_noroute,
        }
    }
}

impl Nla for MplsLinkStats {
    fn value_len(&self) -> usize {
        size_of::<MplsLinkStatsBuffer>()
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        let raw = MplsLinkStatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }

    fn kind(&self) -> u16 {
        MPLS_STATS_LINK
    }
}

/// A parsed entry inside `IFLA_STATS_AF_SPEC`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum AfSpecStat {
    Mpls(MplsLinkStats),
    Other(DefaultNla),
}

impl Nla for AfSpecStat {
    fn value_len(&self) -> usize {
        match self {
            Self::Mpls(v) => v.buffer_len(),
            Self::Other(v) => v.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Mpls(v) => v.emit(buffer),
            Self::Other(v) => v.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Mpls(_) => AF_MPLS,
            Self::Other(v) => v.kind(),
        }
    }
}

/// Parsed content of `IFLA_STATS_AF_SPEC`.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct AfSpecStats(pub Vec<AfSpecStat>);

impl Emitable for AfSpecStats {
    fn buffer_len(&self) -> usize {
        self.0.as_slice().buffer_len()
    }

    fn emit(&self, buffer: &mut [u8]) {
        self.0.as_slice().emit(buffer);
    }
}

impl AfSpecStats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let mut entries = Vec::new();
        for nla in NlasIterator::new(payload) {
            let nla = nla.context("invalid NLA in AF_SPEC stats")?;
            let family = nla.kind() & !NLA_F_NESTED;
            if family == AF_MPLS {
                entries.push(AfSpecStat::Mpls(parse_mpls_stats(nla.value())?));
            } else {
                entries.push(AfSpecStat::Other(DefaultNla::parse(&nla)?));
            }
        }
        Ok(AfSpecStats(entries))
    }
}

fn parse_mpls_stats(payload: &[u8]) -> Result<MplsLinkStats, DecodeError> {
    for nla in NlasIterator::new(payload) {
        let nla = nla.context("invalid NLA in MPLS stats")?;
        let kind = nla.kind() & !NLA_F_NESTED;
        if kind == MPLS_STATS_LINK {
            return MplsLinkStats::parse(nla.value())
                .context("invalid MPLS link stats");
        }
    }
    Ok(MplsLinkStats::default())
}
