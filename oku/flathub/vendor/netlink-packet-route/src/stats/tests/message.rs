// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    Emitable, NetlinkHeader, NetlinkMessage, NetlinkPayload, Nla, Parseable,
};

use crate::{
    stats::{
        AfSpecStats, HwStatsInfo, OffloadXstat, StatsAttribute,
        StatsFilterMask, StatsHeader, StatsMessage,
    },
    AddressFamily, RouteNetlinkMessage,
};

// nlmon of kernel reply on command `ip stat show dev enp3s0u2u1u4`
#[test]
fn test_parsing_combined_stats() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00,
        0xcc, 0x00, 0x01, 0x00, 0x87, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xf3, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xaf, 0x6c, 0x42, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x0d, 0x68, 0x22, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x1c, 0x00, 0x04, 0x00, 0x18, 0x00, 0x02, 0x80, 0x14, 0x00, 0x03, 0x80,
        0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x02, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x05, 0x00,
    ];
    let expected = StatsMessage {
        header: StatsHeader {
            family: AddressFamily::Unspec,
            ifindex: 16,
            filter_mask: StatsFilterMask::Link64
                | StatsFilterMask::LinkXstats
                | StatsFilterMask::LinkXstatsPort
                | StatsFilterMask::LinkOffloadXstats
                | StatsFilterMask::AfSpec,
        },
        attributes: vec![
            StatsAttribute::Link64(crate::link::Stats64 {
                rx_packets: 5255,
                tx_packets: 4339,
                rx_bytes: 4353199,
                tx_bytes: 2254861,
                ..Default::default()
            }),
            StatsAttribute::LinkOffloadXstats(vec![OffloadXstat::HwStatsInfo(
                HwStatsInfo {
                    request: Some(0),
                    used: Some(0),
                },
            )]),
            StatsAttribute::AfSpec(AfSpecStats(vec![])),
        ],
    };

    assert_eq!(expected, StatsMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}

// Test RTM_NEWSTATS message-level parsing via RouteNetlinkMessage
#[test]
fn test_parsing_rtm_newstats_message() {
    // Build a minimal RTM_NEWSTATS payload: header + one AF_SPEC NLA
    let stats_msg = StatsMessage {
        header: StatsHeader {
            family: AddressFamily::Unspec,
            ifindex: 1,
            filter_mask: StatsFilterMask::AfSpec,
        },
        attributes: vec![StatsAttribute::AfSpec(AfSpecStats(vec![]))],
    };

    let mut header = NetlinkHeader::default();
    header.length = 16 + stats_msg.buffer_len() as u32;
    header.message_type = 92; // RTM_NEWSTATS

    let msg = NetlinkMessage::new(
        header,
        NetlinkPayload::from(RouteNetlinkMessage::NewStats(stats_msg)),
    );

    let mut buf = vec![0; msg.buffer_len()];
    msg.emit(&mut buf);

    let parsed = NetlinkMessage::deserialize(&buf).unwrap();
    assert_eq!(parsed, msg);

    match parsed.payload {
        NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewStats(_)) => {}
        _ => panic!("expected NewStats message"),
    }
}

// Test RTM_GETSTATS message-level parsing via RouteNetlinkMessage
#[test]
fn test_parsing_rtm_getstats_message() {
    let stats_msg = StatsMessage {
        header: StatsHeader {
            family: AddressFamily::Unspec,
            ifindex: 2,
            filter_mask: StatsFilterMask::Link64,
        },
        attributes: vec![],
    };

    let mut header = NetlinkHeader::default();
    header.length = 16 + stats_msg.buffer_len() as u32;
    header.message_type = 94; // RTM_GETSTATS

    let msg = NetlinkMessage::new(
        header,
        NetlinkPayload::from(RouteNetlinkMessage::GetStats(stats_msg)),
    );

    let mut buf = vec![0; msg.buffer_len()];
    msg.emit(&mut buf);

    let parsed = NetlinkMessage::deserialize(&buf).unwrap();
    assert_eq!(parsed, msg);

    match parsed.payload {
        NetlinkPayload::InnerMessage(RouteNetlinkMessage::GetStats(_)) => {}
        _ => panic!("expected GetStats message"),
    }
}

// Test that unknown NLA types are preserved as Other(DefaultNla)
#[test]
fn test_parsing_unknown_stats_attribute() {
    let raw: Vec<u8> = {
        let mut v = vec![
            0x00, 0x00, 0x00, 0x00, // family=0, pad=0
            0x01, 0x00, 0x00, 0x00, // ifindex=1
            0x00, 0x00, 0x00, 0x00, // filter_mask=0
        ];
        // Unknown NLA type=99, len=8 (hdr=4 + 4 bytes payload)
        v.extend_from_slice(&8u16.to_ne_bytes());
        v.extend_from_slice(&99u16.to_ne_bytes());
        v.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        v
    };

    let parsed = StatsMessage::parse(&raw).unwrap();
    assert_eq!(parsed.attributes.len(), 1);
    match &parsed.attributes[0] {
        StatsAttribute::Other(nla) => {
            assert_eq!(nla.kind(), 99);
            let mut val = vec![0u8; nla.value_len()];
            nla.emit_value(&mut val);
            assert_eq!(val, &[0xde, 0xad, 0xbe, 0xef]);
        }
        other => panic!("expected Other, got {other:?}"),
    }

    // Roundtrip
    let mut buf = vec![0; parsed.buffer_len()];
    parsed.emit(&mut buf);
    assert_eq!(buf, raw);
}
