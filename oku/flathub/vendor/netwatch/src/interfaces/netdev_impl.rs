//! Conversion from the `netdev` crate into our platform-agnostic types.
//!
//! This module also holds the interface enumeration and home-router lookup
//! shared by all `netdev`-capable platforms (linux, android, bsd, macos,
//! windows).
//!
//! This is the only module that depends on `netdev`. Everything it produces is
//! expressed in terms of the types defined in [`crate::interfaces`].

use std::net::IpAddr;

use super::{Interface, IpNet, Ipv6AddrFlags, State};
use crate::ip::{LocalAddresses, is_link_local, is_private, is_private_v6};

const IFF_UP: u32 = 0x1;
const IFF_LOOPBACK: u32 = 0x8;

/// Converts netdev's IPv6 address flags into our mirrored [`Ipv6AddrFlags`].
///
/// This is a free function rather than a `From` impl on purpose: a public
/// `From<netdev::...>` would re-expose the `netdev` type in our public API,
/// which is exactly what the [`Ipv6AddrFlags`] mirror exists to avoid.
fn to_ipv6_addr_flags(flags: netdev::interface::ipv6_addr_flags::Ipv6AddrFlags) -> Ipv6AddrFlags {
    Ipv6AddrFlags {
        deprecated: flags.deprecated,
        temporary: flags.temporary,
        tentative: flags.tentative,
        duplicated: flags.duplicated,
        permanent: flags.permanent,
    }
}

/// Converts a [`netdev::Interface`] into our platform-agnostic [`Interface`].
///
/// Addresses are sorted (IPv4 first, then IPv6, each by address) so that
/// comparisons between successive snapshots are stable.
fn to_interface(iface: netdev::Interface) -> Interface {
    // netdev keeps these three IPv6 arrays parallel, one entry per address.
    // The zip below relies on that; assert it so a netdev change that breaks
    // the invariant surfaces in tests rather than silently dropping addresses.
    debug_assert_eq!(iface.ipv6.len(), iface.ipv6_scope_ids.len());
    debug_assert_eq!(iface.ipv6.len(), iface.ipv6_addr_flags.len());

    let mut v4: Vec<IpNet> = iface.ipv4.iter().copied().map(IpNet::V4).collect();
    let mut v6: Vec<IpNet> = iface
        .ipv6
        .iter()
        .copied()
        .zip(iface.ipv6_scope_ids.iter().copied())
        .zip(iface.ipv6_addr_flags.iter().copied())
        .map(|((net, scope_id), flags)| IpNet::V6 {
            net,
            scope_id,
            flags: to_ipv6_addr_flags(flags),
        })
        .collect();

    // Sort each family by address so successive snapshots compare equal, then
    // concatenate as IPv4-first.
    v4.sort_by_key(IpNet::addr);
    v6.sort_by_key(IpNet::addr);
    let mut addrs = v4;
    addrs.append(&mut v6);

    Interface {
        name: iface.name,
        index: iface.index,
        flags: iface.flags,
        mac_addr: iface.mac_addr.as_ref().map(|a| a.octets()),
        addrs,
    }
}

/// Enumerates the machine's network interfaces and assembles the [`State`].
pub(super) async fn get_state() -> State {
    let raw = netdev::interface::get_interfaces();
    let local_addresses = local_addresses(&raw);

    let mut interfaces = std::collections::HashMap::new();
    let mut have_v6 = false;
    let mut have_v4 = false;

    for raw in raw {
        let iface = to_interface(raw);
        if iface.is_up() {
            for pfx in iface.addrs() {
                let addr = pfx.addr();
                if addr.is_loopback() {
                    continue;
                }
                have_v6 |= is_usable_v6(&addr);
                have_v4 |= is_usable_v4(&addr);
            }
        }
        interfaces.insert(iface.name().to_string(), iface);
    }

    let default_route_interface = super::default_route_interface().await;

    State {
        interfaces,
        local_addresses,
        have_v4,
        have_v6,
        is_expensive: false,
        default_route_interface,
        last_unsuspend: None,
    }
}

/// The shared home-router lookup for linux, android and windows.
///
/// BSD platforms do not use this as `netdev` cannot yet determine their default
/// gateway, so they provide their own implementation.
#[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
pub(super) fn home_router() -> Option<super::HomeRouter> {
    let gateway = netdev::get_default_gateway().ok()?;
    let gateway = gateway
        .ipv4
        .iter()
        .copied()
        .map(IpAddr::V4)
        .chain(gateway.ipv6.iter().copied().map(IpAddr::V6))
        .next()?;

    Some(super::HomeRouter {
        gateway,
        my_ip: local_ip(),
    })
}

/// Reports whether `ip` is a usable IPv4 address which should have Internet connectivity.
///
/// Globally routable and private IPv4 addresses are always usable, and link-local
/// 169.254.x.x addresses are in some environments.
fn is_usable_v4(ip: &IpAddr) -> bool {
    if !ip.is_ipv4() || ip.is_loopback() {
        return false;
    }

    true
}

/// Reports whether `ip` is a usable IPv6 address which should have Internet connectivity.
///
/// Globally routable IPv6 addresses are always usable, and Unique Local Addresses
/// (fc00::/7) are in some environments used with address translation.
///
/// We consider all 2000::/3 addresses to be routable, which is the interpretation of
/// <https://www.iana.org/assignments/ipv6-unicast-address-assignments/ipv6-unicast-address-assignments.xhtml>
/// as well.  However this probably includes some addresses which should not be routed,
/// e.g. documentation addresses.  See also
/// <https://doc.rust-lang.org/std/net/struct.Ipv6Addr.html#method.is_global> for an
/// alternative implementation which is both stricter and laxer in some regards.
fn is_usable_v6(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V6(ip) => {
            // V6 Global1 2000::/3
            let mask: u16 = 0b1110_0000_0000_0000;
            let base: u16 = 0x2000;
            let segment1 = ip.segments()[0];
            if (base & mask) == (segment1 & mask) {
                return true;
            }

            is_private_v6(ip)
        }
        IpAddr::V4(_) => false,
    }
}

/// The local IP address of this machine, as reported by `netdev`.
pub(super) fn local_ip() -> Option<IpAddr> {
    netdev::net::ip::get_local_ipaddr()
}

const fn is_up(interface: &netdev::Interface) -> bool {
    interface.flags & IFF_UP != 0
}

const fn is_loopback(interface: &netdev::Interface) -> bool {
    interface.flags & IFF_LOOPBACK != 0
}

/// Builds the machine's [`LocalAddresses`] from a raw `netdev` interface list.
///
/// If there are no regular addresses it falls back to IPv4 link-local or IPv6
/// unique-local addresses, because we know of environments where these are used
/// with NAT to provide connectivity.
fn local_addresses(ifaces: &[netdev::Interface]) -> LocalAddresses {
    let mut loopback = Vec::new();
    let mut regular4 = Vec::new();
    let mut regular6 = Vec::new();
    let mut linklocal4 = Vec::new();
    let mut ula6 = Vec::new();

    for iface in ifaces {
        if !is_up(iface) {
            // Skip down interfaces
            continue;
        }
        let ifc_is_loopback = is_loopback(iface);
        let addrs = iface
            .ipv4
            .iter()
            .map(|a| IpAddr::V4(a.addr()))
            .chain(iface.ipv6.iter().map(|a| IpAddr::V6(a.addr())));

        for ip in addrs {
            let ip = ip.to_canonical();

            if ip.is_loopback() || ifc_is_loopback {
                loopback.push(ip);
            } else if is_link_local(ip) {
                if ip.is_ipv4() {
                    linklocal4.push(ip);
                }

                // We know of no cases where the IPv6 fe80:: addresses
                // are used to provide WAN connectivity. It is also very
                // common for users to have no IPv6 WAN connectivity,
                // but their OS supports IPv6 so they have an fe80::
                // address. We don't want to report all of those
                // IPv6 LL to Control.
            } else if ip.is_ipv6() && is_private(&ip) {
                // Google Cloud Run uses NAT with IPv6 Unique
                // Local Addresses to provide IPv6 connectivity.
                ula6.push(ip);
            } else if ip.is_ipv4() {
                regular4.push(ip);
            } else {
                regular6.push(ip);
            }
        }
    }

    if regular4.is_empty() && regular6.is_empty() {
        // if we have no usable IP addresses then be willing to accept
        // addresses we otherwise wouldn't, like:
        //   + 169.254.x.x (AWS Lambda uses NAT with these)
        //   + IPv6 ULA (Google Cloud Run uses these with address translation)
        regular4 = linklocal4;
        regular6 = ula6;
    }
    let mut regular = regular4;
    regular.extend(regular6);

    regular.sort();
    loopback.sort();

    LocalAddresses { loopback, regular }
}

impl LocalAddresses {
    /// Returns the machine's IP addresses.
    ///
    /// If there are no regular addresses it will return any IPv4 link-local or
    /// IPv6 unique-local addresses, because we know of environments where these
    /// are used with NAT to provide connectivity.
    pub fn new() -> Self {
        local_addresses(&netdev::interface::get_interfaces())
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use super::*;

    #[test]
    fn test_local_addresses() {
        let addrs = LocalAddresses::new();
        dbg!(&addrs);
        assert!(!addrs.loopback.is_empty());
        assert!(!addrs.regular.is_empty());
    }

    #[test]
    fn test_is_usable_v6() {
        let loopback = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0x1);
        assert!(!is_usable_v6(&loopback.into()));

        let link_local = Ipv6Addr::new(0xfe80, 0, 0, 0, 0xcbc9, 0x6aff, 0x5b07, 0x4a9e);
        assert!(!is_usable_v6(&link_local.into()));

        let relay_use1 = Ipv6Addr::new(0x2a01, 0x4ff, 0xf0, 0xc4a1, 0, 0, 0, 0x1);
        assert!(is_usable_v6(&relay_use1.into()));

        let random_2603 = Ipv6Addr::new(0x2603, 0x3ff, 0xf1, 0xc3aa, 0x1, 0x2, 0x3, 0x1);
        assert!(is_usable_v6(&random_2603.into()));
    }
}
