// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        link_flag::LinkFlags, link_state::State, InfoData, InfoKind, InfoVxcan,
        LinkAttribute, LinkHeader, LinkInfo, LinkLayerType, LinkMessage,
        LinkMessageBuffer, LinkMode,
    },
    AddressFamily,
};

#[test]
fn test_vxcan_get_link_info() {
    // Captured from kernel 6.12.69 with iproute2-6.15.0
    // ```sh
    // sudo ip link add nl0 type nlmon
    // sudo ip link set nl0 up
    // sudo ip link add vxcan0 type vxcan peer name vxcan1
    // sudo ip link set vxcan0 up
    // sudo ip link set vxcan1 up
    // sudo tcpdump -i nl0 -w create_vxcan.pcap &
    // sudo ip link show vxcan0
    // sudo pkill tcpdump
    // ```
    let raw: Vec<u8> = vec![
        0x00, 0x00, // AF_UNSPEC and reserved
        0x18, 0x01, // Link layer type CAN(280)
        0x19, 0x00, 0x00, 0x00, // iface index 25
        0xc1, 0x00, 0x01, 0x00, // flags
        0x00, 0x00, 0x00, 0x00, // changed flags0
        0x0b, 0x00, // length 11
        0x03, 0x00, // IFLA_IFNAME
        0x76, 0x78, 0x63, 0x61, 0x6e, 0x30, 0x00, 0x00, // "vxcan0\0"
        0x08, 0x00, // length 8
        0x0d, 0x00, // IFLA_TXQLEN 13
        0xe8, 0x03, 0x00, 0x00, // Tx Queue Length 1000
        0x05, 0x00, // length 5
        0x10, 0x00, // IFLA_OPERSTATE 16
        0x06, // Up
        0x00, 0x00, 0x00, // padding
        0x05, 0x00, // length 5
        0x11, 0x00, // IFLA_LINKMODE 17
        0x00, // Value 0
        0x00, 0x00, 0x00, // padding
        0x08, 0x00, // length 5
        0x04, 0x00, // IFLA_MTU 4
        0x48, 0x00, 0x00, 0x00, // MTU 72
        // Some attributes omitted
        0x10, 0x00, // length 16
        0x12, 0x00, // IFLA_LINKINFO 18
        0x0a, 0x00, // length 10
        0x01, 0x00, // IFLA_INFO_KIND 1
        0x76, 0x78, 0x63, 0x61, 0x6e, 0x00, // "vxcan\0"
        0x00, 0x00, // padding
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 25,
            link_layer_type: LinkLayerType::Can,
            flags: LinkFlags::LowerUp
                | LinkFlags::Noarp
                | LinkFlags::Running
                | LinkFlags::Up,
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![
            LinkAttribute::IfName("vxcan0".to_string()),
            LinkAttribute::TxQueueLen(1000),
            LinkAttribute::OperState(State::Up),
            LinkAttribute::Mode(LinkMode::Default),
            LinkAttribute::Mtu(72),
            LinkAttribute::LinkInfo(vec![LinkInfo::Kind(InfoKind::Vxcan)]),
        ],
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
fn test_create_vxcan() {
    // Captured from kernel 6.12.69 with iproute2-6.15.0
    // ```sh
    // sudo ip link add nl0 type nlmon
    // sudo ip link set nl0 up
    // sudo tcpdump -i nl0 -w create_vxcan.pcap &
    // sudo ip link add vxcan0 type vxcan peer name vxcan1
    // sudo pkill tcpdump
    // ```
    //
    // Due to the iproute bug (https://issues.redhat.com/browse/RHEL-14964),
    // The IFLA_INFO_KIND length has been manually fixed to 10 instead of the 9
    // produced by iproute.
    let raw: Vec<u8> = vec![
        0x00, // interface family AF_UNSPEC
        0x00, // reserved
        0x00, 0x00, // link layer type 0
        0x00, 0x00, 0x00, 0x00, // iface index 0
        0x00, 0x00, 0x00, 0x00, // device flags 0
        0x00, 0x00, 0x00, 0x00, // change flags 0
        0x0b, 0x00, // length 11
        0x03, 0x00, // IFLA_IFNAME
        0x76, 0x78, 0x63, 0x61, 0x6e, 0x30, 0x00, 0x00, // "vxcan0\0"
        0x34, 0x00, // length 52
        0x12, 0x00, // IFLA_LINKINFO 18
        0x0a, 0x00, // length 10
        0x01, 0x00, // IFLA_INFO_KIND 1
        0x76, 0x78, 0x63, 0x61, 0x6e, 0x00, // "vxcan\0"
        0x00, 0x00, // padding
        0x24, 0x00, // length 36
        0x02, 0x00, // IFLA_INFO_DATA 2
        0x20, 0x00, // length 32
        0x01, 0x00, // VETH_INFO_PEER 1
        0x00, // Netlink Message header: family AF_UNSPEC
        0x00, // reserved
        0x00, 0x00, // link layer type 0
        0x00, 0x00, 0x00, 0x00, // iface index 0
        0x00, 0x00, 0x00, 0x00, // device flags 0
        0x00, 0x00, 0x00, 0x00, // change flags 0
        0x0b, 0x00, // length 11
        0x03, 0x00, // IFLA_IFNAME 3
        0x76, 0x78, 0x63, 0x61, 0x6e, 0x31, 0x00, 0x00, // "vxcan1\0"
    ];

    let expected = LinkMessage {
        attributes: vec![
            LinkAttribute::IfName("vxcan0".to_string()),
            LinkAttribute::LinkInfo(vec![
                LinkInfo::Kind(InfoKind::Vxcan),
                LinkInfo::Data(InfoData::Vxcan(InfoVxcan::Peer(LinkMessage {
                    attributes: vec![LinkAttribute::IfName(
                        "vxcan1".to_string(),
                    )],
                    ..Default::default()
                }))),
            ]),
        ],
        ..Default::default()
    };

    assert_eq!(
        expected,
        LinkMessage::parse(&LinkMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
