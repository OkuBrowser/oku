// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        link_flag::LinkFlags, InfoData, InfoKind, InfoMacVlan, LinkAttribute,
        LinkHeader, LinkInfo, LinkLayerType, LinkMessage, LinkMessageBuffer,
        MacVlanFlags, MacVlanMacAddressMode, MacVlanMode,
    },
    AddressFamily,
};

#[test]
fn test_macvlan_link_info() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, 0x01, 0x00, 0x17, 0x00, 0x00, 0x00, 0x43, 0x10, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x58, 0x00, 0x12, 0x00, 0x0c, 0x00, 0x01, 0x00,
        0x6d, 0x61, 0x63, 0x76, 0x6c, 0x61, 0x6e, 0x00, 0x48, 0x00, 0x02, 0x00,
        0x08, 0x00, 0x01, 0x00, 0x10, 0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x06, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x1c, 0x00, 0x05, 0x00, 0x0a, 0x00, 0x04, 0x00, 0x00, 0x23, 0x45, 0x67,
        0x89, 0x1d, 0x00, 0x00, 0x0a, 0x00, 0x04, 0x00, 0x00, 0x23, 0x45, 0x67,
        0x89, 0x1c, 0x00, 0x00, 0x08, 0x00, 0x07, 0x00, 0xe8, 0x03, 0x00, 0x00,
        0x08, 0x00, 0x08, 0x00, 0xe8, 0x03, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 23,
            link_layer_type: LinkLayerType::Ether,
            flags: LinkFlags::Broadcast
                | LinkFlags::LowerUp
                | LinkFlags::Multicast
                | LinkFlags::Running
                | LinkFlags::Up,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::MacVlan),
            LinkInfo::Data(InfoData::MacVlan(vec![
                InfoMacVlan::Mode(MacVlanMode::Source),
                InfoMacVlan::Flags(MacVlanFlags::empty()),
                InfoMacVlan::MacAddrCount(2),
                InfoMacVlan::MacAddrData(vec![
                    InfoMacVlan::MacAddr([0, 35, 69, 103, 137, 29]),
                    InfoMacVlan::MacAddr([0, 35, 69, 103, 137, 28]),
                ]),
                InfoMacVlan::BcQueueLen(1000),
                InfoMacVlan::BcQueueLenUsed(1000),
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

// nlmon of command:
//      ip link set mac0 type macvlan \
//          macaddr add 00:23:45:67:89:1f
//
// With modification on the IFLA_INFO_KIND length because iproute has bug on
// it.
#[test]
fn test_macvlan_modify_mac_addr_mode() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00, 0xdb, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x12, 0x00, 0x0c, 0x00, 0x01, 0x00,
        0x6d, 0x61, 0x63, 0x76, 0x6c, 0x61, 0x6e, 0x00, 0x18, 0x00, 0x02, 0x00,
        0x08, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x04, 0x00,
        0x00, 0x23, 0x45, 0x67, 0x89, 0x1f, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 219,
            link_layer_type: LinkLayerType::Netrom,
            flags: LinkFlags::empty(),
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![LinkAttribute::LinkInfo(vec![
            LinkInfo::Kind(InfoKind::MacVlan),
            LinkInfo::Data(InfoData::MacVlan(vec![
                InfoMacVlan::MacAddrMode(MacVlanMacAddressMode::Add),
                InfoMacVlan::MacAddr([0, 35, 69, 103, 137, 31]),
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
