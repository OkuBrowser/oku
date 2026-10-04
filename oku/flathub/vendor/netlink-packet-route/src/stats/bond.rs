// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u64, parse_u64, DecodeError, DefaultNla, Emitable, ErrorContext, Nla,
    NlaBuffer, NlasIterator, Parseable, NLA_F_NESTED,
};

const BOND_XSTATS_3AD: u16 = 1;

/// Parsed bond xstat value.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum BondXstat {
    ThreeAd(Vec<Bond3adStats>),
    Other(DefaultNla),
}

impl Nla for BondXstat {
    fn value_len(&self) -> usize {
        match self {
            Self::ThreeAd(v) => v.as_slice().buffer_len(),
            Self::Other(v) => v.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::ThreeAd(v) => v.as_slice().emit(buffer),
            Self::Other(v) => v.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::ThreeAd(_) => BOND_XSTATS_3AD | NLA_F_NESTED,
            Self::Other(v) => v.kind(),
        }
    }
}

const BOND_3AD_STAT_LACPDU_RX: u16 = 0;
const BOND_3AD_STAT_LACPDU_TX: u16 = 1;
const BOND_3AD_STAT_LACPDU_UNKNOWN_RX: u16 = 2;
const BOND_3AD_STAT_LACPDU_ILLEGAL_RX: u16 = 3;
const BOND_3AD_STAT_MARKER_RX: u16 = 4;
const BOND_3AD_STAT_MARKER_TX: u16 = 5;
const BOND_3AD_STAT_MARKER_RESP_RX: u16 = 6;
const BOND_3AD_STAT_MARKER_RESP_TX: u16 = 7;
const BOND_3AD_STAT_MARKER_UNKNOWN_RX: u16 = 8;

/// Matching kernel `struct bond_3ad_stats`
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum Bond3adStats {
    LacpduRx(u64),
    LacpduTx(u64),
    LacpduUnknownRx(u64),
    LacpduIllegalRx(u64),
    MarkerRx(u64),
    MarkerTx(u64),
    MarkerRespRx(u64),
    MarkerRespTx(u64),
    MarkerUnknownRx(u64),
    Other(DefaultNla),
}

impl Nla for Bond3adStats {
    fn value_len(&self) -> usize {
        match self {
            Self::Other(v) => v.value_len(),
            _ => 8,
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::LacpduRx(d)
            | Self::LacpduTx(d)
            | Self::LacpduUnknownRx(d)
            | Self::LacpduIllegalRx(d)
            | Self::MarkerRx(d)
            | Self::MarkerTx(d)
            | Self::MarkerRespRx(d)
            | Self::MarkerRespTx(d)
            | Self::MarkerUnknownRx(d) => emit_u64(buffer, *d).unwrap(),
            Self::Other(v) => v.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::LacpduRx(_) => BOND_3AD_STAT_LACPDU_RX,
            Self::LacpduTx(_) => BOND_3AD_STAT_LACPDU_TX,
            Self::LacpduUnknownRx(_) => BOND_3AD_STAT_LACPDU_UNKNOWN_RX,
            Self::LacpduIllegalRx(_) => BOND_3AD_STAT_LACPDU_ILLEGAL_RX,
            Self::MarkerRx(_) => BOND_3AD_STAT_MARKER_RX,
            Self::MarkerTx(_) => BOND_3AD_STAT_MARKER_TX,
            Self::MarkerRespRx(_) => BOND_3AD_STAT_MARKER_RESP_RX,
            Self::MarkerRespTx(_) => BOND_3AD_STAT_MARKER_RESP_TX,
            Self::MarkerUnknownRx(_) => BOND_3AD_STAT_MARKER_UNKNOWN_RX,
            Self::Other(v) => v.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for Bond3adStats {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() & !NLA_F_NESTED {
            BOND_3AD_STAT_LACPDU_RX => Self::LacpduRx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_LACPDU_RX value")?,
            ),
            BOND_3AD_STAT_LACPDU_TX => Self::LacpduTx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_LACPDU_TX value")?,
            ),
            BOND_3AD_STAT_LACPDU_UNKNOWN_RX => Self::LacpduUnknownRx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_LACPDU_UNKNOWN_RX value")?,
            ),
            BOND_3AD_STAT_LACPDU_ILLEGAL_RX => Self::LacpduIllegalRx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_LACPDU_ILLEGAL_RX value")?,
            ),
            BOND_3AD_STAT_MARKER_RX => Self::MarkerRx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_MARKER_RX value")?,
            ),
            BOND_3AD_STAT_MARKER_TX => Self::MarkerTx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_MARKER_TX value")?,
            ),
            BOND_3AD_STAT_MARKER_RESP_RX => Self::MarkerRespRx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_MARKER_RESP_RX value")?,
            ),
            BOND_3AD_STAT_MARKER_RESP_TX => Self::MarkerRespTx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_MARKER_RESP_TX value")?,
            ),
            BOND_3AD_STAT_MARKER_UNKNOWN_RX => Self::MarkerUnknownRx(
                parse_u64(payload)
                    .context("invalid BOND_3AD_STAT_MARKER_UNKNOWN_RX value")?,
            ),
            _ => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {}", buf.kind()))?,
            ),
        })
    }
}

pub(crate) fn parse_bond_xstats(
    payload: &[u8],
) -> Result<Vec<BondXstat>, DecodeError> {
    let mut result = Vec::new();
    for nla in NlasIterator::new(payload) {
        let nla = nla.context("invalid NLA in bond xstats")?;
        let kind = nla.kind() & !NLA_F_NESTED;
        let payload = nla.value();
        result.push(match kind {
            BOND_XSTATS_3AD => {
                let mut stats = Vec::new();
                let err = "failed to parse BOND_XSTATS_3AD ";
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(err)?;
                    let parsed = Bond3adStats::parse(nla).context(err)?;
                    stats.push(parsed);
                }
                BondXstat::ThreeAd(stats)
            }
            _ => BondXstat::Other(DefaultNla::parse(&nla)?),
        });
    }
    Ok(result)
}
