//! System DNS configuration from `/etc/resolv.conf`.

use std::net::{IpAddr, SocketAddr};

use super::{Config, DnsProtocol, Hosts, Nameserver};

/// Reads the nameservers and search domains from `/etc/resolv.conf`.
///
/// Also reads the hosts file.
pub(super) fn read_system_dns() -> Result<Config, std::io::Error> {
    let content = std::fs::read_to_string("/etc/resolv.conf")?;
    let mut config = parse_resolv_conf(&content);
    config.hosts = Hosts::from_system();
    Ok(config)
}

/// Parses resolv.conf content and extracts nameserver addresses and search domains.
///
/// Recognized directives:
/// - `nameserver <ip>` -- adds a DNS nameserver
/// - `search <domain> [<domain> ...]` -- sets the search domain list
/// - `domain <domain>` -- equivalent to a single-entry search list
/// - `options ndots:<n>` -- sets the `ndots` search threshold
///
/// The `search` and `domain` directives are mutually exclusive per resolv.conf(5);
/// the last one seen wins, matching standard resolver behavior.
///
/// `options timeout:`, `options attempts:`, `options rotate`, and `sortlist`
/// are deliberately ignored. iroh wraps every lookup in its own timeout and
/// per-nameserver attempt budget, orders nameservers by measured round-trip
/// time (which subsumes `rotate`), and selects addresses itself (which subsumes
/// `sortlist`), so honoring these would have no observable effect.
fn parse_resolv_conf(content: &str) -> Config {
    let mut servers = Vec::new();
    let mut search_domains = Vec::new();
    let mut ndots = None;

    for line in content.lines() {
        let line = line.trim();
        // Skip comments
        if line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("nameserver") => {
                if let Some(addr_str) = parts.next() {
                    // Try parsing as SocketAddr first (supports custom ports like
                    // 8.8.8.8:5353 or [::1]:5353), then fall back to IpAddr with
                    // the default DNS port. A scoped IPv6 address like
                    // `fe80::1%eth0` carries a zone id the address parsers reject,
                    // so strip it for the fallback and keep the address (the zone
                    // needs interface binding we do not do, so it is discarded).
                    let addr = addr_str.parse::<SocketAddr>().ok().or_else(|| {
                        let ip_str = addr_str.split('%').next().unwrap_or(addr_str);
                        ip_str
                            .parse::<IpAddr>()
                            .ok()
                            .map(|ip| SocketAddr::new(ip, DnsProtocol::Udp.port()))
                    });
                    if let Some(addr) = addr {
                        servers.push(Nameserver::new(addr, DnsProtocol::Udp));
                    }
                }
            }
            Some("search") => {
                // `search` replaces any previous search/domain list. Drop entries
                // that cannot form a valid DNS suffix so a placeholder like the
                // `--` systemd-resolved emits never poisons a search expansion.
                search_domains = parts
                    .filter(|s| is_valid_search_domain(s))
                    .map(|s| s.to_string())
                    .collect();
            }
            Some("domain") => {
                // `domain` is equivalent to a single-entry search list.
                if let Some(domain) = parts.next().filter(|s| is_valid_search_domain(s)) {
                    search_domains = vec![domain.to_string()];
                }
            }
            Some("options") => {
                for opt in parts {
                    if let Some(n) = opt.strip_prefix("ndots:").and_then(|v| v.parse().ok()) {
                        ndots = Some(n);
                    }
                }
            }
            _ => {}
        }
    }

    Config {
        nameservers: servers,
        search_domains,
        ndots,
        hosts: Hosts::default(),
    }
}

/// Returns true if `domain` can serve as a search suffix.
///
/// systemd-resolved writes a `--` placeholder when it has no search domain, and
/// other tooling can leave malformed entries. Anything that fails to parse as a
/// DNS name is rejected so a bad suffix never turns a resolvable short name into
/// a failed lookup.
fn is_valid_search_domain(domain: &str) -> bool {
    domain != "--" && simple_dns::Name::new(domain).is_ok()
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    /// Returns the parsed nameserver addresses, dropping ports and protocols.
    fn ips(config: &Config) -> Vec<IpAddr> {
        config.nameservers.iter().map(|ns| ns.addr.ip()).collect()
    }

    /// Builds an IPv4 [`IpAddr`], to keep the expectations below readable.
    fn ipv4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(a, b, c, d))
    }

    #[test]
    fn parse_basic() {
        let config = parse_resolv_conf("nameserver 8.8.8.8\nnameserver 8.8.4.4\n");
        assert_eq!(ips(&config), [ipv4(8, 8, 8, 8), ipv4(8, 8, 4, 4)]);
        assert!(
            config
                .nameservers
                .iter()
                .all(|ns| ns.protocol == DnsProtocol::Udp)
        );
    }

    #[test]
    fn parse_ipv6() {
        let config = parse_resolv_conf("nameserver 8.8.8.8\nnameserver 2001:4860:4860::8888\n");
        assert_eq!(config.nameservers.len(), 2);
        assert!(config.nameservers[1].addr.ip().is_ipv6());
    }

    #[test]
    fn parse_comments_and_directives() {
        let config = parse_resolv_conf(
            "# comment\n; comment\nsearch example.com\nnameserver 1.1.1.1\noptions ndots:5\nnameserver 1.0.0.1\n",
        );
        assert_eq!(ips(&config), [ipv4(1, 1, 1, 1), ipv4(1, 0, 0, 1)]);
        assert_eq!(config.search_domains, ["example.com"]);
        assert_eq!(config.ndots, Some(5));
    }

    #[test]
    fn parse_ndots() {
        let config = parse_resolv_conf("nameserver 8.8.8.8\noptions ndots:3\n");
        assert_eq!(config.ndots, Some(3));
    }

    #[test]
    fn parse_ndots_default() {
        let config = parse_resolv_conf("nameserver 8.8.8.8\n");
        assert_eq!(config.ndots, None);
    }

    #[test]
    fn parse_skips_invalid_ips() {
        let config = parse_resolv_conf("nameserver not-an-ip\nnameserver 8.8.8.8\n");
        assert_eq!(ips(&config), [ipv4(8, 8, 8, 8)]);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_resolv_conf("").nameservers.is_empty());
    }

    #[test]
    fn parse_no_nameservers() {
        let config = parse_resolv_conf("search example.com\noptions ndots:1\n");
        assert!(config.nameservers.is_empty());
        assert_eq!(config.search_domains, ["example.com"]);
    }

    #[test]
    fn parse_whitespace_variations() {
        let config = parse_resolv_conf("  nameserver   8.8.8.8  \n\tnameserver\t1.1.1.1\t\n");
        assert_eq!(config.nameservers.len(), 2);
    }

    #[test]
    fn parse_inline_comment() {
        let config = parse_resolv_conf("nameserver 8.8.8.8 # primary\nnameserver 1.1.1.1\n");
        assert_eq!(ips(&config), [ipv4(8, 8, 8, 8), ipv4(1, 1, 1, 1)]);
    }

    #[test]
    fn parse_no_space_after_keyword() {
        let config = parse_resolv_conf("nameserver8.8.8.8\nnameserver 1.1.1.1\n");
        assert_eq!(ips(&config), [ipv4(1, 1, 1, 1)]);
    }

    #[test]
    fn parse_scoped_ipv6() {
        // The `%eth0` zone id is stripped and the address retained, matching
        // hickory (the zone is unusable without interface binding, which we do
        // not do).
        let config = parse_resolv_conf("nameserver fe80::1%eth0\nnameserver 8.8.8.8\n");
        assert_eq!(
            ips(&config),
            [IpAddr::V6("fe80::1".parse().unwrap()), ipv4(8, 8, 8, 8),]
        );
    }

    #[test]
    fn parse_search_domains() {
        let config = parse_resolv_conf("search example.com foo.bar\nnameserver 8.8.8.8\n");
        assert_eq!(config.search_domains, ["example.com", "foo.bar"]);
    }

    #[test]
    fn parse_domain_directive() {
        let config = parse_resolv_conf("domain example.com\nnameserver 8.8.8.8\n");
        assert_eq!(config.search_domains, ["example.com"]);
    }

    #[test]
    fn parse_custom_port() {
        let config = parse_resolv_conf("nameserver 8.8.8.8:5353\nnameserver 1.1.1.1\n");
        assert_eq!(config.nameservers.len(), 2);
        assert_eq!(
            config.nameservers[0].addr,
            "8.8.8.8:5353".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(config.nameservers[1].addr.port(), DnsProtocol::Udp.port());
    }

    #[test]
    fn parse_skips_invalid_search() {
        // Mirrors hickory's `test_skips_invalid_search`: the `--` placeholder is
        // dropped while the valid `lan` suffix survives.
        let config =
            parse_resolv_conf("\n\nnameserver 127.0.0.53\noptions edns0 trust-ad\nsearch -- lan\n");
        assert_eq!(ips(&config), [ipv4(127, 0, 0, 53)]);
        assert_eq!(config.search_domains, ["lan"]);
    }

    #[test]
    fn parse_tolerates_unrecognized_options() {
        // A common systemd-resolved options line must not drop the surrounding
        // directives; the unknown flags are ignored but `ndots` and the
        // nameserver still parse.
        let config = parse_resolv_conf(
            "options edns0 trust-ad rotate single-request ndots:2\nnameserver 8.8.8.8\n",
        );
        assert_eq!(ips(&config), [ipv4(8, 8, 8, 8)]);
        assert_eq!(config.ndots, Some(2));
    }

    #[test]
    fn search_overrides_domain() {
        let config =
            parse_resolv_conf("domain old.com\nsearch new.com other.com\nnameserver 8.8.8.8\n");
        // Last directive wins, per resolv.conf(5).
        assert_eq!(config.search_domains, ["new.com", "other.com"]);
    }
}
