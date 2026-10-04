// SPDX-License-Identifier: MIT

use std::net::{IpAddr, Ipv4Addr};

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        link_flag::LinkFlags, AmtMode, InfoAmt, InfoData, InfoKind,
        LinkAttribute, LinkHeader, LinkInfo, LinkLayerType, LinkMessage,
    },
    AddressFamily,
};

// nlmon capture of :
// ip link add amt0 type amt mode gateway dev eth0 \
//      local 10.0.0.1 discovery 10.0.0.2
#[test]
fn test_parsing_link_amt_gateway() {
    #[rustfmt::skip]
    let raw = vec![
        // ifinfomsg (16 bytes)
        0x00, 0x00, 0x01, 0x00, // family=AF_UNSPEC, link_layer_type=Ether
        0x04, 0x00, 0x00, 0x00, // ifindex=4
        0x43, 0x10, 0x01, 0x00, // flags=UP|BROADCAST|RUNNING|MULTICAST|LOWER_UP
        0x00, 0x00, 0x00, 0x00, // change_mask=0
        // IFLA_LINK_INFO, len=72, type=18
        0x48, 0x00, 0x12, 0x00,
        // IFLA_INFO_KIND, len=8, type=1
        0x08, 0x00, 0x01, 0x00, 0x61, 0x6d, 0x74, 0x00,
        // IFLA_INFO_DATA, len=60, type=2
        0x3c, 0x00, 0x02, 0x00,
        // IFLA_AMT_MODE, len=8, type=1, value=0 (gateway)
        0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        // IFLA_AMT_RELAY_PORT, len=6, type=2, value=2268 big-endian (0x08dc)
        0x06, 0x00, 0x02, 0x00, 0x08, 0xdc, 0x00, 0x00,
        // IFLA_AMT_GATEWAY_PORT, len=6, type=3, value=2268 big-endian (0x08dc)
        0x06, 0x00, 0x03, 0x00, 0x08, 0xdc, 0x00, 0x00,
        // IFLA_AMT_LINK, len=8, type=4, value=3
        0x08, 0x00, 0x04, 0x00, 0x03, 0x00, 0x00, 0x00,
        // IFLA_AMT_LOCAL_IP, len=8, type=5, value=10.0.0.1
        0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0x01,
        // IFLA_AMT_DISCOVERY_IP, len=8, type=7, value=10.0.0.2
        0x08, 0x00, 0x07, 0x00, 0x0a, 0x00, 0x00, 0x02,
        // IFLA_AMT_MAX_TUNNELS, len=8, type=8, value=128
        0x08, 0x00, 0x08, 0x00, 0x80, 0x00, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 4,
            link_layer_type: LinkLayerType::Ether,
            flags: LinkFlags::Up
                | LinkFlags::Broadcast
                | LinkFlags::Running
                | LinkFlags::Multicast
                | LinkFlags::LowerUp,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::Amt),
            LinkInfo::Data(InfoData::Amt(vec![
                InfoAmt::Mode(AmtMode::Gateway),
                InfoAmt::RelayPort(2268),
                InfoAmt::GatewayPort(2268),
                InfoAmt::Link(3),
                InfoAmt::LocalIp(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
                InfoAmt::DiscoveryIp(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))),
                InfoAmt::MaxTunnels(128),
            ])),
        ])],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}

// Captured from a relay-mode AMT interface.
#[test]
fn test_parsing_link_amt_relay() {
    #[rustfmt::skip]
    let raw = vec![
        // ifinfomsg (16 bytes)
        0x00, 0x00, 0x01, 0x00,
        0x05, 0x00, 0x00, 0x00,
        0x43, 0x10, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00,
        // IFLA_LINK_INFO, len=48, type=18
        0x30, 0x00, 0x12, 0x00,
        // IFLA_INFO_KIND, len=8, type=1
        0x08, 0x00, 0x01, 0x00, 0x61, 0x6d, 0x74, 0x00,
        // IFLA_INFO_DATA, len=36, type=2
        0x24, 0x00, 0x02, 0x00,
        // IFLA_AMT_MODE, len=8, type=1, value=1 (relay)
        0x08, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00,
        // IFLA_AMT_RELAY_PORT, len=6, type=2, value=2268 big-endian (0x08dc)
        0x06, 0x00, 0x02, 0x00, 0x08, 0xdc, 0x00, 0x00,
        // IFLA_AMT_LINK, len=8, type=4, value=5
        0x08, 0x00, 0x04, 0x00, 0x05, 0x00, 0x00, 0x00,
        // IFLA_AMT_LOCAL_IP, len=8, type=5, value=10.0.0.2
        0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0x02,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 5,
            link_layer_type: LinkLayerType::Ether,
            flags: LinkFlags::Up
                | LinkFlags::Broadcast
                | LinkFlags::Running
                | LinkFlags::Multicast
                | LinkFlags::LowerUp,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::Amt),
            LinkInfo::Data(InfoData::Amt(vec![
                InfoAmt::Mode(AmtMode::Relay),
                InfoAmt::RelayPort(2268),
                InfoAmt::Link(5),
                InfoAmt::LocalIp(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))),
            ])),
        ])],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}

// Minimal AMT interface with only mode and link.
#[test]
fn test_parsing_link_amt_minimal() {
    #[rustfmt::skip]
    let raw = vec![
        // ifinfomsg (16 bytes)
        0x00, 0x00, 0x01, 0x00,
        0x07, 0x00, 0x00, 0x00,
        0x43, 0x10, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00,
        // IFLA_LINK_INFO, len=32, type=18
        0x20, 0x00, 0x12, 0x00,
        // IFLA_INFO_KIND, len=8, type=1
        0x08, 0x00, 0x01, 0x00, 0x61, 0x6d, 0x74, 0x00,
        // IFLA_INFO_DATA, len=20, type=2
        0x14, 0x00, 0x02, 0x00,
        // IFLA_AMT_MODE, len=8, type=1, value=0 (gateway)
        0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        // IFLA_AMT_LINK, len=8, type=4, value=7
        0x08, 0x00, 0x04, 0x00, 0x07, 0x00, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 7,
            link_layer_type: LinkLayerType::Ether,
            flags: LinkFlags::Up
                | LinkFlags::Broadcast
                | LinkFlags::Running
                | LinkFlags::Multicast
                | LinkFlags::LowerUp,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::Amt),
            LinkInfo::Data(InfoData::Amt(vec![
                InfoAmt::Mode(AmtMode::Gateway),
                InfoAmt::Link(7),
            ])),
        ])],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
