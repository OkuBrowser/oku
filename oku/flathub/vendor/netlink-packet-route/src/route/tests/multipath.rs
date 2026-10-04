// SPDX-License-Identifier: MIT

use std::net::Ipv4Addr;

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    route::{
        flags::RouteFlags, RouteAttribute, RouteHeader, RouteMessage,
        RouteNextHop, RouteNextHopFlags, RouteProtocol, RouteScope, RouteType,
    },
    AddressFamily,
};

// wireshark capture(netlink message header removed) of nlmon against command:
//   ip route add 10.109.0.0/16 nexthop via 10.0.0.254 dev test-dummy weight 1
//       nexthop via 10.0.0.253 dev test-dummy weight 2
#[test]
fn test_route_multipath_two_nexthops() {
    let raw = vec![
        // rtmsg: family=AF_INET(2), dst_len=16, src_len=0, tos=0,
        //        table=main(254), proto=boot(3), scope=global(0),
        //        type=unicast(1), flags=0
        0x02, 0x10, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        // RTA_TABLE(0x0f)=254
        0x08, 0x00, 0x0f, 0x00, 0xfe, 0x00, 0x00, 0x00,
        // RTA_DST(0x01)=10.109.0.0
        0x08, 0x00, 0x01, 0x00, 0x0a, 0x6d, 0x00, 0x00,
        // RTA_MULTIPATH(0x09) with 2 nexthops:
        //   nexthop 0: len=16, flags=0, hops=0(weight=1), ifindex=17
        //              RTA_GATEWAY=10.0.0.254
        //   nexthop 1: len=16, flags=0, hops=1(weight=2), ifindex=17
        //              RTA_GATEWAY=10.0.0.253
        0x24, 0x00, 0x09, 0x00, 0x10, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0xfe, 0x10, 0x00, 0x00, 0x01,
        0x11, 0x00, 0x00, 0x00, 0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0xfd,
    ];

    let expected = RouteMessage {
        header: RouteHeader {
            address_family: AddressFamily::Inet,
            destination_prefix_length: 16,
            source_prefix_length: 0,
            tos: 0,
            table: 254,
            protocol: RouteProtocol::Boot,
            scope: RouteScope::Universe,
            kind: RouteType::Unicast,
            flags: RouteFlags::empty(),
        },
        attributes: vec![
            RouteAttribute::Table(254),
            RouteAttribute::Destination(Ipv4Addr::new(10, 109, 0, 0).into()),
            RouteAttribute::MultiPath(vec![
                RouteNextHop {
                    flags: RouteNextHopFlags::empty(),
                    hops: 0,
                    interface_index: 17,
                    attributes: vec![RouteAttribute::Gateway(
                        Ipv4Addr::new(10, 0, 0, 254).into(),
                    )],
                },
                RouteNextHop {
                    flags: RouteNextHopFlags::empty(),
                    hops: 1,
                    interface_index: 17,
                    attributes: vec![RouteAttribute::Gateway(
                        Ipv4Addr::new(10, 0, 0, 253).into(),
                    )],
                },
            ]),
        ],
    };

    assert_eq!(expected, RouteMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}
