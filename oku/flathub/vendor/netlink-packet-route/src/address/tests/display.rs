// SPDX-License-Identifier: MIT

use crate::{
    address::{AddressProtocol, AddressScope},
    AddressFamily,
};

#[test]
fn test_address_family_display() {
    assert_eq!(AddressFamily::Unspec.to_string(), "unspec");
    assert_eq!(AddressFamily::Local.to_string(), "local");
    assert_eq!(AddressFamily::Unix.to_string(), "unix");
    assert_eq!(AddressFamily::Inet.to_string(), "inet");
    assert_eq!(AddressFamily::Inet6.to_string(), "inet6");
    assert_eq!(AddressFamily::Other(255).to_string(), "255");
}

#[cfg(any(target_os = "linux", target_os = "fuchsia", target_os = "android"))]
#[test]
fn test_linux_address_family_display() {
    assert_eq!(AddressFamily::Packet.to_string(), "packet");
    assert_eq!(AddressFamily::Bridge.to_string(), "bridge");
    assert_eq!(AddressFamily::Ieee802154.to_string(), "ieee802154");
    assert_eq!(AddressFamily::Qipcrtr.to_string(), "qipcrtr");

    #[cfg(not(target_os = "android"))]
    {
        assert_eq!(AddressFamily::Ib.to_string(), "ib");
        assert_eq!(AddressFamily::Mpls.to_string(), "mpls");
    }
}

#[cfg(target_os = "freebsd")]
#[test]
fn test_freebsd_address_family_display() {
    assert_eq!(AddressFamily::Implink.to_string(), "implink");
    assert_eq!(AddressFamily::Link.to_string(), "link");
    assert_eq!(AddressFamily::Netgraph.to_string(), "netgraph");
}

#[test]
fn test_address_scope_display() {
    assert_eq!(AddressScope::Universe.to_string(), "universe");
    assert_eq!(AddressScope::Site.to_string(), "site");
    assert_eq!(AddressScope::Link.to_string(), "link");
    assert_eq!(AddressScope::Host.to_string(), "host");
    assert_eq!(AddressScope::Nowhere.to_string(), "nowhere");
    assert_eq!(AddressScope::Other(42).to_string(), "42");
}

#[test]
fn test_address_protocol_display() {
    assert_eq!(AddressProtocol::Loopback.to_string(), "kernel_lo");
    assert_eq!(AddressProtocol::RouterAnnouncement.to_string(), "kernel_ra");
    assert_eq!(AddressProtocol::LinkLocal.to_string(), "kernel_ll");
    assert_eq!(AddressProtocol::Other(42).to_string(), "42");
}
