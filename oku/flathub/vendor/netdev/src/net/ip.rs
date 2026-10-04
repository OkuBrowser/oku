use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[cfg(feature = "gateway")]
use std::net::{SocketAddr, UdpSocket};

#[cfg(feature = "gateway")]
fn try_ipv4() -> Option<IpAddr> {
    // Attempt to bind a UDP socket to an unspecified address and port.
    let socket = match UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)) {
        Ok(s) => s,
        Err(_) => return None,
    };
    // Attempt to connect the socket to a specific non-routable IP address.
    // This does not send actual data but is used to determine the routing interface.
    match socket.connect(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(10, 254, 254, 254)),
        1,
    )) {
        Ok(()) => (),
        Err(_) => return None,
    };
    // Retrieve and return the local IP address from the socket.
    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip()),
        Err(_) => return None,
    };
}

#[cfg(feature = "gateway")]
fn try_ipv6() -> Option<IpAddr> {
    // same thing but for IPv6
    let socket = match UdpSocket::bind(SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)) {
        Ok(s) => s,
        Err(_) => return None,
    };
    match socket.connect(SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(
            0xfdff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
        )),
        1,
    )) {
        Ok(()) => (),
        Err(_) => return None,
    };
    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip()),
        Err(_) => return None,
    };
}

/// Returns the local IP address selected for outbound traffic.
///
/// This function opens a UDP socket and lets the operating system choose the source address
/// that would be used to reach a non-routable destination. No application data is sent.
///
/// The returned address is useful for identifying the interface currently preferred for the
/// default route.
///
/// Returns `None` when socket creation, address selection, or local address inspection fails.
#[cfg(feature = "gateway")]
pub fn get_local_ipaddr() -> Option<IpAddr> {
    // binding the IPv4 socket can actually succeed but a later step will fail in IPv6-only,
    // we call `try_ipv6` in any case where `try_ipv4` fails
    if let Some(addr) = try_ipv4() {
        Some(addr)
    } else {
        try_ipv6()
    }
}

/// Returns [`true`] if the address appears to be globally routable.
pub(crate) fn is_global_ip(ip_addr: &IpAddr) -> bool {
    match ip_addr {
        IpAddr::V4(ip) => is_global_ipv4(ip),
        IpAddr::V6(ip) => is_global_ipv6(ip),
    }
}

/// Returns [`true`] if the address appears to be globally reachable
/// as specified by the [IANA IPv4 Special-Purpose Address Registry].
pub(crate) fn is_global_ipv4(ipv4_addr: &Ipv4Addr) -> bool {
    !(ipv4_addr.octets()[0] == 0 // "This network"
        || ipv4_addr.is_private()
        || is_shared_ipv4(ipv4_addr)
        || ipv4_addr.is_loopback()
        || ipv4_addr.is_link_local()
        // addresses reserved for future protocols (`192.0.0.0/24`)
        // .9 and .10 are documented as globally reachable so they're excluded
        || (
            ipv4_addr.octets()[0] == 192 && ipv4_addr.octets()[1] == 0 && ipv4_addr.octets()[2] == 0
            && ipv4_addr.octets()[3] != 9 && ipv4_addr.octets()[3] != 10
        )
        || ipv4_addr.is_documentation()
        || is_benchmarking_ipv4(ipv4_addr)
        || is_reserved_ipv4(ipv4_addr)
        || ipv4_addr.is_broadcast())
}

/// Returns [`true`] if the address appears to be globally reachable
/// as specified by the [IANA IPv6 Special-Purpose Address Registry].
pub(crate) fn is_global_ipv6(ipv6_addr: &Ipv6Addr) -> bool {
    !(ipv6_addr.is_unspecified()
        || ipv6_addr.is_loopback()
        // IPv4-mapped Address (`::ffff:0:0/96`)
        || matches!(ipv6_addr.segments(), [0, 0, 0, 0, 0, 0xffff, _, _])
        // IPv4-IPv6 Translat. (`64:ff9b:1::/48`)
        || matches!(ipv6_addr.segments(), [0x64, 0xff9b, 1, _, _, _, _, _])
        // Discard-Only Address Block (`100::/64`)
        || matches!(ipv6_addr.segments(), [0x100, 0, 0, 0, _, _, _, _])
        // IETF Protocol Assignments (`2001::/23`)
        || (matches!(ipv6_addr.segments(), [0x2001, b, _, _, _, _, _, _] if b < 0x200)
            && !(
                // Port Control Protocol Anycast (`2001:1::1`)
                u128::from_be_bytes(ipv6_addr.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0001
                // Traversal Using Relays around NAT Anycast (`2001:1::2`)
                || u128::from_be_bytes(ipv6_addr.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0002
                // AMT (`2001:3::/32`)
                || matches!(ipv6_addr.segments(), [0x2001, 3, _, _, _, _, _, _])
                // AS112-v6 (`2001:4:112::/48`)
                || matches!(ipv6_addr.segments(), [0x2001, 4, 0x112, _, _, _, _, _])
                // ORCHIDv2 (`2001:20::/28`)
                // Drone Remote ID Protocol Entity Tags (DETs) Prefix (`2001:30::/28`)`
                || matches!(ipv6_addr.segments(), [0x2001, b, _, _, _, _, _, _] if b >= 0x20 && b <= 0x3F)
            ))
        // 6to4 (`2002::/16`) - it's not explicitly documented as globally reachable,
        // IANA says N/A.
        || matches!(ipv6_addr.segments(), [0x2002, _, _, _, _, _, _, _])
        || is_documentation_ipv6(ipv6_addr)
        || ipv6_addr.is_unique_local()
        || ipv6_addr.is_unicast_link_local())
}

/// Returns [`true`] if this address is part of the Shared Address Space defined in
/// [IETF RFC 6598] (`100.64.0.0/10`).
///
/// [IETF RFC 6598]: https://tools.ietf.org/html/rfc6598
fn is_shared_ipv4(ipv4_addr: &Ipv4Addr) -> bool {
    ipv4_addr.octets()[0] == 100 && (ipv4_addr.octets()[1] & 0b1100_0000 == 0b0100_0000)
}

/// Returns [`true`] if this address part of the `198.18.0.0/15` range, which is reserved for
/// network devices benchmarking.
fn is_benchmarking_ipv4(ipv4_addr: &Ipv4Addr) -> bool {
    ipv4_addr.octets()[0] == 198 && (ipv4_addr.octets()[1] & 0xfe) == 18
}

/// Returns [`true`] if this address is reserved by IANA for future use.
fn is_reserved_ipv4(ipv4_addr: &Ipv4Addr) -> bool {
    ipv4_addr.octets()[0] & 240 == 240 && !ipv4_addr.is_broadcast()
}

/// Returns [`true`] if this is an address reserved for documentation
/// (`2001:db8::/32` and `3fff::/20`).
fn is_documentation_ipv6(ipv6_addr: &Ipv6Addr) -> bool {
    matches!(
        ipv6_addr.segments(),
        [0x2001, 0xdb8, ..] | [0x3fff, 0..=0x0fff, ..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_is_global_ipv4() {
        let global = Ipv4Addr::new(1, 1, 1, 1); // Cloudflare
        let private = Ipv4Addr::new(192, 168, 1, 1);
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let shared = Ipv4Addr::new(100, 64, 0, 1); // RFC6598
        let doc = Ipv4Addr::new(192, 0, 2, 1); // Documentation

        assert!(is_global_ipv4(&global));
        assert!(!is_global_ipv4(&private));
        assert!(!is_global_ipv4(&loopback));
        assert!(!is_global_ipv4(&shared));
        assert!(!is_global_ipv4(&doc));
    }

    #[test]
    fn test_is_global_ipv6() {
        let global = Ipv6Addr::new(0x2606, 0x4700, 0, 0, 0, 0, 0, 0x1111); // Cloudflare
        let loopback = Ipv6Addr::LOCALHOST;
        let unspecified = Ipv6Addr::UNSPECIFIED;
        let unique_local = Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1);
        let doc = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1); // Documentation

        assert!(is_global_ipv6(&global));
        assert!(!is_global_ipv6(&loopback));
        assert!(!is_global_ipv6(&unspecified));
        assert!(!is_global_ipv6(&unique_local));
        assert!(!is_global_ipv6(&doc));
    }

    #[test]
    fn test_is_global_ip() {
        let ip_v4 = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let ip_v6 = IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0, 0, 0, 0, 0, 0x1111)); // Cloudflare
        let ip_private = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let ip_ula = IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1));

        assert!(is_global_ip(&ip_v4));
        assert!(is_global_ip(&ip_v6));
        assert!(!is_global_ip(&ip_private));
        assert!(!is_global_ip(&ip_ula));
    }
}
