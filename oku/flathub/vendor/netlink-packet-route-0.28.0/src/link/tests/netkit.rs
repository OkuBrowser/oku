// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        InfoData, InfoKind, InfoNetkit, LinkAttribute, LinkHeader, LinkInfo,
        LinkLayerType, LinkMessage, LinkMessageBuffer, NetkitMode,
        NetkitPolicy,
    },
    AddressFamily,
};

#[test]
fn test_create_netkit() {
    // Captured from kernel 6.14.0 with iproute2-6.14.0
    // ```sh
    // sudo ip link add nlmon0 type nlmon
    // sudo ip link set nlmon0 up
    // sudo tcpdump -i nlmon0 -w netkit.pcap &
    // sudo ip link add nktest0 type netkit peer name nktest1
    // sudo pkill tcpdump
    // ```
    //
    // Tests basic netkit attributes (PRIMARY, POLICY, MODE) without
    // kernel-version-specific attributes that may vary
    let raw: Vec<u8> = vec![
        0x00, // interface family AF_UNSPEC
        0x00, // reserved
        0x00, 0x00, // link layer type 0 (Netrom)
        0x00, 0x00, 0x00, 0x00, // iface index 0
        0x00, 0x00, 0x00, 0x00, // device flags 0
        0x00, 0x00, 0x00, 0x00, // change flags 0
        0x0c, 0x00, // length 12
        0x03, 0x00, // IFLA_IFNAME
        0x6e, 0x6b, 0x74, 0x65, 0x73, 0x74, 0x30, 0x00, // "nktest0\0"
        0x2c, 0x00, // length 44
        0x12, 0x00, // IFLA_LINKINFO 18
        0x0b, 0x00, // length 11
        0x01, 0x00, // IFLA_INFO_KIND 1
        0x6e, 0x65, 0x74, 0x6b, 0x69, 0x74, 0x00, // 'netkit\0'
        0x00, // padding
        0x1c, 0x00, // length 28
        0x02, 0x00, // IFLA_INFO_DATA 2
        0x05, 0x00, // length 5
        0x02, 0x00, // NETKIT_INFO_PRIMARY
        0x01, // value: true
        0x00, 0x00, 0x00, // padding
        0x08, 0x00, // length 8
        0x03, 0x00, // NETKIT_INFO_POLICY
        0x00, 0x00, 0x00, 0x00, // value: 0 (PASS)
        0x08, 0x00, // length 8
        0x05, 0x00, // NETKIT_INFO_MODE
        0x01, 0x00, 0x00, 0x00, // value: 1 (L3)
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 0,
            link_layer_type: LinkLayerType::Netrom,
            ..Default::default()
        },
        attributes: vec![
            LinkAttribute::IfName("nktest0".to_string()),
            LinkAttribute::LinkInfo(vec![
                LinkInfo::Kind(InfoKind::Netkit),
                LinkInfo::Data(InfoData::Netkit(vec![
                    InfoNetkit::Primary(true),
                    InfoNetkit::Policy(NetkitPolicy::Pass),
                    InfoNetkit::Mode(NetkitMode::L3),
                ])),
            ]),
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
