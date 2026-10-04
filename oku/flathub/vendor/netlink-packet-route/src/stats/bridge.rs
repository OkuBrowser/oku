// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlasIterator,
    Parseable, NLA_F_NESTED,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Parsed bridge xstat value.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum BridgeXstat {
    Mcast(BridgeMcastStats),
    Stp(BridgeStpXstats),
    Vlan(BridgeVlanXstats),
    Other(DefaultNla),
}

const BRIDGE_XSTATS_VLAN: u16 = 1;
const BRIDGE_XSTATS_MCAST: u16 = 2;
// const BRIDGE_XSTATS_PAD: u16 = 3;
const BRIDGE_XSTATS_STP: u16 = 4;

pub(crate) fn parse_bridge_xstats(
    payload: &[u8],
) -> Result<Vec<BridgeXstat>, DecodeError> {
    let mut result = Vec::new();
    for nla in NlasIterator::new(payload) {
        let nla = nla.context("invalid NLA in bridge xstats")?;
        let kind = nla.kind() & !NLA_F_NESTED;
        let val = nla.value();
        result.push(match kind {
            BRIDGE_XSTATS_MCAST => BridgeXstat::Mcast(
                BridgeMcastStats::parse(val)
                    .context("invalid bridge mcast stats")?,
            ),
            BRIDGE_XSTATS_STP => BridgeXstat::Stp(
                BridgeStpXstats::parse(val)
                    .context("invalid bridge stp stats")?,
            ),
            BRIDGE_XSTATS_VLAN => BridgeXstat::Vlan(
                BridgeVlanXstats::parse(val)
                    .context("invalid bridge vlan stats")?,
            ),
            _ => BridgeXstat::Other(DefaultNla::parse(&nla)?),
        });
    }
    Ok(result)
}

impl Nla for BridgeXstat {
    fn value_len(&self) -> usize {
        match self {
            Self::Mcast(v) => v.buffer_len(),
            Self::Stp(v) => v.buffer_len(),
            Self::Vlan(v) => v.buffer_len(),
            Self::Other(v) => v.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Mcast(v) => v.emit(buffer),
            Self::Stp(v) => v.emit(buffer),
            Self::Vlan(v) => v.emit(buffer),
            Self::Other(v) => v.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Mcast(_) => BRIDGE_XSTATS_MCAST,
            Self::Stp(_) => BRIDGE_XSTATS_STP,
            Self::Vlan(_) => BRIDGE_XSTATS_VLAN,
            Self::Other(v) => v.kind(),
        }
    }
}

/// Parsed `struct br_mcast_stats` (240 bytes).
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct BridgeMcastStats {
    pub igmp_v1queries_rx: u64,
    pub igmp_v1queries_tx: u64,
    pub igmp_v2queries_rx: u64,
    pub igmp_v2queries_tx: u64,
    pub igmp_v3queries_rx: u64,
    pub igmp_v3queries_tx: u64,
    pub igmp_leaves_rx: u64,
    pub igmp_leaves_tx: u64,
    pub igmp_v1reports_rx: u64,
    pub igmp_v1reports_tx: u64,
    pub igmp_v2reports_rx: u64,
    pub igmp_v2reports_tx: u64,
    pub igmp_v3reports_rx: u64,
    pub igmp_v3reports_tx: u64,
    pub igmp_parse_errors: u64,
    pub mld_v1queries_rx: u64,
    pub mld_v1queries_tx: u64,
    pub mld_v2queries_rx: u64,
    pub mld_v2queries_tx: u64,
    pub mld_leaves_rx: u64,
    pub mld_leaves_tx: u64,
    pub mld_v1reports_rx: u64,
    pub mld_v1reports_tx: u64,
    pub mld_v2reports_rx: u64,
    pub mld_v2reports_tx: u64,
    pub mld_parse_errors: u64,
    pub mcast_bytes_rx: u64,
    pub mcast_bytes_tx: u64,
    pub mcast_packets_rx: u64,
    pub mcast_packets_tx: u64,
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
pub struct BridgeMcastStatsBuffer {
    igmp_v1queries_rx: u64,
    igmp_v1queries_tx: u64,
    igmp_v2queries_rx: u64,
    igmp_v2queries_tx: u64,
    igmp_v3queries_rx: u64,
    igmp_v3queries_tx: u64,
    igmp_leaves_rx: u64,
    igmp_leaves_tx: u64,
    igmp_v1reports_rx: u64,
    igmp_v1reports_tx: u64,
    igmp_v2reports_rx: u64,
    igmp_v2reports_tx: u64,
    igmp_v3reports_rx: u64,
    igmp_v3reports_tx: u64,
    igmp_parse_errors: u64,
    mld_v1queries_rx: u64,
    mld_v1queries_tx: u64,
    mld_v2queries_rx: u64,
    mld_v2queries_tx: u64,
    mld_leaves_rx: u64,
    mld_leaves_tx: u64,
    mld_v1reports_rx: u64,
    mld_v1reports_tx: u64,
    mld_v2reports_rx: u64,
    mld_v2reports_tx: u64,
    mld_parse_errors: u64,
    mcast_bytes_rx: u64,
    mcast_bytes_tx: u64,
    mcast_packets_rx: u64,
    mcast_packets_tx: u64,
}

impl BridgeMcastStats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = BridgeMcastStatsBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<BridgeMcastStatsBuffer>(),
                )
            })?;
        Ok(Self {
            igmp_v1queries_rx: raw.igmp_v1queries_rx,
            igmp_v1queries_tx: raw.igmp_v1queries_tx,
            igmp_v2queries_rx: raw.igmp_v2queries_rx,
            igmp_v2queries_tx: raw.igmp_v2queries_tx,
            igmp_v3queries_rx: raw.igmp_v3queries_rx,
            igmp_v3queries_tx: raw.igmp_v3queries_tx,
            igmp_leaves_rx: raw.igmp_leaves_rx,
            igmp_leaves_tx: raw.igmp_leaves_tx,
            igmp_v1reports_rx: raw.igmp_v1reports_rx,
            igmp_v1reports_tx: raw.igmp_v1reports_tx,
            igmp_v2reports_rx: raw.igmp_v2reports_rx,
            igmp_v2reports_tx: raw.igmp_v2reports_tx,
            igmp_v3reports_rx: raw.igmp_v3reports_rx,
            igmp_v3reports_tx: raw.igmp_v3reports_tx,
            igmp_parse_errors: raw.igmp_parse_errors,
            mld_v1queries_rx: raw.mld_v1queries_rx,
            mld_v1queries_tx: raw.mld_v1queries_tx,
            mld_v2queries_rx: raw.mld_v2queries_rx,
            mld_v2queries_tx: raw.mld_v2queries_tx,
            mld_leaves_rx: raw.mld_leaves_rx,
            mld_leaves_tx: raw.mld_leaves_tx,
            mld_v1reports_rx: raw.mld_v1reports_rx,
            mld_v1reports_tx: raw.mld_v1reports_tx,
            mld_v2reports_rx: raw.mld_v2reports_rx,
            mld_v2reports_tx: raw.mld_v2reports_tx,
            mld_parse_errors: raw.mld_parse_errors,
            mcast_bytes_rx: raw.mcast_bytes_rx,
            mcast_bytes_tx: raw.mcast_bytes_tx,
            mcast_packets_rx: raw.mcast_packets_rx,
            mcast_packets_tx: raw.mcast_packets_tx,
        })
    }
}

impl From<&BridgeMcastStats> for BridgeMcastStatsBuffer {
    fn from(value: &BridgeMcastStats) -> Self {
        Self {
            igmp_v1queries_rx: value.igmp_v1queries_rx,
            igmp_v1queries_tx: value.igmp_v1queries_tx,
            igmp_v2queries_rx: value.igmp_v2queries_rx,
            igmp_v2queries_tx: value.igmp_v2queries_tx,
            igmp_v3queries_rx: value.igmp_v3queries_rx,
            igmp_v3queries_tx: value.igmp_v3queries_tx,
            igmp_leaves_rx: value.igmp_leaves_rx,
            igmp_leaves_tx: value.igmp_leaves_tx,
            igmp_v1reports_rx: value.igmp_v1reports_rx,
            igmp_v1reports_tx: value.igmp_v1reports_tx,
            igmp_v2reports_rx: value.igmp_v2reports_rx,
            igmp_v2reports_tx: value.igmp_v2reports_tx,
            igmp_v3reports_rx: value.igmp_v3reports_rx,
            igmp_v3reports_tx: value.igmp_v3reports_tx,
            igmp_parse_errors: value.igmp_parse_errors,
            mld_v1queries_rx: value.mld_v1queries_rx,
            mld_v1queries_tx: value.mld_v1queries_tx,
            mld_v2queries_rx: value.mld_v2queries_rx,
            mld_v2queries_tx: value.mld_v2queries_tx,
            mld_leaves_rx: value.mld_leaves_rx,
            mld_leaves_tx: value.mld_leaves_tx,
            mld_v1reports_rx: value.mld_v1reports_rx,
            mld_v1reports_tx: value.mld_v1reports_tx,
            mld_v2reports_rx: value.mld_v2reports_rx,
            mld_v2reports_tx: value.mld_v2reports_tx,
            mld_parse_errors: value.mld_parse_errors,
            mcast_bytes_rx: value.mcast_bytes_rx,
            mcast_bytes_tx: value.mcast_bytes_tx,
            mcast_packets_rx: value.mcast_packets_rx,
            mcast_packets_tx: value.mcast_packets_tx,
        }
    }
}

impl Emitable for BridgeMcastStats {
    fn buffer_len(&self) -> usize {
        size_of::<BridgeMcastStatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = BridgeMcastStatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}

/// Parsed `struct bridge_stp_xstats` (48 bytes).
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct BridgeStpXstats {
    pub transition_blk: u64,
    pub transition_fwd: u64,
    pub rx_bpdu: u64,
    pub tx_bpdu: u64,
    pub rx_tcn: u64,
    pub tx_tcn: u64,
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
pub struct BridgeStpXstatsBuffer {
    transition_blk: u64,
    transition_fwd: u64,
    rx_bpdu: u64,
    tx_bpdu: u64,
    rx_tcn: u64,
    tx_tcn: u64,
}

impl BridgeStpXstats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = BridgeStpXstatsBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<BridgeStpXstatsBuffer>(),
                )
            })?;
        Ok(Self {
            transition_blk: raw.transition_blk,
            transition_fwd: raw.transition_fwd,
            rx_bpdu: raw.rx_bpdu,
            tx_bpdu: raw.tx_bpdu,
            rx_tcn: raw.rx_tcn,
            tx_tcn: raw.tx_tcn,
        })
    }
}

impl From<&BridgeStpXstats> for BridgeStpXstatsBuffer {
    fn from(value: &BridgeStpXstats) -> Self {
        Self {
            transition_blk: value.transition_blk,
            transition_fwd: value.transition_fwd,
            rx_bpdu: value.rx_bpdu,
            tx_bpdu: value.tx_bpdu,
            rx_tcn: value.rx_tcn,
            tx_tcn: value.tx_tcn,
        }
    }
}

impl Emitable for BridgeStpXstats {
    fn buffer_len(&self) -> usize {
        size_of::<BridgeStpXstatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = BridgeStpXstatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}

/// Parsed `struct bridge_vlan_xstats` (40 bytes).
#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[non_exhaustive]
pub struct BridgeVlanXstats {
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub vid: u16,
    pub flags: u16,
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
pub struct BridgeVlanXstatsBuffer {
    rx_bytes: u64,
    rx_packets: u64,
    tx_bytes: u64,
    tx_packets: u64,
    vid: u16,
    flags: u16,
    pad2: u32,
}

impl BridgeVlanXstats {
    pub fn parse(payload: &[u8]) -> Result<Self, DecodeError> {
        let (raw, _) = BridgeVlanXstatsBuffer::ref_from_prefix(payload)
            .map_err(|_| {
                DecodeError::buffer_too_small(
                    payload.len(),
                    size_of::<BridgeVlanXstatsBuffer>(),
                )
            })?;
        Ok(Self {
            rx_bytes: raw.rx_bytes,
            rx_packets: raw.rx_packets,
            tx_bytes: raw.tx_bytes,
            tx_packets: raw.tx_packets,
            vid: raw.vid,
            flags: raw.flags,
        })
    }
}

impl From<&BridgeVlanXstats> for BridgeVlanXstatsBuffer {
    fn from(value: &BridgeVlanXstats) -> Self {
        Self {
            rx_bytes: value.rx_bytes,
            rx_packets: value.rx_packets,
            tx_bytes: value.tx_bytes,
            tx_packets: value.tx_packets,
            vid: value.vid,
            flags: value.flags,
            pad2: 0,
        }
    }
}

impl Emitable for BridgeVlanXstats {
    fn buffer_len(&self) -> usize {
        size_of::<BridgeVlanXstatsBuffer>()
    }

    fn emit(&self, buffer: &mut [u8]) {
        let raw = BridgeVlanXstatsBuffer::from(self);
        buffer.copy_from_slice(raw.as_bytes());
    }
}
