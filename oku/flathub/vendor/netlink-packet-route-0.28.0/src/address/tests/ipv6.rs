// SPDX-License-Identifier: MIT

use std::{
    net::{IpAddr, Ipv6Addr},
    str::FromStr,
};

use netlink_packet_core::{Emitable, NlaBuffer, Parseable};

use crate::{
    address::{
        AddressAttribute, AddressFlags, AddressHeader, AddressHeaderFlags,
        AddressMessage, AddressMessageBuffer, AddressProtocol, AddressScope,
        CacheInfo,
    },
    AddressFamily,
};

// TODO(Gris Ge): Need test for `AddressAttribute::Anycast` and `Multicast`.

#[test]
fn test_addr_flag_stable_privacy() {
    let nla = AddressAttribute::Flags(
        AddressFlags::Permanent | AddressFlags::StablePrivacy,
    );

    let raw: [u8; 8] = [
        0x08, 0x00, // length 8
        0x08, 0x00, // IFA_FLAGS
        0x80, 0x08, 0x00, 0x00, // IFA_F_PERMANENT | IFA_F_STABLE_PRIVACY
    ];

    let nla_buffer = NlaBuffer::new_checked(&raw).unwrap();
    let parsed = AddressAttribute::parse(&nla_buffer).unwrap();
    assert_eq!(parsed, nla);

    assert_eq!(nla.buffer_len(), 8);

    let mut buffer: [u8; 8] = [0; 8];
    nla.emit(&mut buffer);
    assert_eq!(buffer, raw);
}

#[test]
fn test_get_loopback_ipv6_addr() {
    let raw = vec![
        0x0a, 0x80, 0x80, 0xfe, 0x01, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x01, 0x14, 0x00, 0x06, 0x00, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0x8e, 0x00, 0x00, 0x00, 0x8e, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x08, 0x00, 0x80, 0x02, 0x00, 0x00,
    ];

    let expected = AddressMessage {
        header: AddressHeader {
            family: AddressFamily::Inet6,
            prefix_len: 128,
            flags: AddressHeaderFlags::Permanent,
            scope: AddressScope::Host,
            index: 1,
        },
        attributes: vec![
            AddressAttribute::Address(IpAddr::V6(Ipv6Addr::LOCALHOST)),
            AddressAttribute::CacheInfo(CacheInfo {
                ifa_preferred: u32::MAX,
                ifa_valid: u32::MAX,
                cstamp: 142,
                tstamp: 142,
            }),
            AddressAttribute::Flags(
                AddressFlags::Permanent | AddressFlags::Noprefixroute,
            ),
        ],
    };

    assert_eq!(
        expected,
        AddressMessage::parse(&AddressMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}

#[test]
fn test_get_ipv6_address_ra_protocol() {
    let raw = vec![
        0x0a, 0x40, 0x00, 0x00, 0x0f, 0x01, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00,
        0x20, 0x01, 0x0d, 0xb8, 0x00, 0x0a, 0x00, 0x00, 0xb4, 0x02, 0xdf, 0xff,
        0xfe, 0x56, 0xc3, 0xce, 0x14, 0x00, 0x06, 0x00, 0x03, 0x07, 0x00, 0x00,
        0x03, 0x07, 0x00, 0x00, 0x63, 0x4f, 0x34, 0x01, 0x4f, 0x5e, 0x34, 0x01,
        0x08, 0x00, 0x08, 0x00, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x0b, 0x00,
        0x02, 0x00, 0x00, 0x00,
    ];

    let expected = AddressMessage {
        header: AddressHeader {
            family: AddressFamily::Inet6,
            prefix_len: 64,
            flags: AddressHeaderFlags::empty(),
            scope: AddressScope::Universe,
            index: 271,
        },
        attributes: vec![
            AddressAttribute::Address(
                IpAddr::from_str("2001:db8:a:0:b402:dfff:fe56:c3ce").unwrap(),
            ),
            AddressAttribute::CacheInfo(CacheInfo {
                ifa_preferred: 1795,
                ifa_valid: 1795,
                cstamp: 20205411,
                tstamp: 20209231,
            }),
            AddressAttribute::Flags(AddressFlags::Managetempaddr),
            AddressAttribute::Protocol(AddressProtocol::RouterAnnouncement),
        ],
    };

    assert_eq!(
        expected,
        AddressMessage::parse(&AddressMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
