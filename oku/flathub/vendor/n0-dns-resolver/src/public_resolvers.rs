//! The public DNS resolvers offered as ready-made fallback nameservers.
//!
//! [`Provider`] names one public resolver operator and hands out its
//! nameservers, holding no ordering policy beyond "primary address first".
//! [`interleave_nameservers`] merges per-provider lists round-robin.
//! [`default_order`] is this crate's opinionated order for a selection of
//! providers, and the one [`Builder::default_fallback_nameservers`] installs.
//!
//! # Examples
//!
//! ```
//! use n0_dns_resolver::{
//!     DnsProtocol, interleave_nameservers,
//!     public_resolvers::{self, Provider},
//! };
//!
//! // This crate's order for two providers.
//! let servers = public_resolvers::default_order(vec![Provider::Quad9, Provider::Cloudflare]);
//!
//! // Or pick the transport and the order yourself: DNS-over-TLS only.
//! let servers = interleave_nameservers([
//!     Provider::Quad9.nameservers(DnsProtocol::Tls),
//!     Provider::Cloudflare.nameservers(DnsProtocol::Tls),
//! ]);
//! ```
//!
//! [`Builder::default_fallback_nameservers`]: crate::Builder::default_fallback_nameservers

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::{DnsProtocol, Nameserver, interleave_nameservers};

/// A public DNS resolver operator.
///
/// Each provider runs an anycast service on two IPv4 and two IPv6 addresses,
/// reachable over every [`DnsProtocol`] this crate speaks. The encrypted
/// entries are addressed by IP rather than by hostname, which works because all
/// three providers list the anycast addresses used here as `iPAddress` SANs in
/// their certificates. DoT and DoH therefore validate without a server name.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum Provider {
    /// Cloudflare's `1.1.1.1` resolver.
    Cloudflare,
    /// Google Public DNS, `8.8.8.8`.
    Google,
    /// Quad9's `9.9.9.9` resolver.
    Quad9,
}

impl Provider {
    /// Every provider this crate knows about.
    pub const ALL: &'static [Provider] = &[Provider::Cloudflare, Provider::Google, Provider::Quad9];

    /// Returns the provider's IPv4 addresses, primary first.
    fn ipv4_addrs(self) -> [Ipv4Addr; 2] {
        match self {
            Provider::Cloudflare => [Ipv4Addr::new(1, 1, 1, 1), Ipv4Addr::new(1, 0, 0, 1)],
            Provider::Google => [Ipv4Addr::new(8, 8, 8, 8), Ipv4Addr::new(8, 8, 4, 4)],
            Provider::Quad9 => [Ipv4Addr::new(9, 9, 9, 9), Ipv4Addr::new(149, 112, 112, 112)],
        }
    }

    /// Returns the provider's IPv6 addresses, primary first.
    fn ipv6_addrs(self) -> [Ipv6Addr; 2] {
        match self {
            Provider::Cloudflare => [
                Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111),
                Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1001),
            ],
            Provider::Google => [
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888),
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8844),
            ],
            Provider::Quad9 => [
                Ipv6Addr::new(0x2620, 0x00fe, 0, 0, 0, 0, 0, 0x00fe),
                Ipv6Addr::new(0x2620, 0x00fe, 0, 0, 0, 0, 0, 0x0009),
            ],
        }
    }

    /// Returns the provider's nameservers reachable over `protocol`.
    ///
    /// Ordered IPv4 primary, IPv6 primary, IPv4 secondary, IPv6 secondary.
    /// Every provider answers on every protocol this crate speaks, so the list
    /// holds all four of that provider's addresses.
    pub fn nameservers(self, protocol: DnsProtocol) -> Vec<Nameserver> {
        let [v4_primary, v4_secondary] = self.ipv4_addrs().map(IpAddr::V4);
        let [v6_primary, v6_secondary] = self.ipv6_addrs().map(IpAddr::V6);
        [v4_primary, v6_primary, v4_secondary, v6_secondary]
            .into_iter()
            .map(|ip| Nameserver::new(SocketAddr::new(ip, protocol.port()), protocol))
            .collect()
    }
}

/// Returns this crate's default nameserver order for `providers`.
///
/// This is the order [`Builder::default_fallback_nameservers`] installs, over
/// [`Provider::ALL`]. It is one opinion rather than the only sensible one, and
/// the pieces it is assembled from are public: a caller who disagrees can build
/// their own order with [`Provider::nameservers`] and
/// [`interleave_nameservers`].
///
/// Plain DNS across the providers, round-robin, comes first, because on a
/// working network UDP answers in a few milliseconds. On a network that filters
/// port 53 that ordering alone would fail badly. The resolver keeps only three
/// attempts in flight, and a nameserver silently dropping UDP/53 holds its slot
/// for the UDP timeout plus the TCP retry behind it, several seconds in total.
/// Three UDP entries would fill the first raced wave and stall every encrypted
/// attempt behind them. So one DNS-over-HTTPS entry per provider is spliced in
/// after the first two UDP entries, which puts it inside that first wave: on a
/// filtered network DoH is racing within 200ms, and on a working network the
/// UDP entries answer long before the staggered DoH attempts start. From the
/// second lookup onwards the resolver's per-server round-trip tracking takes
/// over, floating whatever works on the current network to the front.
///
/// Duplicate providers are not filtered; pass each one once.
///
/// [`Builder::default_fallback_nameservers`]: crate::Builder::default_fallback_nameservers
pub fn default_order(providers: Vec<Provider>) -> Vec<Nameserver> {
    #[cfg_attr(not(transport_https), allow(unused_mut))]
    let mut servers =
        interleave_nameservers(providers.iter().map(|p| p.nameservers(DnsProtocol::Udp)));
    // One DoH entry per provider, at the provider's primary IPv4 address,
    // placed just inside the first raced wave.
    #[cfg(transport_https)]
    {
        let doh: Vec<Nameserver> = providers
            .iter()
            .filter_map(|p| p.nameservers(DnsProtocol::Https).into_iter().next())
            .collect();
        let at = servers.len().min(2);
        servers.splice(at..at, doh);
    }
    servers
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the default order over every provider, entry by entry.
    ///
    /// This list is the crate's default fallback tier. Its exact order is what
    /// keeps DoH reachable on a network that filters UDP/53 (see
    /// [`default_order`]), and it is easy to perturb by accident when the
    /// assembly changes.
    #[test]
    fn default_order_for_all_providers() {
        let actual: Vec<String> = default_order(Provider::ALL.to_vec())
            .iter()
            .map(|ns| format!("{} {:?}", ns.addr(), ns.protocol()))
            .collect();
        let expected = [
            "1.1.1.1:53 Udp",
            "8.8.8.8:53 Udp",
            #[cfg(transport_https)]
            "1.1.1.1:443 Https",
            #[cfg(transport_https)]
            "8.8.8.8:443 Https",
            #[cfg(transport_https)]
            "9.9.9.9:443 Https",
            "9.9.9.9:53 Udp",
            "[2606:4700:4700::1111]:53 Udp",
            "[2001:4860:4860::8888]:53 Udp",
            "[2620:fe::fe]:53 Udp",
            "1.0.0.1:53 Udp",
            "8.8.4.4:53 Udp",
            "149.112.112.112:53 Udp",
            "[2606:4700:4700::1001]:53 Udp",
            "[2001:4860:4860::8844]:53 Udp",
            "[2620:fe::9]:53 Udp",
        ];
        assert_eq!(actual, expected);
    }

    /// Checks that a one-provider selection is self-contained.
    ///
    /// It must still yield all four of that provider's addresses, and nothing
    /// from any other provider.
    #[test]
    fn single_provider_selection_is_self_contained() {
        let servers = default_order(vec![Provider::Quad9]);
        let ips: Vec<IpAddr> = servers.iter().map(|ns| ns.addr().ip()).collect();
        for addr in Provider::Quad9.ipv4_addrs() {
            assert!(ips.contains(&IpAddr::V4(addr)));
        }
        for addr in Provider::Quad9.ipv6_addrs() {
            assert!(ips.contains(&IpAddr::V6(addr)));
        }
        assert!(!ips.contains(&IpAddr::V4(Provider::Google.ipv4_addrs()[0])));
    }

    /// Checks that every protocol yields four entries per provider.
    ///
    /// Each entry must sit on one of that provider's own addresses, at the port
    /// the protocol is served on.
    #[test]
    fn nameservers_per_protocol_cover_all_addresses() {
        let protocols = [
            DnsProtocol::Udp,
            DnsProtocol::Tcp,
            #[cfg(transport_tls)]
            DnsProtocol::Tls,
            #[cfg(transport_https)]
            DnsProtocol::Https,
        ];
        for &provider in Provider::ALL {
            for protocol in protocols {
                let servers = provider.nameservers(protocol);
                assert_eq!(servers.len(), 4, "{provider:?} {protocol:?}");
                assert!(servers.iter().all(|ns| ns.addr().port() == protocol.port()));
                assert!(servers.iter().all(|ns| ns.protocol() == protocol));
                assert_eq!(servers[0].addr().ip(), IpAddr::V4(provider.ipv4_addrs()[0]));
            }
        }
    }

    /// Checks that DoH lands in the first raced wave, for any selection size.
    ///
    /// The wave is `MAX_CONCURRENT_QUERIES` (3) entries wide. If no DoH entry
    /// falls inside it, a network that silently drops UDP/53 stalls DoH behind
    /// several seconds of UDP timeout. Callers pick their own providers, so
    /// this has to hold for every selection size, not just for all three.
    #[cfg(transport_https)]
    #[test]
    fn doh_lands_in_first_wave() {
        for count in 1..=Provider::ALL.len() {
            let servers = default_order(Provider::ALL[..count].to_vec());
            let wave = &servers[..3.min(servers.len())];
            assert!(
                wave.iter().any(|ns| ns.protocol() == DnsProtocol::Https),
                "expected a DoH entry within the first 3 of {count} providers, got {wave:?}",
            );
        }
    }
}
