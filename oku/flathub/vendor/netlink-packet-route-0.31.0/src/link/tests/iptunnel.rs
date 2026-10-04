// SPDX-License-Identifier: MIT

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        InfoData, InfoIpTunnel, InfoKind, Ip6TunnelFlags, LinkAttribute,
        LinkFlags, LinkHeader, LinkInfo, LinkLayerType, LinkMessage,
        LinkMessageBuffer, TunnelEncapFlags, TunnelEncapType,
    },
    AddressFamily, IpProtocol,
};

#[test]
fn test_iptunnel_ipip_link_info() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, // AF_UNSPEC and reserved
        0x00, 0x03, // Link Layer Type IPTUNNEL (768)
        0x06, 0x00, 0x00, 0x00, // iface ifindex 6
        0x90, 0x00, 0x00, 0x00, // flags
        0x00, 0x00, 0x00, 0x00, // changed flags
        0x74, 0x00, // length 74
        0x12, 0x00, // IFLA_LINK_INFO (18)
        0x09, 0x00, 0x01, 0x00, 0x69, 0x70, 0x69, 0x70, 0x00, 0x00, 0x00, 0x00,
        0x64, 0x00, 0x02, 0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x02, 0x00, 0xc0, 0xa8, 0x7a, 0xb7, 0x08, 0x00, 0x03, 0x00,
        0x0a, 0xff, 0xfe, 0x02, 0x05, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x05, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x09, 0x00,
        0x04, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x0f, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x11, 0x00, 0x00, 0x0a, 0x00, 0x00,
        0x06, 0x00, 0x12, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x06, 0x00, 0x10, 0x00,
        0x00, 0x00, 0x00, 0x00, // data
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 6,
            link_layer_type: LinkLayerType::Tunnel,
            flags: LinkFlags::Noarp | LinkFlags::Pointopoint,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::IpIp),
            LinkInfo::Data(InfoData::IpTunnel(vec![
                InfoIpTunnel::Link(0),
                InfoIpTunnel::Local(std::net::IpAddr::V4(
                    Ipv4Addr::from_str("192.168.122.183").unwrap(),
                )),
                InfoIpTunnel::Remote(std::net::IpAddr::V4(
                    Ipv4Addr::from_str("10.255.254.2").unwrap(),
                )),
                InfoIpTunnel::Ttl(0),
                InfoIpTunnel::Tos(0),
                InfoIpTunnel::Protocol(IpProtocol::Ipip),
                InfoIpTunnel::PMtuDisc(true),
                InfoIpTunnel::FwMark(0),
                InfoIpTunnel::EncapType(TunnelEncapType::None),
                InfoIpTunnel::EncapSPort(10),
                InfoIpTunnel::EncapDPort(12),
                InfoIpTunnel::EncapFlags(TunnelEncapFlags::from_bits_retain(0)),
            ])),
        ])],
    };

    assert_eq!(
        expected,
        LinkMessage::parse(&LinkMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}

#[test]
fn test_iptunnel_ipip6_link_info() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, // AF_UNSPEC and reserved
        0x01, 0x03, // Link Layer Type IP6TUNNEL (769)
        0x06, 0x00, 0x00, 0x00, // iface ifindex 6
        0x90, 0x00, 0x00, 0x00, // flags
        0x00, 0x00, 0x00, 0x00, // changed flags
        0x94, 0x00, // length 148
        0x12, 0x00, // IFLA_LINK_INFO (18)
        0x0b, 0x00, 0x01, 0x00, 0x69, 0x70, 0x36, 0x74, 0x6e, 0x6c, 0x00, 0x00,
        0x84, 0x00, 0x02, 0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x14, 0x00, 0x02, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x14, 0x00, 0x03, 0x00,
        0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x04, 0x00, 0x40, 0x00, 0x00, 0x00,
        0x05, 0x00, 0x06, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x07, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00,
        0x05, 0x00, 0x09, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x14, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x06, 0x00, 0x11, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x06, 0x00, 0x12, 0x00,
        0x00, 0x0c, 0x00, 0x00, 0x06, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00,
        0x00, //data
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 6,
            link_layer_type: LinkLayerType::Tunnel6,
            flags: LinkFlags::Noarp | LinkFlags::Pointopoint,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::Ip6Tnl),
            LinkInfo::Data(InfoData::IpTunnel(vec![
                InfoIpTunnel::Link(0),
                InfoIpTunnel::Local(std::net::IpAddr::V6(
                    Ipv6Addr::from_str("2001:db8:1::1").unwrap(),
                )),
                InfoIpTunnel::Remote(std::net::IpAddr::V6(
                    Ipv6Addr::from_str("2001:db8:1::2").unwrap(),
                )),
                InfoIpTunnel::Ttl(64),
                InfoIpTunnel::EncapLimit(4),
                InfoIpTunnel::FlowInfo(0),
                InfoIpTunnel::Ipv6Flags(
                    Ip6TunnelFlags::CapXmit | Ip6TunnelFlags::CapRcv,
                ),
                InfoIpTunnel::Protocol(IpProtocol::Ipip),
                InfoIpTunnel::FwMark(0),
                InfoIpTunnel::EncapType(TunnelEncapType::None),
                InfoIpTunnel::EncapSPort(10),
                InfoIpTunnel::EncapDPort(12),
                InfoIpTunnel::EncapFlags(TunnelEncapFlags::from_bits_retain(0)),
            ])),
        ])],
    };

    assert_eq!(
        expected,
        LinkMessage::parse(&LinkMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}

#[test]
fn test_iptunnel_ip6ip6_link_info() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, // AF_UNSPEC and reserved
        0x01, 0x03, // Link Layer Type IP6TUNNEL (769)
        0x06, 0x00, 0x00, 0x00, // iface ifindex 6
        0x90, 0x00, 0x00, 0x00, // flags
        0x00, 0x00, 0x00, 0x00, // changed flags
        0x94, 0x00, // length 148
        0x12, 0x00, // IFLA_LINK_INFO (18)
        0x0b, 0x00, 0x01, 0x00, 0x69, 0x70, 0x36, 0x74, 0x6e, 0x6c, 0x00, 0x00,
        0x84, 0x00, 0x02, 0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x14, 0x00, 0x02, 0x00, 0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x14, 0x00, 0x03, 0x00,
        0x20, 0x01, 0x0d, 0xb8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x02, 0x05, 0x00, 0x04, 0x00, 0x40, 0x00, 0x00, 0x00,
        0x05, 0x00, 0x06, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x07, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00,
        0x05, 0x00, 0x09, 0x00, 0x29, 0x00, 0x00, 0x00, 0x08, 0x00, 0x14, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x06, 0x00, 0x11, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x06, 0x00, 0x12, 0x00,
        0x00, 0x0c, 0x00, 0x00, 0x06, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00,
        0x00, // data
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 6,
            link_layer_type: LinkLayerType::Tunnel6,
            flags: LinkFlags::Noarp | LinkFlags::Pointopoint,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::Ip6Tnl),
            LinkInfo::Data(InfoData::IpTunnel(vec![
                InfoIpTunnel::Link(0),
                InfoIpTunnel::Local(std::net::IpAddr::V6(
                    Ipv6Addr::from_str("2001:db8:1::1").unwrap(),
                )),
                InfoIpTunnel::Remote(std::net::IpAddr::V6(
                    Ipv6Addr::from_str("2001:db8:1::2").unwrap(),
                )),
                InfoIpTunnel::Ttl(64),
                InfoIpTunnel::EncapLimit(4),
                InfoIpTunnel::FlowInfo(0),
                InfoIpTunnel::Ipv6Flags(
                    Ip6TunnelFlags::CapXmit | Ip6TunnelFlags::CapRcv,
                ),
                InfoIpTunnel::Protocol(IpProtocol::Ipv6),
                InfoIpTunnel::FwMark(0),
                InfoIpTunnel::EncapType(TunnelEncapType::None),
                InfoIpTunnel::EncapSPort(10),
                InfoIpTunnel::EncapDPort(12),
                InfoIpTunnel::EncapFlags(TunnelEncapFlags::from_bits_retain(0)),
            ])),
        ])],
    };

    assert_eq!(
        expected,
        LinkMessage::parse(&LinkMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}

#[test]
fn test_iptunnel_sit_link_info() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, // AF_UNSPEC, reserved
        0x00, 0x03, // LL type = IPTUNNEL (768)
        0x07, 0x00, 0x00, 0x00, // ifindex = 7
        0x90, 0x00, 0x00, 0x00, // flags = NoARP|POINTOPOINT
        0x00, 0x00, 0x00, 0x00, // change_mask = 0
        // --- IFLA_LINK_INFO NLA (len=0x78, type=18) ---
        0x78, 0x00, 0x12, 0x00,
        // IFLA_INFO_KIND = "sit\0" (len=8, type=1)
        0x08, 0x00, 0x01, 0x00, b's', b'i', b't', 0x00, 0x6c, 0x00, 0x02, 0x00,
        0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x02, 0x00,
        0xc0, 0xa8, 0x7a, 0xb7, 0x08, 0x00, 0x03, 0x00, 0x0a, 0xff, 0xfe, 0x02,
        0x05, 0x00, 0x04, 0x00, 0x40, 0x00, 0x00, 0x00, 0x05, 0x00, 0x05, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x05, 0x00, 0x09, 0x00, 0x29, 0x00, 0x00, 0x00, 0x06, 0x00, 0x08, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x06, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x11, 0x00,
        0x00, 0x0a, 0x00, 0x00, 0x06, 0x00, 0x12, 0x00, 0x00, 0x0c, 0x00, 0x00,
        0x06, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 7,
            link_layer_type: LinkLayerType::Tunnel,
            flags: LinkFlags::Noarp | LinkFlags::Pointopoint,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::SitTun),
            LinkInfo::Data(InfoData::IpTunnel(vec![
                InfoIpTunnel::Link(0),
                InfoIpTunnel::Local(std::net::IpAddr::V4(
                    Ipv4Addr::from_str("192.168.122.183").unwrap(),
                )),
                InfoIpTunnel::Remote(std::net::IpAddr::V4(
                    Ipv4Addr::from_str("10.255.254.2").unwrap(),
                )),
                InfoIpTunnel::Ttl(64),
                InfoIpTunnel::Tos(0),
                InfoIpTunnel::PMtuDisc(true),
                InfoIpTunnel::Protocol(IpProtocol::Ipv6),
                InfoIpTunnel::Ipv6SitFlags(0),
                InfoIpTunnel::FwMark(0),
                InfoIpTunnel::EncapType(TunnelEncapType::None),
                InfoIpTunnel::EncapSPort(10),
                InfoIpTunnel::EncapDPort(12),
                InfoIpTunnel::EncapFlags(TunnelEncapFlags::empty()),
            ])),
        ])],
    };

    assert_eq!(
        expected,
        LinkMessage::parse(&LinkMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
