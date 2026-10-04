// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        InfoData, InfoIpoib, InfoKind, IpoibMode, LinkAttribute, LinkFlags,
        LinkHeader, LinkInfo, LinkLayerType, LinkMessage, LinkMessageBuffer,
    },
    AddressFamily,
};

// nlmon capture of commmand:
//  ip link add veth1 naem veth1.8001  type ipoib pkey 0x8001 mode connected
//
// Fixed the length of IFLA_INFO_KIND which is caused by iproute bug.
//
// This is not running against real InfiniBand NIC, so might be incorrect.
// Gris will update this once he got a server with real NIC.
#[test]
fn test_create_ipoib_with_pkey_and_mode() {
    let raw: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x03, 0x00, 0x76, 0x65, 0x74, 0x68,
        0x31, 0x2e, 0x38, 0x30, 0x30, 0x31, 0x00, 0x00, 0x24, 0x00, 0x12, 0x00,
        0x0a, 0x00, 0x01, 0x00, 0x69, 0x70, 0x6f, 0x69, 0x62, 0x00, 0x00, 0x00,
        0x14, 0x00, 0x02, 0x00, 0x06, 0x00, 0x01, 0x00, 0x01, 0x80, 0x00, 0x00,
        0x06, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 0,
            link_layer_type: LinkLayerType::Netrom,
            flags: LinkFlags::empty(),
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![
            LinkAttribute::IfName("veth1.8001".to_string()),
            LinkAttribute::LinkInfo(vec![
                LinkInfo::Kind(InfoKind::Ipoib),
                LinkInfo::Data(InfoData::Ipoib(vec![
                    InfoIpoib::Pkey(0x8001),
                    InfoIpoib::Mode(IpoibMode::Connected),
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
