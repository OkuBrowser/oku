// SPDX-License-Identifier: MIT

use netlink_packet_core::{Emitable, Parseable};

use crate::{
    link::{
        link_flag::LinkFlags, InfoBareUdp, InfoData, InfoKind, LinkAttribute,
        LinkHeader, LinkInfo, LinkLayerType, LinkMessage,
    },
    AddressFamily, EthernetProtocol,
};

// nlmon capture of netlink packet sent by command:
//      ip link add bareudp0 type bareudp port 6635 ethertype mpls_uc
#[test]
fn test_parsing_link_bareudp_basic() {
    #[rustfmt::skip]
    let raw = vec![
        // ifinfomsg (16 bytes)
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        // IFLA_IFNAME, len=13, type=3
        0x0d, 0x00, 0x03, 0x00,
        0x62, 0x61, 0x72, 0x65,
        0x75, 0x64, 0x70, 0x30,
        0x00, 0x00, 0x00, 0x00,
        // IFLA_LINKINFO, len=36, type=18
        0x24, 0x00, 0x12, 0x00,
        // IFLA_INFO_KIND, len=12, type=1
        0x0c, 0x00, 0x01, 0x00,
        0x62, 0x61, 0x72, 0x65,
        0x75, 0x64, 0x70, 0x00,
        // IFLA_INFO_DATA, len=20, type=2
        0x14, 0x00, 0x02, 0x00,
        // IFLA_BAREUDP_PORT, len=6, type=1, value=6635
        0x06, 0x00, 0x01, 0x00,
        0x19, 0xeb, 0x00, 0x00,
        // IFLA_BAREUDP_ETHERTYPE, len=6, type=2, value=0x8847 (mpls_uc)
        0x06, 0x00, 0x02, 0x00,
        0x88, 0x47, 0x00, 0x00,
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
            LinkAttribute::IfName("bareudp0".to_string()),
            LinkAttribute::LinkInfo(vec![
                LinkInfo::Kind(InfoKind::BareUdp),
                LinkInfo::Data(InfoData::BareUdp(vec![
                    InfoBareUdp::Port(6635),
                    InfoBareUdp::Ethertype(EthernetProtocol::MplsUc),
                ])),
            ]),
        ],
    };

    assert_eq!(expected, LinkMessage::parse(&raw).unwrap());

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
