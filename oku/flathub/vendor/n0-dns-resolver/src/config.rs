//! The parsed DNS configuration.
//!
//! [`Config`] is the nameservers plus resolv.conf options the resolver runs
//! on. It is produced by the platform readers in [`crate::system_config`] and
//! consumed by the resolver, so it lives in its own module rather than beside
//! either one.

use crate::{Nameserver, system_config::Hosts};

/// Parsed DNS configuration: the nameservers to query and resolv.conf options.
#[derive(Debug, Clone, Default)]
pub(crate) struct Config {
    /// The nameservers to query, in the order they should be tried.
    pub(crate) nameservers: Vec<Nameserver>,
    /// Search domains from resolv.conf `search` or `domain` directives.
    ///
    /// When resolving a short hostname (one with fewer dots than `ndots`,
    /// default 1), the resolver should try appending each search domain
    /// before querying the bare name.
    pub(crate) search_domains: Vec<String>,
    /// The `ndots` option from resolv.conf.
    ///
    /// Names with at least this many dots are tried as absolute first.
    /// `None` means use the default (1).
    /// See <https://man7.org/linux/man-pages/man5/resolv.conf.5.html>.
    pub(crate) ndots: Option<usize>,
    /// Static name-to-address mappings from the system hosts file.
    ///
    /// Consulted ahead of the cache for A/AAAA lookups. Populated by the
    /// platform readers when the system configuration is read; empty otherwise.
    pub(crate) hosts: Hosts,
}
