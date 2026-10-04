//! The [`Builder`] for a [`DnsResolver`], and the types it configures.

use std::{net::SocketAddr, time::Duration};

use crate::{DnsResolver, public_resolvers};

/// A builder for a [`DnsResolver`].
///
/// A fresh builder is empty: it queries no nameservers and reads nothing from
/// the host, so every source is opted into explicitly. Get one from
/// [`DnsResolver::builder`], add the sources you want, then call
/// [`Builder::build`]. For the common case, the system configuration with the
/// public resolvers behind it, [`DnsResolver::system_with_fallback`] does the
/// assembly for you.
///
/// Nameservers form two tiers, both empty to start. The *primary* tier is what
/// [`Builder::use_system_config`], [`Builder::nameserver`] and
/// [`Builder::nameservers`] add. The *fallback* tier is what
/// [`Builder::fallback_nameservers`] and
/// [`Builder::default_fallback_nameservers`] add. [`Builder::fallback_mode`]
/// decides how the two relate: by default the fallback tier is queried only
/// once the primary tier cannot answer.
///
/// # Examples
///
/// ```
/// use n0_dns_resolver::{
///     DnsResolver,
///     public_resolvers::{self, Provider},
/// };
///
/// // The system configuration, with Quad9 behind it as a fallback.
/// let resolver = DnsResolver::builder()
///     .use_system_config()
///     .fallback_nameservers(public_resolvers::default_order(vec![Provider::Quad9]))
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct Builder {
    /// Whether to read the host's DNS configuration into the primary tier.
    pub(crate) use_system_config: bool,
    /// Explicitly added primary-tier nameservers, ahead of any from the system.
    pub(crate) nameservers: Vec<Nameserver>,
    /// The fallback tier, empty unless the caller added to it.
    pub(crate) fallback_nameservers: Vec<Nameserver>,
    /// How the fallback tier relates to the primary one.
    pub(crate) fallback: FallbackMode,
    /// Serve-stale window, or `None` to disable serving expired answers.
    pub(crate) serve_stale: Option<Duration>,
    /// Floor for cached positive TTLs, or `None` to keep the server's TTL.
    pub(crate) cache_min_ttl: Option<Duration>,
    /// Cap on how long NXDOMAIN and NODATA are cached, or `None` to disable.
    pub(crate) negative_max_ttl: Option<Duration>,
    /// Caller-supplied TLS settings for DoT and DoH.
    ///
    /// `None` falls back to a config built from the compiled-in crypto
    /// provider, and failing that those transports error.
    #[cfg(with_rustls)]
    pub(crate) tls_client_config: Option<rustls::ClientConfig>,
}

impl Builder {
    /// Reads the host system's DNS configuration into the primary tier.
    ///
    /// The system's nameservers join the primary tier, and the resolver honors
    /// the system's `search` domains and `ndots` setting and consults the
    /// system hosts file. Without this, nothing is read from the host.
    #[must_use]
    pub fn use_system_config(mut self) -> Self {
        self.use_system_config = true;
        self
    }

    /// Adds a primary nameserver.
    ///
    /// Build one with [`Nameserver::new`], or with
    /// [`Nameserver::with_server_name`] for a DoT/DoH server whose certificate
    /// covers a hostname rather than its IP.
    #[must_use]
    pub fn nameserver(mut self, nameserver: Nameserver) -> Self {
        self.nameservers.push(nameserver);
        self
    }

    /// Adds several primary nameservers.
    ///
    /// Appends, so it can be called repeatedly. It takes the same
    /// [`Nameserver`] list as [`Self::fallback_nameservers`], so a selection of
    /// public resolvers can serve as the primary tier just as well as the
    /// fallback one.
    #[must_use]
    pub fn nameservers(mut self, nameservers: impl IntoIterator<Item = Nameserver>) -> Self {
        self.nameservers.extend(nameservers);
        self
    }

    /// Sets how the fallback nameservers relate to the primary ones.
    ///
    /// The default is [`FallbackMode::Deferred`]: the fallback tier waits for
    /// the primary one to fail. See [`FallbackMode`] for the alternatives. Has
    /// no effect while the fallback tier is empty.
    #[must_use]
    pub fn fallback_mode(mut self, mode: FallbackMode) -> Self {
        self.fallback = mode;
        self
    }

    /// Adds `nameservers` to the fallback tier.
    ///
    /// Appends, so it can be called repeatedly and combines with
    /// [`Self::default_fallback_nameservers`]. Build the list for a selection
    /// of public resolvers with [`public_resolvers::default_order`], or
    /// assemble your own from [`Provider::nameservers`] and
    /// [`interleave_nameservers`].
    ///
    /// [`Provider::nameservers`]: public_resolvers::Provider::nameservers
    #[must_use]
    pub fn fallback_nameservers(
        mut self,
        nameservers: impl IntoIterator<Item = Nameserver>,
    ) -> Self {
        self.fallback_nameservers.extend(nameservers);
        self
    }

    /// Adds the crate's default public resolvers to the fallback tier.
    ///
    /// Shorthand for [`Self::fallback_nameservers`] with
    /// [`public_resolvers::default_order`] over [`Provider::ALL`]: Cloudflare,
    /// Google and Quad9, reached over UDP and DNS-over-HTTPS on both address
    /// families. That function documents the order and the reasoning for it.
    ///
    /// [`Provider::ALL`]: public_resolvers::Provider::ALL
    #[must_use]
    pub fn default_fallback_nameservers(self) -> Self {
        self.fallback_nameservers(public_resolvers::default_order(
            public_resolvers::Provider::ALL.to_vec(),
        ))
    }

    /// Sets a custom TLS client config for DNS-over-TLS and DNS-over-HTTPS.
    ///
    /// Requires the `transport-tls` or `transport-https` feature. Without a
    /// config, DoT/DoH use one built from the crypto provider (`tls-ring` or
    /// `tls-aws-lc-rs`); with neither a config nor a provider, they error.
    #[cfg(with_rustls)]
    #[must_use]
    pub fn tls_client_config(mut self, config: rustls::ClientConfig) -> Self {
        self.tls_client_config = Some(config);
        self
    }

    /// Serves an expired cached answer when live resolution fails.
    ///
    /// When every nameserver fails or times out, a positive answer that expired
    /// no more than `max_age` ago is returned instead of an error, so a brief
    /// upstream outage does not break resolution. This is serve-stale, RFC
    /// 8767. Only positive answers are served stale; an authoritative NXDOMAIN
    /// is never overridden. Off by default.
    #[must_use]
    pub fn serve_stale(mut self, max_age: Duration) -> Self {
        self.serve_stale = Some(max_age);
        self
    }

    /// Floors every cached positive time-to-live (TTL) to at least `min_ttl`.
    ///
    /// Absorbs bursts of lookups for records with very low (or zero) TTLs by
    /// holding them for at least this long, at the cost of serving a slightly
    /// staler answer. Off by default, since it trades freshness for fewer
    /// queries; leave it unset for records that change frequently.
    #[must_use]
    pub fn cache_min_ttl(mut self, min_ttl: Duration) -> Self {
        self.cache_min_ttl = Some(min_ttl);
        self
    }

    /// Caps how long NXDOMAIN and NODATA answers are cached.
    ///
    /// Off by default, so a first NXDOMAIN does not hide a name that starts
    /// resolving a moment later. Pass a positive duration to cache negatives,
    /// capped by this value and by the response's SOA (RFC 2308).
    #[must_use]
    pub fn negative_max_ttl(mut self, max_ttl: Duration) -> Self {
        self.negative_max_ttl = Some(max_ttl);
        self
    }

    /// Consumes the builder and returns the configured [`DnsResolver`].
    #[must_use]
    pub fn build(self) -> DnsResolver {
        DnsResolver::from_builder(self)
    }
}

/// How the resolver uses its fallback nameservers relative to the primary ones.
///
/// The primary nameservers come from [`Builder::use_system_config`] and
/// [`Builder::nameserver`]; the fallback nameservers from
/// [`Builder::fallback_nameservers`]. Set the mode with
/// [`Builder::fallback_mode`]. An empty fallback tier is unaffected by the
/// mode: to query no fallback at all, add none.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum FallbackMode {
    /// Races the fallback nameservers alongside the primary ones from the start.
    Eager,
    /// Uses the fallback nameservers only when the system configuration is empty.
    ///
    /// A system configuration counts as empty when it yields no nameservers,
    /// whether because it could not be read or because it listed none. One that
    /// did yield nameservers is never supplemented: if those fail at query time
    /// the lookup fails rather than escalating. Without
    /// [`Builder::use_system_config`] there is no configuration at all, which
    /// also counts as empty, so this behaves like [`Self::Eager`].
    IfSystemEmpty,
    /// Queries the fallback nameservers only after every primary one has failed.
    ///
    /// This is the default. The fallback stays a lower-priority second tier
    /// rather than joining the initial race, so a working primary nameserver
    /// always answers first.
    #[default]
    Deferred,
}

/// Merges nameserver lists round-robin.
///
/// Takes every list's first entry, then every list's second, and so on,
/// skipping lists that have run out. Nameserver order is query order, so
/// interleaving per-provider lists spreads the early attempts across providers
/// instead of exhausting one provider before trying the next. No other policy
/// is applied. For this crate's opinionated order over the public resolvers,
/// see [`public_resolvers::default_order`].
///
/// # Examples
///
/// ```
/// use n0_dns_resolver::{DnsProtocol, interleave_nameservers, public_resolvers::Provider};
///
/// // DNS-over-TLS to Quad9 and Cloudflare, alternating between them.
/// let servers = interleave_nameservers([
///     Provider::Quad9.nameservers(DnsProtocol::Tls),
///     Provider::Cloudflare.nameservers(DnsProtocol::Tls),
/// ]);
/// ```
pub fn interleave_nameservers<L>(lists: impl IntoIterator<Item = L>) -> Vec<Nameserver>
where
    L: IntoIterator<Item = Nameserver>,
{
    // Fused so that a list which has run dry stays dry, whatever the caller's
    // iterator does after returning `None`.
    let mut lists: Vec<_> = lists.into_iter().map(|l| l.into_iter().fuse()).collect();
    // Each round takes one entry from every list that still has one, so the
    // rounds shrink as lists run out and an empty round ends the merge.
    std::iter::repeat_with(|| {
        lists
            .iter_mut()
            .filter_map(Iterator::next)
            .collect::<Vec<_>>()
    })
    .take_while(|round| !round.is_empty())
    .flatten()
    .collect()
}

/// A nameserver to query: an address, a transport, and an optional TLS name.
///
/// The connection is always made to the address. When a TLS server name is set
/// it drives the SNI and certificate validation, and serves as the DoH URL
/// authority with the address pinned. Otherwise DoT and DoH are addressed by
/// IP.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Nameserver {
    /// The address to connect to. Always an IP, never resolved as a name.
    pub(crate) addr: SocketAddr,
    /// The transport to query over, which also fixes the default port.
    pub(crate) protocol: DnsProtocol,
    /// The name to validate the TLS certificate against, if not the IP.
    ///
    /// Only used for DoT/DoH (the `transport-tls` or `transport-https` feature).
    #[cfg(any(with_rustls, doc))]
    pub(crate) server_name: Option<String>,
}

impl Nameserver {
    /// Creates a nameserver addressed by IP, with no TLS server name.
    pub const fn new(addr: SocketAddr, protocol: DnsProtocol) -> Self {
        Self {
            addr,
            protocol,
            #[cfg(any(with_rustls, doc))]
            server_name: None,
        }
    }

    /// Returns the address this nameserver is queried at.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns the transport this nameserver is queried over.
    pub fn protocol(&self) -> DnsProtocol {
        self.protocol
    }

    /// Returns the TLS server name, or `None` when addressed by IP.
    ///
    /// Set only by [`Self::with_server_name`], and used only for DNS-over-TLS
    /// and DNS-over-HTTPS.
    #[cfg(any(with_rustls, doc))]
    pub fn server_name(&self) -> Option<&str> {
        self.server_name.as_deref()
    }

    /// Creates a DoT or DoH nameserver validated against `server_name`.
    ///
    /// The connection is made to `addr`, while `server_name` drives the TLS SNI
    /// and certificate validation. Use this for providers whose certificates
    /// cover a hostname rather than the IP address.
    #[cfg(any(with_rustls, doc))]
    pub fn with_server_name(
        addr: SocketAddr,
        protocol: DnsProtocol,
        server_name: impl Into<String>,
    ) -> Self {
        Self {
            addr,
            protocol,
            server_name: Some(server_name.into()),
        }
    }
}

/// A protocol over which DNS records can be resolved.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum DnsProtocol {
    /// DNS over UDP.
    ///
    /// The classic DNS transport, supported by essentially every DNS server.
    #[default]
    Udp,
    /// DNS over TCP.
    ///
    /// Specified in the original DNS RFCs, but not supported by every server.
    Tcp,
    /// DNS over TLS (DoT), as defined in [RFC 7858].
    ///
    /// Runs the DNS protocol over a TLS-encrypted TCP connection.
    ///
    /// [RFC 7858]: https://www.rfc-editor.org/rfc/rfc7858.html
    #[cfg(transport_tls)]
    Tls,
    /// DNS over HTTPS (DoH), as defined in [RFC 8484].
    ///
    /// Carries DNS messages inside HTTPS requests.
    ///
    /// [RFC 8484]: https://www.rfc-editor.org/rfc/rfc8484.html
    #[cfg(transport_https)]
    Https,
}

impl DnsProtocol {
    /// Returns the port this protocol is served on by default.
    ///
    /// Plain DNS over UDP or TCP uses 53, DNS-over-TLS uses 853, and
    /// DNS-over-HTTPS uses 443. A [`Nameserver`] carries its own port, which
    /// need not be the default; this is what to reach for when all you have is
    /// an IP address.
    pub const fn port(self) -> u16 {
        match self {
            // Do53, the classic DNS port.
            DnsProtocol::Udp | DnsProtocol::Tcp => 53,
            // DoT, RFC 7858.
            #[cfg(transport_tls)]
            DnsProtocol::Tls => 853,
            // DoH, RFC 8484.
            #[cfg(transport_https)]
            DnsProtocol::Https => 443,
        }
    }
}
