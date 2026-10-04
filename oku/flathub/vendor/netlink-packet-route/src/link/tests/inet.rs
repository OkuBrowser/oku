// SPDX-License-Identifier: MIT

use std::mem::size_of;

use netlink_packet_core::{Nla, NLA_F_NESTED};

use crate::link::{AfSpecInet, InetDevConf, InetDevConfBuffer};

const IFLA_INET_CONF: u16 = 1;

#[test]
fn test_devconf_request_emit_forwarding_on() {
    let conf = InetDevConf {
        forwarding: 1,
        ..Default::default()
    };

    let req = AfSpecInet::DevConfRequest(conf);
    let val_len = req.value_len();
    let mut buf = vec![0u8; val_len];
    req.emit_value(&mut buf);

    // Reference bytes captured from iproute2 `ip link set ... inet forwarding
    // on`: Nested NLA entry: len=8, type=IPV4_DEVCONF_FORWARDING(1), val=1
    let expected: &[u8] = &[0x08, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00];
    assert_eq!(val_len, 8);
    assert_eq!(buf, expected);
}

#[test]
fn test_devconf_request_emit_rp_filter() {
    let conf = InetDevConf {
        rp_filter: 2,
        ..Default::default()
    };

    let req = AfSpecInet::DevConfRequest(conf);
    let val_len = req.value_len();
    let mut buf = vec![0u8; val_len];
    req.emit_value(&mut buf);

    // Nested NLA entry: len=8, type=IPV4_DEVCONF_RP_FILTER(8), val=2
    let expected: &[u8] = &[0x08, 0x00, 0x08, 0x00, 0x02, 0x00, 0x00, 0x00];
    assert_eq!(val_len, 8);
    assert_eq!(buf, expected);
}

#[test]
fn test_devconf_request_kind_has_nested_flag() {
    let conf = InetDevConf::default();
    let req = AfSpecInet::DevConfRequest(conf);
    assert_eq!(req.kind() & NLA_F_NESTED, NLA_F_NESTED);
    assert_eq!(req.kind() & !NLA_F_NESTED, IFLA_INET_CONF);
}

#[test]
fn test_devconf_flat_emit_forwarding_on() {
    let conf = InetDevConf {
        forwarding: 1,
        ..Default::default()
    };

    let req = AfSpecInet::DevConf(conf);
    let val_len = req.value_len();
    let mut buf = vec![0u8; val_len];
    req.emit_value(&mut buf);

    // Flat buffer: 132 bytes, first 4 bytes = forwarding=1
    assert_eq!(val_len, size_of::<InetDevConfBuffer>());
    assert_eq!(&buf[0..4], &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn test_devconf_flat_kind_no_nested_flag() {
    let conf = InetDevConf::default();
    let req = AfSpecInet::DevConf(conf);
    assert_eq!(req.kind() & NLA_F_NESTED, 0);
    assert_eq!(req.kind(), IFLA_INET_CONF);
}
