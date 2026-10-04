//! IP address related utilities.

use std::net::{IpAddr, Ipv6Addr};

/// List of machine's IP addresses.
///
/// The netdev-based constructors live in [`crate::interfaces`]'s `netdev_impl`
/// module; on platforms without `netdev` this is only ever the empty default.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LocalAddresses {
    /// Loopback addresses.
    pub loopback: Vec<IpAddr>,
    /// Regular addresses.
    pub regular: Vec<IpAddr>,
}

/// Reports whether `ip` is a private address, according to RFC 1918
/// (IPv4 addresses) and RFC 4193 (IPv6 addresses). That is, it reports whether
/// ip is in 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, or fc00::/7.
#[cfg(netdev)]
pub(crate) fn is_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            // RFC 1918 allocates 10.0.0.0/8, 172.16.0.0/12, and 192.168.0.0/16 as
            // private IPv4 address subnets.
            let octets = ip.octets();
            octets[0] == 10
                || (octets[0] == 172 && octets[1] & 0xf0 == 16)
                || (octets[0] == 192 && octets[1] == 168)
        }
        IpAddr::V6(ip) => is_private_v6(ip),
    }
}

#[cfg(netdev)]
pub(crate) fn is_private_v6(ip: &Ipv6Addr) -> bool {
    // RFC 4193 allocates fc00::/7 as the unique local unicast IPv6 address subnet.
    ip.octets()[0] & 0xfe == 0xfc
}

pub(crate) fn is_link_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_link_local(),
        IpAddr::V6(ip) => is_unicast_link_local(ip),
    }
}

/// Returns true if the address is a unicast address with link-local scope, as defined in RFC 4291.
// Copied from std lib, not stable yet
pub const fn is_unicast_link_local(addr: Ipv6Addr) -> bool {
    (addr.segments()[0] & 0xffc0) == 0xfe80
}
