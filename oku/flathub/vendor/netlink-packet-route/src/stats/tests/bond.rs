// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    stats::{
        Bond3adStats, BondXstat, LinkXstatGroup, StatsAttribute,
        StatsFilterMask, StatsHeader, StatsMessage,
    },
    AddressFamily,
};

// Synthetic data matching kernel layout: RTM_NEWSTATS for bond0
// (ifindex=5, filter_mask=0x06) with IFLA_STATS_LINK_XSTATS containing
// bond 3AD stats. TODO: replace with real nlmon capture via
// `ip link add bond0 type bond mode 802.3ad && ip stats show dev bond0`.
#[test]
fn test_parsing_bond_xstats() {
    let raw: Vec<u8> = {
        let mut v = vec![
            0x00, 0x00, 0x00, 0x00, // family=0, pad=0
            0x05, 0x00, 0x00, 0x00, // ifindex=5
            0x06, 0x00, 0x00, 0x00, // filter_mask=0x06
        ];
        // NLA: IFLA_STATS_LINK_XSTATS(type=2)
        // Inner: LINK_XSTATS_TYPE_BOND(type=2)
        // Inner: BOND_XSTATS_3AD(type=1|NLA_F_NESTED)
        // Inner: 3 x u64 stat NLAs (lacpdu_rx=10, lacpdu_tx=20, marker_rx=5)
        //
        // Each inner stat NLA: len=12 (hdr=4 + u64=8)
        // BOND_XSTATS_3AD nest: 3*12 = 36 payload + 4 hdr = 40
        // LINK_XSTATS_TYPE_BOND: 40 payload + 4 hdr = 44
        // IFLA_STATS_LINK_XSTATS: 44 payload + 4 hdr = 48
        let bond_3ad_len: u16 = 40; // 4 hdr + 3*12
        let bond_type_len: u16 = 44; // 4 hdr + 40
        let link_xstats_len: u16 = 48; // 4 hdr + 44

        // IFLA_STATS_LINK_XSTATS header
        v.extend_from_slice(&link_xstats_len.to_ne_bytes());
        v.extend_from_slice(&2u16.to_ne_bytes()); // type=2

        // LINK_XSTATS_TYPE_BOND header
        v.extend_from_slice(&bond_type_len.to_ne_bytes());
        v.extend_from_slice(&2u16.to_ne_bytes()); // type=2

        // BOND_XSTATS_3AD header (nested)
        v.extend_from_slice(&bond_3ad_len.to_ne_bytes());
        v.extend_from_slice(&(1u16 | 0x8000).to_ne_bytes()); // type=1|NESTED

        // BOND_3AD_STAT_LACPDU_RX (type=0), val=10
        v.extend_from_slice(&12u16.to_ne_bytes());
        v.extend_from_slice(&0u16.to_ne_bytes());
        v.extend_from_slice(&10u64.to_ne_bytes());

        // BOND_3AD_STAT_LACPDU_TX (type=1), val=20
        v.extend_from_slice(&12u16.to_ne_bytes());
        v.extend_from_slice(&1u16.to_ne_bytes());
        v.extend_from_slice(&20u64.to_ne_bytes());

        // BOND_3AD_STAT_MARKER_RX (type=4), val=5
        v.extend_from_slice(&12u16.to_ne_bytes());
        v.extend_from_slice(&4u16.to_ne_bytes());
        v.extend_from_slice(&5u64.to_ne_bytes());

        v
    };

    let expected = StatsMessage {
        header: StatsHeader {
            family: AddressFamily::Unspec,
            ifindex: 5,
            filter_mask: StatsFilterMask::LinkXstats
                | StatsFilterMask::LinkXstatsPort,
        },
        attributes: vec![StatsAttribute::LinkXstats(vec![
            LinkXstatGroup::Bond(vec![BondXstat::ThreeAd(vec![
                Bond3adStats::LacpduRx(10),
                Bond3adStats::LacpduTx(20),
                Bond3adStats::MarkerRx(5),
            ])]),
        ])],
    };

    assert_eq!(expected, StatsMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);

    // Non-zeroed buffer to verify emit zeros padding
    let mut buf = vec![255; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}
