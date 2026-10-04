// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        link_flag::LinkFlags, GtpRole, InfoData, InfoGtp, InfoKind,
        LinkAttribute, LinkHeader, LinkInfo, LinkLayerType, LinkMessage,
    },
    AddressFamily,
};

#[test]
fn test_parsing_link_gtp() {
    // nlmon capture (netlink message header removed) of command:
    // `ip link add gtp0 type gtp role sgsn hsize 1024`
    let raw = vec![
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x03, 0x00, 0x67, 0x74, 0x70, 0x30,
        0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x12, 0x00, 0x08, 0x00, 0x01, 0x00,
        0x67, 0x74, 0x70, 0x00, 0x1c, 0x00, 0x02, 0x00, 0x05, 0x00, 0x05, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x03, 0x00, 0x00, 0x04, 0x00, 0x00,
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
            LinkAttribute::IfName("gtp0".to_string()),
            LinkAttribute::LinkInfo(vec![
                LinkInfo::Kind(InfoKind::Gtp),
                LinkInfo::Data(InfoData::Gtp(vec![
                    InfoGtp::CreateSockets(true),
                    InfoGtp::Role(GtpRole::Sgsn),
                    InfoGtp::PdpHashsize(1024),
                ])),
            ]),
        ],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
