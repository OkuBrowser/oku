// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    stats::{
        HwStats64, HwStatsInfo, OffloadXstat, StatsAttribute, StatsFilterMask,
        StatsHeader, StatsMessage,
    },
    AddressFamily,
};

// Synthetic data matching kernel layout: RTM_NEWSTATS for lo
// (ifindex=1, filter_mask=0x08) with IFLA_STATS_LINK_OFFLOAD_XSTATS
// containing HW_S_INFO. TODO: replace with real nlmon capture.
#[test]
fn test_parsing_offload_xstats() {
    let raw = vec![
        0x00, 0x00, 0x00, 0x00, // family=0, pad=0
        0x01, 0x00, 0x00, 0x00, // ifindex=1
        0x08, 0x00, 0x00, 0x00, // filter_mask=0x08
        // NLA: IFLA_STATS_LINK_OFFLOAD_XSTATS(type=4), len=28
        0x1c, 0x00, 0x04, 0x00,
        // Inner: HW_S_INFO(type=2|NLA_F_NESTED), len=24
        0x18, 0x00, 0x02, 0x80,
        // Inner: L3_STATS(type=3|NLA_F_NESTED), len=20
        0x14, 0x00, 0x03, 0x80,
        // NLA: request(type=1), len=5, val=0 + 3 pad
        0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        // NLA: used(type=2), len=5, val=0 + 3 pad
        0x05, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let expected = StatsMessage {
        header: StatsHeader {
            family: AddressFamily::Unspec,
            ifindex: 1,
            filter_mask: StatsFilterMask::LinkOffloadXstats,
        },
        attributes: vec![StatsAttribute::LinkOffloadXstats(vec![
            OffloadXstat::HwStatsInfo(HwStatsInfo {
                request: Some(0),
                used: Some(0),
            }),
        ])],
    };

    assert_eq!(expected, StatsMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}

// Synthetic data matching kernel layout: RTM_NEWSTATS
// (ifindex=1, filter_mask=0x08) with IFLA_STATS_LINK_OFFLOAD_XSTATS
// containing L3_STATS (struct rtnl_hw_stats64, 72 bytes).
// TODO: replace with real nlmon capture.
#[test]
fn test_parsing_offload_l3_stats() {
    let raw: Vec<u8> = {
        let mut v = vec![
            0x00, 0x00, 0x00, 0x00, // family=0, pad=0
            0x01, 0x00, 0x00, 0x00, // ifindex=1
            0x08, 0x00, 0x00, 0x00, // filter_mask=0x08
        ];
        // NLA: IFLA_STATS_LINK_OFFLOAD_XSTATS(type=4), len=80
        // Inner: L3_STATS(type=3), len=76 (hdr=4 + 72 payload)
        let l3_nla_len: u16 = 76;
        let outer_nla_len: u16 = 80;
        v.extend_from_slice(&outer_nla_len.to_ne_bytes());
        v.extend_from_slice(&4u16.to_ne_bytes()); // type=4
        v.extend_from_slice(&l3_nla_len.to_ne_bytes());
        v.extend_from_slice(&3u16.to_ne_bytes()); // type=3
                                                  // struct rtnl_hw_stats64 (72 bytes)
                                                  // rx_packets=100, tx_packets=200, rest zero
        v.extend_from_slice(&100u64.to_ne_bytes());
        v.extend_from_slice(&200u64.to_ne_bytes());
        v.extend_from_slice(&[0u8; 56]); // remaining 7 fields
        v
    };

    let expected = StatsMessage {
        header: StatsHeader {
            family: AddressFamily::Unspec,
            ifindex: 1,
            filter_mask: StatsFilterMask::LinkOffloadXstats,
        },
        attributes: vec![StatsAttribute::LinkOffloadXstats(vec![
            OffloadXstat::L3Stats(HwStats64 {
                rx_packets: 100,
                tx_packets: 200,
                ..Default::default()
            }),
        ])],
    };

    assert_eq!(expected, StatsMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}
