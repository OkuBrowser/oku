// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        link_flag::LinkFlags, InfoKind, LinkAttribute, LinkHeader, LinkInfo,
        LinkLayerType, LinkMessage,
    },
    AddressFamily,
};

#[test]
fn test_parsing_link_pfcp() {
    // nlmon capture (netlink message header removed) of kernel emitted
    // RTM_NEWLINK notification for command: `ip link add pfcp0 type pfcp`
    let raw = vec![
        0x00, 0x00, // interface family AF_UNSPEC, reserved
        0xfe, 0xff, // link layer type ARPHRD_NONE
        0x10, 0x00, 0x00, 0x00, // interface index 16
        0x90, 0x10, 0x00,
        0x00, // flags IFF_POINTOPOINT|IFF_NOARP|IFF_MULTICAST
        0x00, 0x00, 0x00, 0x00, // change flags 0
        0x0a, 0x00, // length 10
        0x03, 0x00, // IFLA_IFNAME
        0x70, 0x66, 0x63, 0x70, 0x30, 0x00, // 'pfcp0\0'
        0x00, 0x00, // padding
        0x10, 0x00, // length 16
        0x12, 0x00, // IFLA_LINKINFO
        0x09, 0x00, // length 9
        0x01, 0x00, // IFLA_INFO_KIND 1
        0x70, 0x66, 0x63, 0x70, 0x00, // 'pfcp\0'
        0x00, 0x00, 0x00, // padding
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 16,
            link_layer_type: LinkLayerType::None,
            flags: LinkFlags::Pointopoint
                | LinkFlags::Noarp
                | LinkFlags::Multicast,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![
            LinkAttribute::IfName("pfcp0".to_string()),
            LinkAttribute::LinkInfo(vec![LinkInfo::Kind(InfoKind::Pfcp)]),
        ],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
