// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::link::{
    link_flag::LinkFlags, CanBitTiming, CanCtrlMode, CanCtrlModeFlags, InfoCan,
    InfoData, InfoKind, LinkAttribute, LinkHeader, LinkInfo, LinkLayerType,
    LinkMessage,
};

// nlmon capture of packet sent by
//  ip link add can0 type can bitrate 500000 loopback on
// Not on real hardware
#[test]
fn test_parsing_link_can_add() {
    let raw = vec![
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x03, 0x00, 0x63, 0x61, 0x6e, 0x30,
        0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x12, 0x00, 0x08, 0x00, 0x01, 0x00,
        0x63, 0x61, 0x6e, 0x00, 0x34, 0x00, 0x02, 0x00, 0x24, 0x00, 0x01, 0x00,
        0x50, 0xc3, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x05, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: crate::AddressFamily::Unspec,
            index: 0,
            link_layer_type: LinkLayerType::Netrom,
            flags: LinkFlags::empty(),
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![
            LinkAttribute::IfName("can0".to_string()),
            LinkAttribute::LinkInfo(vec![
                LinkInfo::Kind(InfoKind::Can),
                LinkInfo::Data(InfoData::Can(vec![
                    InfoCan::BitTiming(CanBitTiming {
                        bitrate: 50000,
                        sample_point: 0,
                        tq: 0,
                        prop_seg: 0,
                        phase_seg1: 0,
                        phase_seg2: 0,
                        sjw: 0,
                        brp: 0,
                    }),
                    InfoCan::CtrlMode(CanCtrlMode {
                        mask: CanCtrlModeFlags::Loopback,
                        flags: CanCtrlModeFlags::empty(),
                    }),
                ])),
            ]),
        ],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];
    expected.emit(&mut buf);
    assert_eq!(buf, raw);
}
