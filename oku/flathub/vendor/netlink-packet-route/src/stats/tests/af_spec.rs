// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    stats::{
        AfSpecStat, AfSpecStats, MplsLinkStats, StatsAttribute,
        StatsFilterMask, StatsHeader, StatsMessage,
    },
    AddressFamily,
};

// Synthetic data matching kernel layout: RTM_NEWSTATS for lo
// (ifindex=1, filter_mask=0x10) with IFLA_STATS_AF_SPEC containing
// AF_MPLS / MPLS_STATS_LINK. All counter values are zero.
// TODO: replace with real nlmon capture.
#[test]
fn test_parsing_af_spec_mpls() {
    let raw: Vec<u8> = {
        let mut v = vec![
            0x00, 0x00, 0x00, 0x00, // family=0, pad=0
            0x01, 0x00, 0x00, 0x00, // ifindex=1
            0x10, 0x00, 0x00, 0x00, // filter_mask=0x10
            // NLA: IFLA_STATS_AF_SPEC(type=5), len=84
            0x54, 0x00, 0x05, 0x00,
            // Inner: AF_MPLS(type=28), len=80
            0x50, 0x00, 0x1c, 0x00,
            // Inner: MPLS_STATS_LINK(type=1), len=76
            0x4c, 0x00, 0x01, 0x00,
        ];
        // struct mpls_link_stats (72 bytes, all zeros)
        v.extend_from_slice(&[0u8; 72]);
        v
    };

    let expected = StatsMessage {
        header: StatsHeader {
            family: AddressFamily::Unspec,
            ifindex: 1,
            filter_mask: StatsFilterMask::AfSpec,
        },
        attributes: vec![StatsAttribute::AfSpec(AfSpecStats(vec![
            AfSpecStat::Mpls(MplsLinkStats::default()),
        ]))],
    };

    assert_eq!(expected, StatsMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}
