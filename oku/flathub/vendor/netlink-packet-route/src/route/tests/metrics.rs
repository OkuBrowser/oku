// SPDX-License-Identifier: MIT

use std::net::Ipv4Addr;

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    route::{
        flags::RouteFlags, RouteAttribute, RouteHeader, RouteMessage,
        RouteMetric, RouteProtocol, RouteScope, RouteType,
    },
    AddressFamily,
};

// wireshark capture(netlink message header removed) of nlmon against command:
//   ip route add 10.103.0.0/16 via 10.0.0.254 dev test-dummy mtu 1500
//       window 65535 rtt 100ms hoplimit 64
#[test]
fn test_route_metrics_mtu_window_rtt_hoplimit() {
    let raw = vec![
        // rtmsg: family=AF_INET(2), dst_len=16, src_len=0, tos=0,
        //        table=main(254), proto=boot(3), scope=global(0),
        //        type=unicast(1), flags=0
        0x02, 0x10, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        // RTA_TABLE(0x0f)=254
        0x08, 0x00, 0x0f, 0x00, 0xfe, 0x00, 0x00, 0x00,
        // RTA_DST(0x01)=10.103.0.0
        0x08, 0x00, 0x01, 0x00, 0x0a, 0x67, 0x00, 0x00,
        // RTA_METRICS(0x08) containing:
        //   RTAX_MTU(2)=1500(0x05dc), RTAX_WINDOW(3)=65535(0xffff),
        //   RTAX_RTT(4)=800(100ms*8=0x0320), RTAX_HOPLIMIT(10)=64(0x40)
        0x24, 0x00, 0x08, 0x00, 0x08, 0x00, 0x02, 0x00, 0xdc, 0x05, 0x00, 0x00,
        0x08, 0x00, 0x03, 0x00, 0xff, 0xff, 0x00, 0x00, 0x08, 0x00, 0x04, 0x00,
        0x20, 0x03, 0x00, 0x00, 0x08, 0x00, 0x0a, 0x00, 0x40, 0x00, 0x00, 0x00,
        // RTA_GATEWAY(0x05)=10.0.0.254
        0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0xfe,
        // RTA_OIF(0x04)=17
        0x08, 0x00, 0x04, 0x00, 0x11, 0x00, 0x00, 0x00,
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
            RouteAttribute::Destination(Ipv4Addr::new(10, 103, 0, 0).into()),
            RouteAttribute::Metrics(vec![
                RouteMetric::Mtu(1500),
                RouteMetric::Window(65535),
                RouteMetric::Rtt(800),
                RouteMetric::Hoplimit(64),
            ]),
            RouteAttribute::Gateway(Ipv4Addr::new(10, 0, 0, 254).into()),
            RouteAttribute::Oif(17),
        ],
    };

    assert_eq!(expected, RouteMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}

// wireshark capture of nlmon against command:
//   ip route add 10.106.0.0/16 via 10.0.0.254 dev test-dummy
//       advmss 1460 ssthresh 100 cwnd 100 initcwnd 10 initrwnd 10
#[test]
fn test_route_metrics_advmss_ssthresh_cwnd_initcwnd_initrwnd() {
    let raw = vec![
        // rtmsg: family=AF_INET(2), dst_len=16, src_len=0, tos=0,
        //        table=main(254), proto=boot(3), scope=global(0),
        //        type=unicast(1), flags=0
        0x02, 0x10, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        // RTA_TABLE(0x0f)=254
        0x08, 0x00, 0x0f, 0x00, 0xfe, 0x00, 0x00, 0x00,
        // RTA_DST(0x01)=10.106.0.0
        0x08, 0x00, 0x01, 0x00, 0x0a, 0x6a, 0x00, 0x00,
        // RTA_METRICS(0x08):
        //   RTAX_SSTHRESH(6)=100, RTAX_CWND(7)=100, RTAX_ADVMSS(8)=1460,
        //   RTAX_INITCWND(11)=10, RTAX_INITRWND(14)=10
        0x2c, 0x00, 0x08, 0x00, 0x08, 0x00, 0x06, 0x00, 0x64, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x07, 0x00, 0x64, 0x00, 0x00, 0x00, 0x08, 0x00, 0x08, 0x00,
        0xb4, 0x05, 0x00, 0x00, 0x08, 0x00, 0x0b, 0x00, 0x0a, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x0e, 0x00, 0x0a, 0x00, 0x00, 0x00,
        // RTA_GATEWAY(0x05)=10.0.0.254
        0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0xfe,
        // RTA_OIF(0x04)=17
        0x08, 0x00, 0x04, 0x00, 0x11, 0x00, 0x00, 0x00,
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
            RouteAttribute::Destination(Ipv4Addr::new(10, 106, 0, 0).into()),
            RouteAttribute::Metrics(vec![
                RouteMetric::SsThresh(100),
                RouteMetric::Cwnd(100),
                RouteMetric::Advmss(1460),
                RouteMetric::InitCwnd(10),
                RouteMetric::InitRwnd(10),
            ]),
            RouteAttribute::Gateway(Ipv4Addr::new(10, 0, 0, 254).into()),
            RouteAttribute::Oif(17),
        ],
    };

    assert_eq!(expected, RouteMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}

// wireshark capture of nlmon against command:
//   ip route add 10.107.0.0/16 via 10.0.0.254 dev test-dummy
//       reordering 5 rto_min 200ms quickack 1 fastopen_no_cookie 1
#[test]
fn test_route_metrics_reordering_rto_min_quickack_fastopen() {
    let raw = vec![
        0x02, 0x10, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        // RTA_TABLE(0x0f)=254
        0x08, 0x00, 0x0f, 0x00, 0xfe, 0x00, 0x00, 0x00,
        // RTA_DST(0x01)=10.107.0.0
        0x08, 0x00, 0x01, 0x00, 0x0a, 0x6b, 0x00, 0x00,
        // RTA_METRICS(0x08):
        //   RTAX_LOCK(1)=0x2000 (RTAX_RTO_MIN bit), RTAX_REORDERING(9)=5,
        //   RTAX_RTO_MIN(13)=200(0xc8), RTAX_QUICKACK(15)=1,
        //   RTAX_FASTOPEN_NO_COOKIE(17)=1
        0x2c, 0x00, 0x08, 0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x20, 0x00, 0x00,
        0x08, 0x00, 0x09, 0x00, 0x05, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0d, 0x00,
        0xc8, 0x00, 0x00, 0x00, 0x08, 0x00, 0x0f, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x11, 0x00, 0x01, 0x00, 0x00, 0x00,
        // RTA_GATEWAY(0x05)=10.0.0.254
        0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0xfe,
        // RTA_OIF(0x04)=17
        0x08, 0x00, 0x04, 0x00, 0x11, 0x00, 0x00, 0x00,
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
            RouteAttribute::Destination(Ipv4Addr::new(10, 107, 0, 0).into()),
            RouteAttribute::Metrics(vec![
                RouteMetric::Lock(0x2000),
                RouteMetric::Reordering(5),
                RouteMetric::RtoMin(200),
                RouteMetric::QuickAck(1),
                RouteMetric::FastopenNoCookie(1),
            ]),
            RouteAttribute::Gateway(Ipv4Addr::new(10, 0, 0, 254).into()),
            RouteAttribute::Oif(17),
        ],
    };

    assert_eq!(expected, RouteMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}

// wireshark capture of nlmon against command:
//   ip route add 10.108.0.0/16 via 10.0.0.254 dev test-dummy features ecn
#[test]
fn test_route_metrics_features_ecn() {
    let raw = vec![
        0x02, 0x10, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        // RTA_TABLE(0x0f)=254
        0x08, 0x00, 0x0f, 0x00, 0xfe, 0x00, 0x00, 0x00,
        // RTA_DST(0x01)=10.108.0.0
        0x08, 0x00, 0x01, 0x00, 0x0a, 0x6c, 0x00, 0x00,
        // RTA_METRICS(0x08): RTAX_FEATURES(12)=1 (ecn bit)
        0x0c, 0x00, 0x08, 0x00, 0x08, 0x00, 0x0c, 0x00, 0x01, 0x00, 0x00, 0x00,
        // RTA_GATEWAY(0x05)=10.0.0.254
        0x08, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x00, 0xfe,
        // RTA_OIF(0x04)=17
        0x08, 0x00, 0x04, 0x00, 0x11, 0x00, 0x00, 0x00,
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
            RouteAttribute::Destination(Ipv4Addr::new(10, 108, 0, 0).into()),
            RouteAttribute::Metrics(vec![RouteMetric::Features(1)]),
            RouteAttribute::Gateway(Ipv4Addr::new(10, 0, 0, 254).into()),
            RouteAttribute::Oif(17),
        ],
    };

    assert_eq!(expected, RouteMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}
