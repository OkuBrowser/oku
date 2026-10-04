// SPDX-License-Identifier: MIT

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    route::{
        lwtunnel::RouteIp6TunnelFlags, RouteAttribute, RouteFlags, RouteHeader,
        RouteIp6Tunnel, RouteLwEnCapType, RouteLwTunnelEncap, RouteMessage,
        RouteMessageBuffer, RouteProtocol, RouteScope, RouteType,
    },
    AddressFamily,
};

// Setup:
//      ip link add dummy1 type dummy
//      ip link set dummy1 up
//      ip route add 192.0.2.0/24 encap ip6 \
//          dst 2001:db8:1::1 src 2001:db8:1::2 \
//          id 100 tc 7 hoplimit 253 csum dev dummy1
// wireshark capture(netlink message header removed) of nlmon against command:
//      ip route show dev dummy1
#[test]
fn test_ip6_tunnel() {
    let raw = vec![
        0x02, 0x18, 0x00, 0x00, 0xfe, 0x03, 0xfd, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x0f, 0x00, 0xfe, 0x00, 0x00, 0x00, 0x08, 0x00, 0x01, 0x00,
        0xc0, 0x00, 0x02, 0x00, 0x08, 0x00, 0x04, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x50, 0x00, 0x16, 0x00, 0x0c, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x64, 0x14, 0x00, 0x02, 0x00, 0x20, 0x01, 0x0d, 0xb8,
        0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x14, 0x00, 0x03, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x05, 0x00,
        0x07, 0x00, 0x00, 0x00, 0x05, 0x00, 0x04, 0x00, 0xfd, 0x00, 0x00, 0x00,
        0x06, 0x00, 0x06, 0x00, 0x00, 0x01, 0x00, 0x00, 0x06, 0x00, 0x15, 0x00,
        0x04, 0x00, 0x00, 0x00,
    ];

    let expected = RouteMessage {
        header: RouteHeader {
            address_family: AddressFamily::Inet,
            destination_prefix_length: 24,
            source_prefix_length: 0,
            tos: 0,
            table: 254,
            protocol: RouteProtocol::Boot,
            scope: RouteScope::Link,
            kind: RouteType::Unicast,
            flags: RouteFlags::empty(),
        },
        attributes: vec![
            RouteAttribute::Table(254),
            RouteAttribute::Destination(
                Ipv4Addr::from_str("192.0.2.0").unwrap().into(),
            ),
            RouteAttribute::Oif(8),
            RouteAttribute::Encap(vec![
                RouteLwTunnelEncap::Ip6(RouteIp6Tunnel::Id(100)),
                RouteLwTunnelEncap::Ip6(RouteIp6Tunnel::Destination(
                    Ipv6Addr::from_str("2001:db8:1::1").unwrap(),
                )),
                RouteLwTunnelEncap::Ip6(RouteIp6Tunnel::Source(
                    Ipv6Addr::from_str("2001:db8:1::2").unwrap(),
                )),
                RouteLwTunnelEncap::Ip6(RouteIp6Tunnel::Tc(7)),
                RouteLwTunnelEncap::Ip6(RouteIp6Tunnel::Hoplimit(253)),
                RouteLwTunnelEncap::Ip6(RouteIp6Tunnel::Flags(
                    RouteIp6TunnelFlags::Checksum,
                )),
            ]),
            RouteAttribute::EncapType(RouteLwEnCapType::Ip6),
        ],
    };

    assert_eq!(
        expected,
        RouteMessage::parse(&RouteMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
