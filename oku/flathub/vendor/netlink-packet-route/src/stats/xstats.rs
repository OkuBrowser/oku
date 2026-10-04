// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, DefaultNla, Emitable, ErrorContext, Nla, NlaBuffer,
    NlasIterator, Parseable, NLA_F_NESTED,
};

use super::{
    bond::{parse_bond_xstats, BondXstat},
    bridge::{parse_bridge_xstats, BridgeXstat},
};

const LINK_XSTATS_TYPE_BRIDGE: u16 = 1;
const LINK_XSTATS_TYPE_BOND: u16 = 2;

// ---------------------------------------------------------------------------
// Link xstats (inside IFLA_STATS_LINK_XSTATS / _PORT)
// ---------------------------------------------------------------------------

/// A fully parsed entry inside `IFLA_STATS_LINK_XSTATS` or
/// `IFLA_STATS_LINK_XSTATS_PORT`. The inner data is parsed at
/// construction time.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum LinkXstatGroup {
    Bridge(Vec<BridgeXstat>),
    Bond(Vec<BondXstat>),
    Other(DefaultNla),
}

impl Nla for LinkXstatGroup {
    fn value_len(&self) -> usize {
        match self {
            Self::Bridge(v) => v.as_slice().buffer_len(),
            Self::Bond(v) => v.as_slice().buffer_len(),
            Self::Other(v) => v.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::Bridge(v) => v.as_slice().emit(buffer),
            Self::Bond(v) => v.as_slice().emit(buffer),
            Self::Other(v) => v.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::Bridge(_) => LINK_XSTATS_TYPE_BRIDGE,
            Self::Bond(_) => LINK_XSTATS_TYPE_BOND,
            Self::Other(v) => v.kind(),
        }
    }
}

pub(crate) struct VecLinkXstats(pub(crate) Vec<LinkXstatGroup>);

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for VecLinkXstats
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let mut groups = Vec::new();
        for nla in NlasIterator::new(buf.into_inner()) {
            let nla = nla?;
            let kind = nla.kind() & !NLA_F_NESTED;
            let val = nla.value();
            groups.push(match kind {
                LINK_XSTATS_TYPE_BRIDGE => LinkXstatGroup::Bridge(
                    parse_bridge_xstats(val)
                        .context("invalid bridge xstats in link xstats")?,
                ),
                LINK_XSTATS_TYPE_BOND => LinkXstatGroup::Bond(
                    parse_bond_xstats(val)
                        .context("invalid bond xstats in link xstats")?,
                ),
                _ => LinkXstatGroup::Other(DefaultNla::parse(&nla)?),
            });
        }
        Ok(Self(groups))
    }
}
