//! Static name-to-address mappings from the system hosts file.
//!
//! On Unix this is `/etc/hosts`, on Windows the per-system hosts file under
//! `%SystemRoot%`. Entries are consulted ahead of the cache and the network so
//! that an operator can pin a relay or discovery origin to a fixed address, the
//! way the old hickory-backed resolver did via its `use_hosts_file` default.

use std::{
    borrow::Cow,
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use rustc_hash::FxBuildHasher;
use tracing::warn;

/// A hosts-file map, keyed by name.
///
/// Hashes with FxHash rather than the standard library's SipHash. That trades
/// HashDoS resistance for speed, which is sound here because the only thing
/// ever hashed into these maps is a hosts file written by the machine's
/// administrator. Nothing from the network reaches these keys.
type NameMap<T> = HashMap<Box<str>, Addrs<T>, FxBuildHasher>;

/// The addresses of one family mapped to a single name.
///
/// Effectively a `Vec` that holds its first element inline. Hosts files are
/// overwhelmingly one address per name per family, so a `Vec` here would mean
/// one heap allocation per name. That is the single largest cost of parsing a
/// large hosts file: the ad-blocking lists people install run to a few hundred
/// thousand names, and storing the first address inline cuts the parse roughly
/// in half.
#[derive(Debug, Default, Clone)]
enum Addrs<T> {
    #[default]
    None,
    One(T),
    Many(Vec<T>),
}

impl<T: Copy> Addrs<T> {
    /// Appends `addr`, promoting to a heap vector on the second address.
    fn push(&mut self, addr: T) {
        *self = match self {
            Addrs::None => Addrs::One(addr),
            Addrs::One(first) => Addrs::Many(vec![*first, addr]),
            Addrs::Many(addrs) => {
                addrs.push(addr);
                return;
            }
        };
    }

    /// Returns the addresses, or `None` when this name has none of this family.
    fn to_vec(&self) -> Option<Vec<T>> {
        match self {
            Addrs::None => None,
            Addrs::One(addr) => Some(vec![*addr]),
            Addrs::Many(addrs) => Some(addrs.clone()),
        }
    }
}

/// Static host-to-address mappings parsed from the system hosts file.
#[derive(Debug, Default, Clone)]
pub(crate) struct Hosts {
    /// Names with an IPv4 address, pre-sized on parse since most lines are A.
    a: NameMap<Ipv4Addr>,
    /// Names with an IPv6 address.
    ///
    /// Kept apart from `a` so an A-only name costs nothing here.
    aaaa: NameMap<Ipv6Addr>,
}

impl Hosts {
    /// Reads and parses the system hosts file.
    ///
    /// Returns an empty mapping when the file is missing, unreadable, or the
    /// platform has no hosts file, so a missing file is never an error.
    pub(crate) fn from_system() -> Self {
        match hosts_path().and_then(|path| std::fs::read_to_string(path).ok()) {
            Some(content) => Self::parse(&content),
            None => Self::default(),
        }
    }

    /// Parses hosts-file content into a name-to-address mapping.
    ///
    /// Each non-comment line has the form `address host [host ...]`. Names are
    /// lowercased; comments (`#` to end of line) and unparsable addresses are
    /// skipped.
    ///
    /// Tokenizing is ASCII-only. A hosts file holds IP addresses and DNS names,
    /// both of which are ASCII, so the Unicode-aware splitters would only spend
    /// time classifying bytes that cannot appear.
    fn parse(content: &str) -> Self {
        // Only the A map is pre-sized, so that a large file does not rehash its
        // way up from nothing. Hosts files are overwhelmingly IPv4, and lines
        // average well under this many bytes, so the estimate over-reserves
        // slightly rather than growing repeatedly. An IPv6-heavy file simply
        // grows its map as it goes.
        const BYTES_PER_ENTRY: usize = 32;
        let mut a: NameMap<Ipv4Addr> =
            NameMap::with_capacity_and_hasher(content.len() / BYTES_PER_ENTRY, FxBuildHasher);
        let mut aaaa: NameMap<Ipv6Addr> = NameMap::default();
        // An editor may save the hosts file with a UTF-8 byte order mark (BOM),
        // which would otherwise fuse onto the first address token and make it
        // unparsable, dropping the first entry.
        let content = content.strip_prefix('\u{feff}').unwrap_or(content);
        for line in content.lines() {
            let line = match line.split_once('#') {
                Some((before, _)) => before,
                None => line,
            }
            .trim_ascii();
            if line.is_empty() {
                continue;
            }
            let mut fields = line.split_ascii_whitespace();
            let Some(addr) = fields.next() else {
                continue;
            };
            let Ok(addr) = addr.parse::<IpAddr>() else {
                warn!(%addr, "ignoring unparsable address in hosts file");
                continue;
            };
            for name in fields {
                let key = name.to_ascii_lowercase().into_boxed_str();
                match addr {
                    IpAddr::V4(ip) => a.entry(key).or_default().push(ip),
                    IpAddr::V6(ip) => aaaa.entry(key).or_default().push(ip),
                }
            }
        }
        Self { a, aaaa }
    }

    /// Normalizes a query name to the hosts-file key form.
    ///
    /// The key form is the name lowercased, with any trailing dot removed. A
    /// name that is already in that form is borrowed rather than copied, since
    /// every A and AAAA lookup passes through here and query names are
    /// overwhelmingly lowercase already.
    fn normalize(name: &str) -> Cow<'_, str> {
        let name = name.strip_suffix('.').unwrap_or(name);
        match name.bytes().any(|byte| byte.is_ascii_uppercase()) {
            true => Cow::Owned(name.to_ascii_lowercase()),
            false => Cow::Borrowed(name),
        }
    }

    /// Returns the mapped IPv4 addresses for `name`, if any.
    pub(crate) fn lookup_ipv4(&self, name: &str) -> Option<Vec<Ipv4Addr>> {
        self.a.get(Self::normalize(name).as_ref())?.to_vec()
    }

    /// Returns the mapped IPv6 addresses for `name`, if any.
    pub(crate) fn lookup_ipv6(&self, name: &str) -> Option<Vec<Ipv6Addr>> {
        self.aaaa.get(Self::normalize(name).as_ref())?.to_vec()
    }

    /// Builds a hosts map directly from file content, for tests.
    #[cfg(test)]
    pub(crate) fn from_content(content: &str) -> Self {
        Self::parse(content)
    }
}

#[cfg(unix)]
fn hosts_path() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from("/etc/hosts"))
}

#[cfg(windows)]
fn hosts_path() -> Option<std::path::PathBuf> {
    let system_root = std::env::var_os("SystemRoot")?;
    Some(std::path::Path::new(&system_root).join("System32\\drivers\\etc\\hosts"))
}

#[cfg(not(any(unix, windows)))]
fn hosts_path() -> Option<std::path::PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_and_lookup() {
        let hosts = Hosts::parse(
            "127.0.0.1 localhost\n10.0.1.10 myrelay.test relay\n::1 localhost myrelay.test\n",
        );
        assert_eq!(
            hosts.lookup_ipv4("myrelay.test"),
            Some(vec![Ipv4Addr::new(10, 0, 1, 10)])
        );
        // Alias on the same line resolves too.
        assert_eq!(
            hosts.lookup_ipv4("relay"),
            Some(vec![Ipv4Addr::new(10, 0, 1, 10)])
        );
        assert_eq!(
            hosts.lookup_ipv6("myrelay.test"),
            Some(vec![Ipv6Addr::LOCALHOST])
        );
        // No AAAA entry for a name that only has an A record.
        assert_eq!(hosts.lookup_ipv6("relay"), None);
    }

    #[test]
    fn lookup_is_case_insensitive_and_fqdn_tolerant() {
        let hosts = Hosts::parse("10.0.1.10 MyRelay.Test\n");
        assert_eq!(
            hosts.lookup_ipv4("myrelay.test."),
            Some(vec![Ipv4Addr::new(10, 0, 1, 10)])
        );
    }

    #[test]
    fn parse_skips_comments_and_garbage() {
        let hosts = Hosts::parse(
            "# a comment\n\n  10.0.1.10  host1  # trailing comment\nnot-an-ip host2\n",
        );
        assert_eq!(
            hosts.lookup_ipv4("host1"),
            Some(vec![Ipv4Addr::new(10, 0, 1, 10)])
        );
        assert_eq!(hosts.lookup_ipv4("host2"), None);
    }

    #[test]
    fn multiple_addresses_accumulate() {
        // Three addresses, so the third exercises the push onto an already
        // promoted list rather than only the promotion itself.
        let hosts = Hosts::parse("10.0.0.1 host\n10.0.0.2 host\n10.0.0.3 host\n");
        assert_eq!(
            hosts.lookup_ipv4("host"),
            Some(vec![
                Ipv4Addr::new(10, 0, 0, 1),
                Ipv4Addr::new(10, 0, 0, 2),
                Ipv4Addr::new(10, 0, 0, 3)
            ])
        );
    }

    #[test]
    fn parse_strips_leading_bom() {
        // A hosts file saved with a UTF-8 BOM must still resolve its first entry.
        let hosts = Hosts::parse("\u{feff}10.0.1.10 myrelay.test\n");
        assert_eq!(
            hosts.lookup_ipv4("myrelay.test"),
            Some(vec![Ipv4Addr::new(10, 0, 1, 10)])
        );
    }

    /// Many distinct names must all round-trip through the map.
    ///
    /// Guards the non-default hasher: one that collided badly would still
    /// return correct answers, so only a test with enough keys to fill several
    /// tables would notice a problem with it.
    #[test]
    fn many_names_all_resolve() {
        let content: String = (0..5_000)
            .map(|i| format!("10.0.{}.{} host{i}.example\n", i / 256, i % 256))
            .collect();
        let hosts = Hosts::parse(&content);
        for i in 0..5_000u32 {
            assert_eq!(
                hosts.lookup_ipv4(&format!("host{i}.example")),
                Some(vec![Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8)]),
                "host{i}.example"
            );
        }
        assert_eq!(hosts.lookup_ipv4("host5000.example"), None);
    }

    #[test]
    fn empty_lookup_returns_none() {
        let hosts = Hosts::default();
        assert_eq!(hosts.lookup_ipv4("anything"), None);
    }
}
