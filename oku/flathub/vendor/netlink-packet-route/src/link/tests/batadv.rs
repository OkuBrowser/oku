// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        link_flag::LinkFlags, InfoBatAdv, InfoData, InfoKind, LinkAttribute,
        LinkHeader, LinkInfo, LinkLayerType, LinkMessage,
    },
    AddressFamily,
};

// nlmon capture of netlink packet sent by command:
//      ip link add dev bat0 type batadv ra BATMAN_IV
#[test]
fn test_parsing_link_batadv_with_ra() {
    #[rustfmt::skip]
    let raw = vec![
        // ifinfomsg (16 bytes)
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        // IFLA_IFNAME, len=9, type=3
        0x09, 0x00, 0x03, 0x00,
        0x62, 0x61, 0x74, 0x30,
        0x00, 0x00, 0x00, 0x00,
        // IFLA_LINKINFO, len=36, type=18
        0x24, 0x00, 0x12, 0x00,
        // IFLA_INFO_KIND, len=11, type=1
        0x0b, 0x00, 0x01, 0x00,
        0x62, 0x61, 0x74, 0x61,
        0x64, 0x76, 0x00, 0x00,
        // IFLA_INFO_DATA, len=20, type=2
        0x14, 0x00, 0x02, 0x00,
        // IFLA_BATADV_ALGO_NAME, len=14, type=1, value="BATMAN_IV"
        0x0e, 0x00, 0x01, 0x00,
        0x42, 0x41, 0x54, 0x4d,
        0x41, 0x4e, 0x5f, 0x49,
        0x56, 0x00, 0x00, 0x00,
    ];

    let expected = LinkMessage {
        header: LinkHeader {
            interface_family: AddressFamily::Unspec,
            index: 0,
            link_layer_type: LinkLayerType::default(),
            flags: LinkFlags::empty(),
            change_mask: LinkFlags::empty(),
        },
        attributes: vec![
            LinkAttribute::IfName("bat0".to_string()),
            LinkAttribute::LinkInfo(vec![
                LinkInfo::Kind(InfoKind::BatAdv),
                LinkInfo::Data(InfoData::BatAdv(vec![InfoBatAdv::AlgoName(
                    "BATMAN_IV".to_string(),
                )])),
            ]),
        ],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
