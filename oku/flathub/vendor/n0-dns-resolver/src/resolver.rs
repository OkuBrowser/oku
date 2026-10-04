//! The resolver itself.
//!
//! Builds and parses packets with `simple-dns` and moves them with tokio. The
//! submodules hold the pieces: the cache, the connection pool, the per-server
//! RTT map, packet construction and parsing, and the transports.

#[cfg(with_rustls)]
use std::sync::Arc;
#[cfg(transport_https)]
use std::sync::Mutex;
use std::{
    future::Future,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::OnceLock,
};

use n0_error::e;
use n0_future::{
    FuturesUnordered, MaybeFuture, StreamExt,
    time::{self, Duration, Instant},
};
use simple_dns::TYPE;
use tracing::{debug, trace};

#[cfg(test)]
use crate::system_config::Hosts;
use crate::{
    Builder, DnsProtocol, Error, FallbackMode, HttpsRecordData, MxRecordData, Nameserver, Record,
    RecordKind, SrvRecordData, SvcbRecordData, TxtRecordData, config::Config, system_config,
};

mod cache;
mod pool;
mod query;
mod rtt_map;
mod transport;

pub use self::transport::TransportError;
use self::{
    cache::{CachedResult, DnsCache, NEGATIVE_TTL_MAX_SECS, NEGATIVE_TTL_SECS},
    pool::ConnPool,
    query::{MAX_CNAME_DEPTH, QueryError},
    rtt_map::RttMap,
};

impl RecordKind {
    /// Maps this kind onto the `simple_dns` query type used on the wire.
    fn dns_type(self) -> TYPE {
        match self {
            RecordKind::A => TYPE::A,
            RecordKind::Aaaa => TYPE::AAAA,
            RecordKind::Txt => TYPE::TXT,
            RecordKind::Ns => TYPE::NS,
            RecordKind::Srv => TYPE::SRV,
            RecordKind::Mx => TYPE::MX,
            RecordKind::Caa => TYPE::CAA,
            RecordKind::Svcb => TYPE::SVCB,
            RecordKind::Https => TYPE::HTTPS,
        }
    }
}

/// Maps a transport-layer failure onto the public [`Error`].
impl From<TransportError> for Error {
    fn from(source: TransportError) -> Self {
        e!(Error::Transport { source })
    }
}

/// Maps a query build or response-parse failure onto the public [`Error`].
impl From<QueryError> for Error {
    fn from(err: QueryError) -> Self {
        match err {
            QueryError::BuildQuery { name, .. } => e!(Error::InvalidName { name }),
            QueryError::Malformed { .. } | QueryError::Unexpected { .. } => {
                e!(Error::InvalidResponse)
            }
            QueryError::NxDomain { .. } => e!(Error::NxDomain),
            QueryError::ServerFailure { rcode, .. } => {
                e!(Error::ServerError {
                    code: query::response_code(rcode)
                })
            }
        }
    }
}

/// Total time one nameserver gets for a query sent over UDP.
///
/// Covers every datagram [`DnsResolver::udp_query`] sends plus the TCP query
/// that joins them, which is what lets a retransmit go out while the previous
/// one is still outstanding. Leaves TCP roughly [`STREAM_TIMEOUT`] once started.
const UDP_NAMESERVER_TIMEOUT: Duration = Duration::from_secs(6);

/// Per-attempt timeout for a connection-oriented query (TCP, DoT, DoH).
///
/// Covers connection setup and, for DoT/DoH, the TLS handshake on top of the
/// round trip. Matches the ~5s used by glibc, Go, hickory-resolver, and
/// systemd-resolved.
const STREAM_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum number of nameserver queries in flight at once.
///
/// Keeps a long nameserver list from turning every lookup into an N-way fan-out.
const MAX_CONCURRENT_QUERIES: usize = 3;

/// Delay before starting the next nameserver attempt.
///
/// Gives faster servers a head start without blasting the whole list at once,
/// happy-eyeballs style. An attempt that fails starts the next one at once.
const QUERY_ATTEMPT_DELAY: Duration = Duration::from_millis(100);

/// Number of UDP datagrams sent to one nameserver for a single query.
///
/// The first [`UDP_PACED_DATAGRAMS`] go out at the interval
/// [`DnsResolver::retransmit_interval`] gives, the rest at double the one
/// before, so the last lands around five seconds in. Stopping after the paced
/// run left UDP silent for most of [`UDP_NAMESERVER_TIMEOUT`], resting on a TCP
/// query that on a lossy link is the weaker of the two. The later datagrams cost
/// nothing when they are not needed, since an answer over either transport drops
/// the whole set.
const UDP_DATAGRAMS: usize = 6;

/// Number of UDP datagrams sent before the interval starts doubling.
///
/// Matches hickory-resolver's `max_retries`. Datagrams overlap rather than run
/// in sequence: each goes out while the earlier ones are still outstanding, and
/// any may carry the answer.
const UDP_PACED_DATAGRAMS: usize = 3;

/// Multiplier on a nameserver's datagram round trip to get its retransmit
/// interval.
///
/// A server that answers in 400ms is not late until well past 400ms, so scaling
/// off the measurement keeps us from retransmitting into a merely slow link.
/// hickory-resolver uses 1.2; the wider margin is a judgement call, and below
/// [`UDP_RETRANSMIT_MIN`] the floor governs either way.
const UDP_RETRANSMIT_SRTT_FACTOR: f64 = 1.5;

/// Lower bound on the UDP retransmit interval.
///
/// Below this the retransmit races the answer rather than replacing a lost
/// datagram, costing every healthy lookup an extra packet. Sits between
/// c-ares's 250ms and hickory-resolver's 333ms.
const UDP_RETRANSMIT_MIN: Duration = Duration::from_millis(300);

/// Upper bound on the UDP retransmit interval.
///
/// Caps how long a badly degraded nameserver can stall the next datagram.
const UDP_RETRANSMIT_MAX: Duration = Duration::from_secs(1);

/// How long a nameserver may leave UDP unanswered before the query also goes
/// out over TCP.
///
/// [`UDP_RETRANSMIT_MIN`] times [`UDP_PACED_DATAGRAMS`], so at the floor the
/// paced run has gone out with an interval to spare and a link that merely
/// loses datagrams never pays for a connection.
///
/// Fixed, where the retransmit interval is measured. A delay derived from how
/// long this server takes would grow with the fallback it exists to reach: the
/// server answers over TCP, that answer is what the measurement sees, and the
/// switch slips further on every lookup.
const TCP_JOIN_DELAY: Duration = UDP_RETRANSMIT_MIN.saturating_mul(UDP_PACED_DATAGRAMS as u32);

/// Default value for `ndots` per resolv.conf(5).
///
/// Names with at least this many dots are tried as absolute names first,
/// before appending search domains. Names with fewer dots try search
/// domains first. See <https://man7.org/linux/man-pages/man5/resolv.conf.5.html>.
const DEFAULT_NDOTS: usize = 1;

/// Returns whether `host` is `localhost` or a name under it.
///
/// RFC 6761 Section 6.3 reserves these to resolve to loopback without a query.
/// DNS names are case-insensitive, so `foo.LOCALHOST` is one of them too, and
/// must not go out to a nameserver that could answer it with any address.
fn is_localhost(host: &str) -> bool {
    let host = host.strip_suffix('.').unwrap_or(host);
    host.rsplit('.')
        .next()
        .is_some_and(|label| label.eq_ignore_ascii_case("localhost"))
}

/// A stub DNS resolver over UDP/TCP (and, with a crypto provider, DoT/DoH).
///
/// See the [crate] docs for an overview. Construct one with
/// [`Self::system_with_fallback`] for cross-platform defaults, or with
/// [`Self::builder`] to configure the nameservers and the fallback behavior.
///
/// # Examples
///
/// ```no_run
/// # async fn run() -> Result<(), n0_dns_resolver::Error> {
/// use n0_dns_resolver::DnsResolver;
///
/// let resolver = DnsResolver::system_with_fallback();
/// for addr in resolver.lookup_ipv4("example.com").await? {
///     println!("{addr}");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct DnsResolver {
    /// TLS settings for DoT and DoH, from the builder or the crypto provider.
    ///
    /// `None` when neither supplied one, in which case those transports fail
    /// with [`Error::MissingTlsConfig`] rather than falling back to a default.
    #[cfg(with_rustls)]
    tls_config: Option<Arc<rustls::ClientConfig>>,
    /// Lazily initialized, cached reqwest client for DNS-over-HTTPS queries.
    #[cfg(transport_https)]
    https_client: Mutex<Option<reqwest::Client>>,
    /// Pooled TCP/DoT connections, reused across queries.
    conn_pool: ConnPool,
    /// Cached positive and negative answers.
    ///
    /// Shared with the resolver [`Self::reset`] produces, so a network change
    /// does not start DNS cold.
    cache: DnsCache,
    /// The settings this resolver was built from.
    ///
    /// Kept so [`Self::reset`] can rebuild against a changed network, and used
    /// to compute `state`.
    builder: Builder,
    /// Config-derived state, built on first use.
    ///
    /// Deferring it keeps construction and [`Self::reset`] free of IO. The
    /// system DNS read blocks on some platforms, and a network change calls
    /// reset, so it waits for the first lookup rather than running eagerly.
    state: OnceLock<ResolverState>,
}

/// What a resolver derives from its builder and the system configuration.
///
/// Built lazily by [`DnsResolver::state`].
#[derive(Debug)]
struct ResolverState {
    /// The effective configuration this resolver runs on.
    ///
    /// Holds the assembled nameserver list, the search list, `ndots`, and the
    /// hosts file. `config.nameservers` is the primary tier followed by the
    /// fallback one, split at `primary_count`; see [`DnsResolver::send_query`]
    /// for how the two are used.
    config: Config,
    /// Number of leading `config.nameservers` entries forming the primary tier.
    primary_count: usize,
    /// Smoothed round-trip times per nameserver.
    ///
    /// Indexed in parallel to `config.nameservers`. Orders servers
    /// fastest-first, re-probes ones that were demoted, and paces UDP
    /// retransmits.
    rtt_map: RttMap,
}

impl ResolverState {
    /// Assembles the effective configuration and the RTT map.
    ///
    /// Reads the system DNS configuration first when the builder enabled it.
    ///
    /// This is the IO-performing part of building a resolver: the platform
    /// readers read the nameservers, search list, and hosts file.
    fn build(builder: &Builder) -> Self {
        // Start from the system configuration (when enabled). It also carries
        // the search list and hosts file, which we keep as-is; a failed read
        // yields an empty configuration so the fallback can take over.
        let mut config = if builder.use_system_config {
            system_config::read_system()
        } else {
            Config::default()
        };
        // Primary tier: the system nameservers plus any explicitly configured
        // ones.
        let system_has_nameservers = !config.nameservers.is_empty();
        config
            .nameservers
            .extend(builder.nameservers.iter().cloned());

        // Fallback tier: whether to include the configured fallback
        // nameservers, and whether they defer behind the primary tier or race
        // alongside it, depends on the mode. `defer` marks them as a
        // lower-priority second tier; otherwise they merge into the primary tier
        // and are raced from the start.
        let fallback_servers = || builder.fallback_nameservers.clone();
        let (mut fallback, defer) = match builder.fallback {
            FallbackMode::Eager => (fallback_servers(), false),
            FallbackMode::Deferred => (fallback_servers(), true),
            FallbackMode::IfSystemEmpty if system_has_nameservers => (Vec::new(), false),
            FallbackMode::IfSystemEmpty => (fallback_servers(), false),
        };
        let primary_count = if defer {
            config.nameservers.len()
        } else {
            config.nameservers.len() + fallback.len()
        };
        config.nameservers.append(&mut fallback);

        debug!(
            nameservers = ?config.nameservers,
            primary_count,
            search_domains = ?config.search_domains,
            ndots = ?config.ndots,
            "configured DNS resolver"
        );
        let rtt_map = RttMap::new(config.nameservers.len());
        Self {
            config,
            primary_count,
            rtt_map,
        }
    }
}

impl DnsResolver {
    /// Looks up records of `kind` for `name`.
    pub fn lookup_record(
        &self,
        name: impl Into<String>,
        kind: RecordKind,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Record>, Error>> + Send + '_>> {
        // Type-erase the large lookup future here. Downstream adapters may box
        // a typed lookup again, and retaining the complete concrete future can
        // exhaust rustc's trait solver while it proves the outer future is Send.
        Box::pin(self.lookup_record_impl(name.into(), kind))
    }

    /// Creates a resolver on the system configuration, backed by public resolvers.
    ///
    /// This is the cross-platform default. The host's nameservers, search
    /// domains, `ndots` and hosts file are used first, and the public resolvers
    /// (Cloudflare, Google, Quad9) are queried only when the system
    /// configuration cannot be read or its nameservers do not answer. It is
    /// equivalent to
    /// `DnsResolver::builder().use_system_config().default_fallback_nameservers().build()`.
    ///
    /// Every other configuration goes through [`Self::builder`].
    pub fn system_with_fallback() -> Self {
        Self::builder()
            .use_system_config()
            .default_fallback_nameservers()
            .build()
    }

    /// Returns an empty [`Builder`] for configuring a resolver.
    ///
    /// The builder reads nothing from the host and queries nothing until
    /// nameservers are added. See [`Builder`] for the available settings.
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Builds a resolver from `builder`, used by [`Builder::build`].
    pub(crate) fn from_builder(builder: Builder) -> Self {
        Self::new_deferred(builder, DnsCache::new())
    }

    /// Builds a resolver that reads the system DNS configuration lazily.
    ///
    /// Only cheap, IO-free state is set up here; the nameserver list, search
    /// list, RTT map, and hosts file are built on first use by [`Self::state`].
    /// [`Self::reset`] reuses this to rebuild after a network change while
    /// carrying the cache across, so lookups keep hitting cached records while
    /// the new nameservers settle (see issue #4037), without any eager IO.
    fn new_deferred(builder: Builder, cache: DnsCache) -> Self {
        // Use the caller's TLS client config, or fall back to one built from the
        // compiled-in crypto provider. When neither is present, DoT/DoH fail with
        // `MissingTlsConfig` rather than reaching for a reqwest/rustls default.
        #[cfg(with_rustls)]
        let tls_config = builder
            .tls_client_config
            .as_ref()
            .map(|config| Arc::new(config.clone()))
            .or_else(Self::default_tls_config);
        Self {
            #[cfg(with_rustls)]
            tls_config,
            #[cfg(transport_https)]
            https_client: Mutex::new(None),
            conn_pool: ConnPool::new(),
            cache,
            builder,
            state: OnceLock::new(),
        }
    }

    /// Builds a default TLS client config from the compiled-in crypto provider.
    ///
    /// Prefers ring over aws-lc-rs when both features are on, and trusts the
    /// webpki roots.
    #[cfg(all(with_rustls, with_crypto_provider))]
    fn default_tls_config() -> Option<Arc<rustls::ClientConfig>> {
        #[cfg(feature = "tls-ring")]
        let provider = rustls::crypto::ring::default_provider();
        #[cfg(all(feature = "tls-aws-lc-rs", not(feature = "tls-ring")))]
        let provider = rustls::crypto::aws_lc_rs::default_provider();
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()
            .expect("crypto provider supports the default protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth();
        Some(Arc::new(config))
    }

    /// Returns no default TLS client config, for builds without a crypto provider.
    ///
    /// DoT and DoH then need one from [`Builder::tls_client_config`], and fail
    /// with [`Error::MissingTlsConfig`] without it.
    #[cfg(all(with_rustls, not(with_crypto_provider)))]
    fn default_tls_config() -> Option<Arc<rustls::ClientConfig>> {
        None
    }

    /// Returns the runtime state, building it on first call.
    ///
    /// The first call reads the system DNS configuration, which on some
    /// platforms blocks; every later call reuses the result.
    fn state(&self) -> &ResolverState {
        self.state
            .get_or_init(|| ResolverState::build(&self.builder))
    }

    /// Returns the candidate names to try for `host`.
    ///
    /// Applies search domain expansion per resolv.conf(5) semantics.
    ///
    /// - If the name ends with `.` (FQDN), it is used as-is.
    /// - If the name has more labels than `ndots`, try the bare name first,
    ///   then each search domain suffix.
    /// - Otherwise, try each search domain suffix first, then the bare name.
    ///
    /// See <https://man7.org/linux/man-pages/man5/resolv.conf.5.html>.
    fn search_names(&self, host: &str) -> Vec<String> {
        let config = &self.state().config;
        // Explicit FQDN: no search domain expansion.
        if host.ends_with('.') || config.search_domains.is_empty() {
            return vec![host.to_string()];
        }

        // Label count = dots + 1 (e.g. "foo.bar" has 2 labels).
        // resolv.conf(5): "if the name has more dots than ndots, try as absolute first"
        // which is equivalent to num_labels > ndots.
        let num_labels = host.bytes().filter(|&b| b == b'.').count() + 1;
        let bare_first = num_labels > config.ndots.unwrap_or(DEFAULT_NDOTS);

        let mut names: Vec<String> = Vec::with_capacity(config.search_domains.len() + 1);

        /// Appends a candidate name unless the list already holds it.
        fn push(names: &mut Vec<String>, name: String) {
            if !names.contains(&name) {
                names.push(name);
            }
        }

        if bare_first {
            push(&mut names, host.to_string());
        }
        for domain in &config.search_domains {
            let expanded = format!("{host}.{domain}");
            // Drop an expansion that cannot form a valid DNS name, such as an
            // over-long suffix or a `--` placeholder from systemd-resolved. One
            // bad search entry must not abort the lookup by producing an
            // InvalidQuery that the search loop treats as fatal; the bare name is
            // always kept so an invalid host still surfaces its own error.
            if simple_dns::Name::new(&expanded).is_ok() {
                push(&mut names, expanded);
            }
        }
        if !bare_first {
            push(&mut names, host.to_string());
        }

        names
    }

    /// Returns a clone of the cached reqwest client, creating it on first use.
    ///
    /// `reqwest::Client` uses an inner `Arc`, so cloning is cheap.
    #[cfg(transport_https)]
    fn get_or_init_https_client(&self) -> Result<reqwest::Client, Error> {
        let mut guard = self.https_client.lock().expect("poisoned");
        match guard.as_ref() {
            Some(client) => Ok(client.clone()),
            None => {
                // DoH needs a TLS client config, like the DoT path. Without one, reqwest
                // (built with `rustls-no-provider`) would fall back to a process-default
                // crypto provider and panic when none is installed.
                let tls_config = self
                    .tls_config
                    .as_ref()
                    .ok_or_else(|| e!(Error::MissingTlsConfig))?;
                // Pin each named DoH server to its address so reqwest does not
                // recursively resolve the hostname.
                let resolves: Vec<(String, std::net::SocketAddr)> = self
                    .state()
                    .config
                    .nameservers
                    .iter()
                    .filter(|ns| ns.protocol == DnsProtocol::Https)
                    .filter_map(|ns| ns.server_name.clone().map(|name| (name, ns.addr)))
                    .collect();
                let client = transport::build_https_client(tls_config, &resolves)?;
                *guard = Some(client.clone());
                Ok(client)
            }
        }
    }

    /// The configured cache min-TTL floor in seconds, or 0 when unset.
    fn cache_min_ttl_secs(&self) -> u32 {
        self.builder
            .cache_min_ttl
            .map_or(0, |d| u32::try_from(d.as_secs()).unwrap_or(u32::MAX))
    }

    /// Returns how long to cache a NODATA or NXDOMAIN answer, in seconds.
    ///
    /// Zero means do not cache it, which is what an unset
    /// [`Builder::negative_max_ttl`] and an indeterminate failure both yield.
    fn negative_cache_ttl_secs(&self, soa_ttl: Option<u32>) -> u32 {
        let Some(max) = self.builder.negative_max_ttl else {
            return 0;
        };
        let max_secs = u32::try_from(max.as_secs())
            .unwrap_or(u32::MAX)
            .min(NEGATIVE_TTL_MAX_SECS);
        if max_secs == 0 {
            return 0;
        }
        soa_ttl.unwrap_or(NEGATIVE_TTL_SECS).min(max_secs)
    }

    /// Runs a transport future with a `timeout`, mapping both failures to
    /// [`Error`].
    async fn with_timeout<T>(
        timeout: Duration,
        fut: impl Future<Output = Result<T, TransportError>>,
    ) -> Result<T, Error> {
        time::timeout(timeout, fut)
            .await
            .map(|r| r.map_err(Error::from))
            .map_err(|_| e!(Error::Timeout))?
    }

    /// Query a single nameserver, retrying without EDNS on a FORMERR.
    ///
    /// `retransmit` is how long to wait between UDP datagrams, from
    /// [`Self::retransmit_interval`]; it is ignored for the other transports.
    ///
    /// Returns the response, and the round trip of the datagram that carried it
    /// when one did. See [`Self::query_nameserver_once`].
    ///
    /// A FORMERR response often means the server or a middlebox rejected our
    /// EDNS(0) OPT record, so retry the same server once without it (RFC 6891
    /// Section 6.2.2) before letting the caller move on. If the query carries no
    /// OPT, or the stripped retry still fails, the response is returned as-is and
    /// the race treats a lingering FORMERR like any other retryable failure.
    async fn query_nameserver(
        &self,
        ns: &Nameserver,
        query_bytes: &[u8],
        retransmit: Duration,
    ) -> Result<(Vec<u8>, Option<Duration>), Error> {
        let (resp, datagram_rtt) = self
            .query_nameserver_once(ns, query_bytes, retransmit)
            .await?;
        if query::is_format_error(&resp)
            && let Some(stripped) = query::strip_edns(query_bytes)
        {
            debug!(addr = %ns.addr, "FORMERR with EDNS, retrying without OPT");
            return self.query_nameserver_once(ns, &stripped, retransmit).await;
        }
        Ok((resp, datagram_rtt))
    }

    /// Query a single nameserver once over its configured transport.
    ///
    /// Returns the response, and the round trip of the datagram that carried it
    /// when one did. That sample paces the next lookup, so it is narrow on
    /// purpose: only a datagram that came back can say how long a datagram
    /// takes. What the attempt cost is the caller's to measure, for ordering.
    async fn query_nameserver_once(
        &self,
        ns: &Nameserver,
        query_bytes: &[u8],
        retransmit: Duration,
    ) -> Result<(Vec<u8>, Option<Duration>), Error> {
        let addr = ns.addr;
        // Only UDP has a datagram round trip to report; the rest send one query.
        let resp = match ns.protocol {
            DnsProtocol::Udp => {
                return time::timeout(
                    UDP_NAMESERVER_TIMEOUT,
                    self.udp_query(addr, query_bytes, retransmit),
                )
                .await
                .map_err(|_| e!(Error::Timeout))?;
            }
            DnsProtocol::Tcp => {
                Self::with_timeout(
                    STREAM_TIMEOUT,
                    transport::tcp_query(&self.conn_pool, addr, query_bytes),
                )
                .await?
            }
            #[cfg(transport_tls)]
            DnsProtocol::Tls => {
                let tls_config = self
                    .tls_config
                    .as_ref()
                    .ok_or_else(|| e!(Error::MissingTlsConfig))?;
                Self::with_timeout(
                    STREAM_TIMEOUT,
                    transport::tls_query(
                        &self.conn_pool,
                        addr,
                        query_bytes,
                        tls_config,
                        ns.server_name.as_deref(),
                    ),
                )
                .await?
            }
            #[cfg(transport_https)]
            DnsProtocol::Https => {
                let client = self.get_or_init_https_client()?;
                Self::with_timeout(
                    STREAM_TIMEOUT,
                    transport::https_query(addr, ns.server_name.as_deref(), query_bytes, &client),
                )
                .await?
            }
        };
        Ok((resp, None))
    }

    /// Queries a nameserver over UDP, overlapping retransmits and then TCP.
    ///
    /// Sends up to [`UDP_DATAGRAMS`] datagrams, each from its own socket and all
    /// left outstanding, so one that never arrives costs an interval rather than
    /// a timeout. After [`TCP_JOIN_DELAY`] unanswered a TCP query joins them,
    /// which is how a lookup survives a network that drops outbound UDP/53 but
    /// permits TCP/53. Whichever answers first wins, and a truncated response
    /// brings TCP in early since the full answer only fits there. Duplicates are
    /// safe: every datagram carries the same transaction id from a fresh source
    /// port, and the caller validates the id, the QR bit, and the question.
    ///
    /// Returns the response, and how long the datagram that carried it was in
    /// flight, timed from its own send. `None` when TCP answered instead. That
    /// sample paces the next lookup, so it must not contain the intervals this
    /// one waited: a sample that did would stretch the next interval, which
    /// would stretch the next sample.
    ///
    /// Has no internal deadline; the caller bounds it with
    /// [`UDP_NAMESERVER_TIMEOUT`].
    async fn udp_query(
        &self,
        addr: SocketAddr,
        query_bytes: &[u8],
        retransmit: Duration,
    ) -> Result<(Vec<u8>, Option<Duration>), Error> {
        // Outstanding datagrams, each carrying the instant it went out. Dropping
        // the set on return closes their sockets.
        let mut datagrams = FuturesUnordered::new();
        // Cancelling the TCP query is safe: the pool hands out a connection by
        // removing it and only takes it back on success, so a dropped query
        // closes its connection rather than returning a half-read one.
        let tcp = MaybeFuture::None;
        tokio::pin!(tcp);
        let next_send = MaybeFuture::None;
        tokio::pin!(next_send);
        let tcp_due = MaybeFuture::Some(time::sleep(TCP_JOIN_DELAY));
        tokio::pin!(tcp_due);
        let mut sends_left = UDP_DATAGRAMS;
        let mut interval = retransmit;
        // Only the pacing releases a datagram, or a send that failed outright. A
        // wakeup from the TCP side must not spend one early.
        let mut send_due = true;
        let mut last_err = None;

        loop {
            if send_due && sends_left > 0 {
                let nth = UDP_DATAGRAMS - sends_left;
                trace!(%addr, datagram = nth, ?interval, "sending UDP query");
                let sent = Instant::now();
                datagrams
                    .push(async move { (sent, transport::udp_query(addr, query_bytes).await) });
                sends_left -= 1;
                if sends_left > 0 {
                    if UDP_DATAGRAMS - sends_left >= UDP_PACED_DATAGRAMS {
                        interval *= 2;
                    }
                    next_send.as_mut().set_future(time::sleep(interval));
                } else {
                    // A timer left running here would hold the loop open past
                    // the last failure.
                    next_send.as_mut().set_none();
                }
            }
            send_due = false;

            // Nothing outstanding and nothing left to send, so the rest of the
            // delay cannot produce an answer. This is a server that refuses UDP
            // rather than dropping it, or an interface with no route to it.
            if datagrams.is_empty() && sends_left == 0 && tcp_due.is_some() {
                tcp_due.as_mut().set_future(time::sleep(Duration::ZERO));
            }

            tokio::select! {
                biased;
                // A datagram came back.
                Some((sent, res)) = datagrams.next(), if !datagrams.is_empty() => match res {
                    Ok((resp, maybe_truncated)) if maybe_truncated || query::is_truncated(&resp) => {
                        debug!(%addr, "UDP response truncated, fetching the answer over TCP");
                        // The answer does not fit in a datagram: stop sending
                        // them and bring TCP forward.
                        sends_left = 0;
                        next_send.as_mut().set_none();
                        if tcp.is_none() {
                            tcp_due.as_mut().set_future(time::sleep(Duration::ZERO));
                        }
                    }
                    Ok((resp, _)) => return Ok((resp, Some(sent.elapsed()))),
                    Err(err) => {
                        trace!(%addr, %err, "UDP query failed");
                        last_err = Some(Error::from(err));
                        // Nothing left in flight to wait for, so replace it now.
                        send_due = true;
                    }
                },
                // The TCP query came back.
                res = &mut tcp, if tcp.is_some() => match res {
                    Ok(resp) => return Ok((resp, None)),
                    Err(err) => {
                        debug!(%addr, %err, "TCP query failed");
                        last_err = Some(Error::from(err));
                    }
                },
                // Time to send the next datagram.
                () = &mut next_send, if next_send.is_some() => send_due = true,
                // Time to bring TCP in.
                () = &mut tcp_due, if tcp_due.is_some() => {
                    // The datagrams may be in flight rather than lost, so they
                    // keep their sockets and TCP races them.
                    debug!(%addr, "UDP unanswered, joining with TCP");
                    tcp.as_mut().set_future(transport::tcp_query(
                        &self.conn_pool,
                        addr,
                        query_bytes,
                    ));
                },
                // Nothing left in flight and nothing left to start.
                else => return Err(last_err.unwrap_or_else(|| e!(Error::NoResponse))),
            }
        }
    }

    /// Returns how long to wait between UDP datagrams to a nameserver.
    ///
    /// `datagram_micros` is one datagram exchange with that server, as
    /// [`RttMap::get_datagram`] reports it, so a slow link gets proportionally
    /// longer before we send again.
    fn retransmit_interval(datagram_rtt: Duration) -> Duration {
        datagram_rtt
            .mul_f64(UDP_RETRANSMIT_SRTT_FACTOR)
            .clamp(UDP_RETRANSMIT_MIN, UDP_RETRANSMIT_MAX)
    }

    /// Returns the given nameserver indices ordered fastest-first by smoothed RTT.
    fn order_indices(&self, indices: &[usize]) -> Vec<usize> {
        let rtt_map = &self.state().rtt_map;
        let mut order = indices.to_vec();
        order.sort_by(|&a, &b| rtt_map.get_decayed(a).total_cmp(&rtt_map.get_decayed(b)));
        order
    }

    /// Sends a query, trying the primary nameservers before the fallback tier.
    ///
    /// The fallback tier is reached only once every primary nameserver has
    /// failed or timed out.
    ///
    /// The two tiers are the leading `primary_count` entries of `nameservers`
    /// and the rest. Only [`FallbackMode::Deferred`] with a non-empty fallback
    /// tier produces a second tier; otherwise
    /// `primary_count == nameservers.len()`, so no escalation happens. When the primary tier is empty (for example the system
    /// configuration could not be read), escalation makes the fallback tier the
    /// effective primary.
    async fn send_query(&self, query_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let state = self.state();
        if state.config.nameservers.is_empty() {
            return Err(e!(Error::NoNameservers));
        }

        let primary: Vec<usize> = (0..state.primary_count).collect();
        match self.race(&primary, query_bytes).await {
            Ok(resp) => Ok(resp),
            Err(primary_err) => {
                if state.primary_count == state.config.nameservers.len() {
                    return Err(primary_err);
                }
                debug!(err = %primary_err, "primary nameservers failed, escalating to fallback");
                let fallback: Vec<usize> =
                    (state.primary_count..state.config.nameservers.len()).collect();
                self.race(&fallback, query_bytes).await
            }
        }
    }

    /// Races the nameservers named by `indices` happy-eyeballs style.
    ///
    /// The historically fastest server goes first. The next attempt starts
    /// either [`QUERY_ATTEMPT_DELAY`] later or as soon as the in-flight one
    /// fails, whichever comes first, and in-flight attempts are capped at
    /// [`MAX_CONCURRENT_QUERIES`].
    ///
    /// The first successful response wins. Within each attempt a UDP query
    /// overlaps its own retransmits, paced by how long that server's datagrams
    /// take to come back (see [`Self::udp_query`]).
    ///
    /// A completed attempt updates both of the server's estimates, so the list
    /// is self-healing and the pacing adapts with it. Ordering measures the whole
    /// attempt, retransmit intervals and TCP fallback included, since that is
    /// what the server costs us. Pacing sees only a datagram's flight time.
    async fn race(&self, indices: &[usize], query_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let state = self.state();
        let order = self.order_indices(indices);
        // Index into `order` of the next nameserver to try.
        let mut next = 0;
        // In-flight attempts, each yielding (nameserver index, start, result).
        let mut dials = FuturesUnordered::new();
        let mut last_err = None;
        // Timer after which to start the next attempt, or `None` for immediately.
        let next_attempt = MaybeFuture::None;
        tokio::pin!(next_attempt);

        loop {
            // Start the next attempt if one is due (no pending delay), we are
            // under the concurrency cap, and a nameserver remains.
            if next_attempt.is_none() && dials.len() < MAX_CONCURRENT_QUERIES && next < order.len()
            {
                let idx = order[next];
                next += 1;
                let start = Instant::now();
                let retransmit = Self::retransmit_interval(state.rtt_map.get_datagram(idx));
                dials.push(async move {
                    let ns = &state.config.nameservers[idx];
                    let res = self.query_nameserver(ns, query_bytes, retransmit).await;
                    (idx, start, res)
                });
                // Pace the following attempt, unless this was the last server.
                if next < order.len() {
                    next_attempt
                        .as_mut()
                        .set_future(time::sleep(QUERY_ATTEMPT_DELAY));
                }
            }

            if dials.is_empty() && next >= order.len() {
                return Err(last_err.unwrap_or_else(|| e!(Error::NoResponse)));
            }

            tokio::select! {
                biased;
                // A dial attempt completed.
                Some((idx, start, res)) = dials.next(), if !dials.is_empty() => match res {
                    Ok((resp, datagram_rtt)) => {
                        // A SERVFAIL, REFUSED, or FORMERR response means this server
                        // will not answer for the name (overloaded, not authoritative,
                        // policy block, or it rejected the query even without EDNS).
                        // Treat it like a transport failure and race the next server
                        // rather than making it the final answer; another nameserver
                        // may still resolve the name.
                        if let Some(rcode) = query::retryable_failure_rcode(&resp) {
                            state.rtt_map.record_failure(idx);
                            last_err = Some(e!(Error::ServerError {
                                code: query::response_code(rcode),
                            }));
                            // Fail fast: start the next attempt now rather than waiting.
                            next_attempt.as_mut().set_none();
                        } else {
                            state
                                .rtt_map
                                .record_success(idx, start.elapsed(), datagram_rtt);
                            return Ok(resp);
                        }
                    }
                    Err(e) => {
                        state.rtt_map.record_failure(idx);
                        last_err = Some(e);
                        // Fail fast: start the next attempt now rather than waiting.
                        next_attempt.as_mut().set_none();
                    }
                },
                // The next attempt is due.
                () = &mut next_attempt, if next_attempt.is_some() => {
                    next_attempt.as_mut().set_none();
                }
            }
        }
    }

    /// Sends a query, following CNAME chains across responses.
    ///
    /// A nameserver that answers with a CNAME but no records of the requested
    /// type has left the chain unresolved, so the target is queried in turn.
    async fn send_query_following_cnames(
        &self,
        host: String,
        qtype: TYPE,
    ) -> Result<Vec<u8>, Error> {
        let mut current_host = host;
        for _ in 0..MAX_CNAME_DEPTH {
            let name = simple_dns::Name::new(&current_host).map_err(|_| {
                e!(Error::InvalidName {
                    name: current_host.clone()
                })
            })?;
            let (id, query_bytes) = query::build_query(&current_host, qtype)?;
            let response = self.send_query(&query_bytes).await?;
            let packet = query::parse_packet(&response)?;

            // Validate the id, QR bit, question, and RCODE before trusting the
            // packet to decide the answer or the next CNAME target. This is the
            // only check of the response against the name we actually asked for.
            query::check_response(&packet, id, &name, qtype)?;

            // The response holds the answer, or has no CNAME to follow.
            let Some(target) = query::unresolved_cname_target(&packet, &name, qtype) else {
                return Ok(response);
            };
            debug!(from = %current_host, to = %target, "following CNAME");
            current_host = target;
        }
        Err(e!(Error::InvalidResponse))
    }

    /// Looks up the records of `kind` for `name`, following CNAME chains.
    ///
    /// This is the one generic lookup path. It checks the cache, expands search
    /// domains, races the nameservers, parses the response into [`Record`]s of
    /// the requested [`RecordKind`], and caches a positive result. Negative
    /// answers are not cached unless [`Builder::negative_max_ttl`] was set. The
    /// typed methods ([`Self::lookup_ipv4`], [`Self::lookup_ipv6`],
    /// [`Self::lookup_txt`]) are thin wrappers over it.
    ///
    /// Unlike the typed methods, this does not apply the RFC 6761 `localhost`
    /// rule or the hosts-file override; those are specific to A and AAAA lookups
    /// and live in [`Self::lookup_ipv4`] and [`Self::lookup_ipv6`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::NxDomain`] when the name does not exist, and the other
    /// [`Error`] variants when every nameserver fails to answer.
    async fn lookup_record_impl(
        &self,
        name: String,
        kind: RecordKind,
    ) -> Result<Vec<Record>, Error> {
        match self.cache.get(&name, kind) {
            Some(CachedResult::Positive(records)) => {
                trace!(%name, records = records.len(), ?kind, "cache hit");
                return Ok(records);
            }
            Some(CachedResult::NoData) => {
                trace!(%name, ?kind, "cache hit (NODATA)");
                return Ok(Vec::new());
            }
            Some(CachedResult::NxDomain) => {
                trace!(%name, ?kind, "cache hit (NXDOMAIN)");
                return Err(e!(Error::NxDomain));
            }
            None => {}
        }

        let mut last_err = None;
        // The first authoritative negative answer in search order, if any. A
        // positive answer returns immediately, so this only ever records the
        // earliest NODATA or NXDOMAIN a candidate produced. Honoring search
        // order keeps an appended candidate's NODATA from masking an NXDOMAIN
        // of the intended (bare) name.
        let mut first_negative: Option<CachedResult> = None;
        // The negative-cache TTL for `first_negative`, derived from that
        // response's authority SOA (RFC 2308) and capped, or the fixed fallback.
        let mut negative_ttl_secs = NEGATIVE_TTL_SECS;
        // Set when any candidate returned an indeterminate failure (SERVFAIL or
        // the like). We never learned whether that name exists, so a later
        // candidate's negative must not be cached: a flaky nameserver would
        // otherwise pin a live name as absent for the negative TTL.
        let mut saw_transient = false;
        let names = self.search_names(&name);
        let total = names.len();
        for (i, name) in names.into_iter().enumerate() {
            trace!(%name, ?kind, "resolving");
            let (res, soa_negative_ttl) = match self
                .send_query_following_cnames(name.clone(), kind.dns_type())
                .await
            {
                Ok(response) => {
                    let parsed = query::parse_records(&response, kind).map_err(Error::from);
                    // Derive the RFC 2308 negative TTL from the authority SOA while
                    // the response bytes are still in scope; only meaningful for a
                    // negative answer (empty or NXDOMAIN).
                    let soa = match &parsed {
                        Ok((records, _)) if records.is_empty() => query::negative_ttl(&response),
                        Err(Error::NxDomain { .. }) => query::negative_ttl(&response),
                        _ => None,
                    };
                    (parsed, soa)
                }
                Err(e) => (Err(e), None),
            };
            match res {
                Ok((results, ttl)) if !results.is_empty() => {
                    let ttl = ttl.max(self.cache_min_ttl_secs());
                    debug!(%name, ?kind, ?results, ttl, "resolved");
                    self.cache
                        .insert(&name, kind, CachedResult::Positive(results.clone()), ttl);
                    return Ok(results);
                }
                // A successful but empty answer is NODATA: the name exists but
                // has no records of this kind.
                Ok(_) => {
                    if first_negative.is_none() {
                        first_negative = Some(CachedResult::NoData);
                        negative_ttl_secs = self.negative_cache_ttl_secs(soa_negative_ttl);
                    }
                }
                Err(e @ Error::NxDomain { .. }) => {
                    let remaining = total - i - 1;
                    trace!(%name, ?kind, remaining, reason = %e, "lookup failed");
                    if first_negative.is_none() {
                        first_negative = Some(CachedResult::NxDomain);
                        negative_ttl_secs = self.negative_cache_ttl_secs(soa_negative_ttl);
                    }
                    last_err = Some(e);
                }
                // An indeterminate failure: we never learned whether this name
                // exists, so try the next search candidate. Mark it so a later
                // candidate's negative is not cached, since a flaky nameserver
                // must not pin a name that a retry could still resolve.
                Err(
                    e @ (Error::ServerError { .. }
                    | Error::Timeout { .. }
                    | Error::NoResponse { .. }
                    | Error::Transport { .. }
                    | Error::InvalidResponse { .. }),
                ) => {
                    let remaining = total - i - 1;
                    trace!(%name, ?kind, remaining, reason = %e, "lookup failed");
                    saw_transient = true;
                    last_err = Some(e);
                }
                // A fatal error is not specific to this candidate: a query that
                // cannot be built or a missing TLS config would fail every
                // candidate identically, so there is nothing to gain by trying
                // the rest.
                Err(e) => {
                    debug!(%name, ?kind, reason = %e, "lookup failed");
                    return Err(e);
                }
            }
        }

        // No candidate held records. Report the first authoritative negative in
        // search order (NODATA is a successful empty result, NXDOMAIN an error).
        // Negative caching is off by default; when enabled, skip it if a
        // candidate failed indeterminately, since the negative may be wrong.
        match first_negative {
            Some(CachedResult::NoData) => {
                debug!(%name, ?kind, "resolved to no records (NODATA)");
                if !saw_transient {
                    self.cache
                        .insert(&name, kind, CachedResult::NoData, negative_ttl_secs);
                }
                Ok(Vec::new())
            }
            Some(CachedResult::NxDomain) => {
                debug!(%name, ?kind, "does not exist (NXDOMAIN)");
                if !saw_transient {
                    self.cache
                        .insert(&name, kind, CachedResult::NxDomain, negative_ttl_secs);
                }
                Err(e!(Error::NxDomain))
            }
            _ => {
                let err = last_err.unwrap_or_else(|| e!(Error::NoResponse));
                // Serve-stale (RFC 8767): every candidate failed indeterminately
                // and produced no authoritative answer, so fall back to an expired
                // positive answer within the configured window rather than fail.
                if let Some(max_age) = self.builder.serve_stale
                    && let Some(records) = self.cache.get_stale(&name, kind, max_age)
                {
                    debug!(%name, ?kind, "serving stale answer after resolution failure");
                    return Ok(records);
                }
                debug!(%name, ?kind, reason = %err, "resolve failed");
                Err(err)
            }
        }
    }

    /// Looks up the IPv4 (A) records for `name`.
    pub async fn lookup_ipv4(&self, name: impl Into<String>) -> Result<Vec<Ipv4Addr>, Error> {
        let name = name.into();
        // RFC 6761: localhost always resolves to loopback.
        if is_localhost(&name) {
            return Ok(vec![Ipv4Addr::LOCALHOST]);
        }
        // A hosts-file entry overrides DNS, so check it ahead of the cache.
        if let Some(addrs) = self
            .search_names(&name)
            .iter()
            .find_map(|name| self.state().config.hosts.lookup_ipv4(name))
        {
            trace!(%name, ?addrs, "resolved from hosts file");
            return Ok(addrs);
        }
        let addrs: Vec<Ipv4Addr> = self
            .lookup_record(name, RecordKind::A)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::A(ip) => Some(ip),
                _ => None,
            })
            .collect();
        Ok(addrs)
    }

    /// Looks up the IPv6 (AAAA) records for `name`.
    pub async fn lookup_ipv6(&self, name: impl Into<String>) -> Result<Vec<Ipv6Addr>, Error> {
        let name = name.into();
        // RFC 6761: localhost always resolves to loopback.
        if is_localhost(&name) {
            return Ok(vec![Ipv6Addr::LOCALHOST]);
        }
        // A hosts-file entry overrides DNS, so check it ahead of the cache.
        if let Some(addrs) = self
            .search_names(&name)
            .iter()
            .find_map(|name| self.state().config.hosts.lookup_ipv6(name))
        {
            trace!(%name, ?addrs, "resolved from hosts file");
            return Ok(addrs);
        }
        let addrs: Vec<Ipv6Addr> = self
            .lookup_record(name, RecordKind::Aaaa)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::Aaaa(ip) => Some(ip),
                _ => None,
            })
            .collect();
        Ok(addrs)
    }

    /// Looks up the TXT records for `name`.
    pub async fn lookup_txt(&self, name: impl Into<String>) -> Result<Vec<TxtRecordData>, Error> {
        let name = name.into();
        let records: Vec<TxtRecordData> = self
            .lookup_record(name, RecordKind::Txt)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::Txt(txt) => Some(txt),
                _ => None,
            })
            .collect();
        Ok(records)
    }

    /// Looks up the MX (mail exchange) records for `name`.
    pub async fn lookup_mx(&self, name: impl Into<String>) -> Result<Vec<MxRecordData>, Error> {
        let name = name.into();
        let records: Vec<MxRecordData> = self
            .lookup_record(name, RecordKind::Mx)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::Mx(data) => Some(data),
                _ => None,
            })
            .collect();
        Ok(records)
    }

    /// Looks up the SVCB (service binding) records for `name`.
    pub async fn lookup_svcb(&self, name: impl Into<String>) -> Result<Vec<SvcbRecordData>, Error> {
        let name = name.into();
        let records: Vec<SvcbRecordData> = self
            .lookup_record(name, RecordKind::Svcb)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::Svcb(data) => Some(data),
                _ => None,
            })
            .collect();
        Ok(records)
    }

    /// Looks up the HTTPS service binding records for `name`.
    ///
    /// Returns [`HttpsRecordData`]s, which layer HTTPS-specific helpers (the
    /// AliasMode/ServiceMode distinction, the effective target name, and the
    /// default `http/1.1` ALPN) over the raw service binding.
    pub async fn lookup_https(
        &self,
        name: impl Into<String>,
    ) -> Result<Vec<HttpsRecordData>, Error> {
        let name = name.into();
        let records: Vec<HttpsRecordData> = self
            .lookup_record(name, RecordKind::Https)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::Https(data) => Some(data),
                _ => None,
            })
            .collect();
        Ok(records)
    }

    /// Looks up the NS (name server) records for `name`.
    ///
    /// Each entry is the name of one authoritative name server.
    pub async fn lookup_ns(&self, name: impl Into<String>) -> Result<Vec<String>, Error> {
        let name = name.into();
        let records: Vec<String> = self
            .lookup_record(name, RecordKind::Ns)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::Ns(name) => Some(name),
                _ => None,
            })
            .collect();
        Ok(records)
    }

    /// Looks up the SRV (service location) records for `name`.
    pub async fn lookup_srv(&self, name: impl Into<String>) -> Result<Vec<SrvRecordData>, Error> {
        let name = name.into();
        let records: Vec<SrvRecordData> = self
            .lookup_record(name, RecordKind::Srv)
            .await?
            .into_iter()
            .filter_map(|r| match r {
                Record::Srv(data) => Some(data),
                _ => None,
            })
            .collect();
        Ok(records)
    }

    /// Clears the positive DNS cache, dropping every cached answer.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Returns the effective nameservers, primary tier first.
    ///
    /// This is the assembled list the resolver queries: the system
    /// configuration (when [`Builder::use_system_config`] was set) and the
    /// explicitly added nameservers, followed by the fallback tier unless
    /// [`FallbackMode`] excluded it. The first call builds the resolver's
    /// state, which reads the host's DNS configuration.
    pub fn configured_nameservers(&self) -> Vec<Nameserver> {
        self.state().config.nameservers.clone()
    }

    /// Overrides the search domains and `ndots` used for search-list expansion.
    ///
    /// Test-only hook for the ported search-list scenarios in [`crate::tests`],
    /// which drive the public API but cannot reach these fields, since the
    /// builder only populates them from the system configuration.
    #[cfg(test)]
    pub(crate) fn set_search(&mut self, search_domains: Vec<String>, ndots: usize) {
        // Force the runtime state to exist (keeping the configured nameservers),
        // then override just the search settings under test.
        let mut state = ResolverState::build(&self.builder);
        state.config.search_domains = search_domains;
        state.config.ndots = Some(ndots);
        self.state = OnceLock::new();
        let _ = self.state.set(state);
    }

    /// Overrides the hosts-file mapping. Test-only hook, like [`Self::set_search`].
    #[cfg(test)]
    pub(crate) fn set_hosts(&mut self, hosts: Hosts) {
        let mut state = ResolverState::build(&self.builder);
        state.config.hosts = hosts;
        self.state = OnceLock::new();
        let _ = self.state.set(state);
    }

    /// Rebuilds the resolver after a network change, carrying the cache across.
    ///
    /// Does no IO: the new resolver re-reads the system DNS configuration lazily
    /// on its first lookup. Carries the cache across so a network change does not
    /// start DNS cold, which would strand reconnects while the new nameservers
    /// settle (#4037).
    pub fn reset(&self) -> Self {
        Self::new_deferred(self.builder.clone(), self.cache.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
        time::{Duration, Instant},
    };

    use simple_dns::{
        CLASS, Name, Packet, PacketFlag, QCLASS, QTYPE, Question, ResourceRecord, TYPE,
        rdata::{A, RData},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tracing::info;

    use super::{
        CachedResult, DnsResolver, Hosts, TCP_JOIN_DELAY, UDP_PACED_DATAGRAMS, UDP_RETRANSMIT_MIN,
    };
    use crate::{DnsProtocol, FallbackMode, Nameserver, Record, RecordKind, public_resolvers};

    /// Builds a resolver with no nameservers at all.
    ///
    /// For unit tests that do not query the network. A default builder reads
    /// nothing from the host, so these stay hermetic.
    fn empty_resolver() -> DnsResolver {
        DnsResolver::builder().build()
    }

    const GOOGLE_DNS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);
    const CLOUDFLARE_DNS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53);
    #[cfg(transport_tls)]
    const GOOGLE_DNS_TLS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 853);
    #[cfg(transport_https)]
    const CLOUDFLARE_DNS_HTTPS: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 443);

    /// Builds a resolver that queries a single nameserver over `proto`.
    ///
    /// DoT/DoH rely on the default client config built from the crypto provider
    /// (`tls-ring` in the default features), so no config is set here.
    fn with_proto(addr: SocketAddr, proto: DnsProtocol) -> DnsResolver {
        DnsResolver::builder()
            .nameserver(Nameserver::new(addr, proto))
            .build()
    }

    /// A resolver that reads the host system's DNS configuration.
    fn system_resolver() -> DnsResolver {
        DnsResolver::system_with_fallback()
    }

    async fn assert_resolves_ipv4(resolver: &DnsResolver, host: &str) {
        let addrs = resolver.lookup_ipv4(host).await.unwrap();
        assert!(!addrs.is_empty(), "{host} should have IPv4 addresses");
    }

    /// Builds an A response for `example.com`, echoing the question as a real
    /// server does.
    fn a_reply(id: u16, answer: Ipv4Addr) -> Vec<u8> {
        let mut reply = Packet::new_reply(id);
        reply.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        reply.questions.push(Question::new(
            Name::new_unchecked("example.com"),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        reply.answers.push(ResourceRecord::new(
            Name::new_unchecked("example.com"),
            CLASS::IN,
            300,
            RData::A(A {
                address: u32::from(answer),
            }),
        ));
        reply.build_bytes_vec().unwrap()
    }

    /// Spawns a nameserver that swallows `losses` datagrams and answers the next.
    ///
    /// Asserts along the way that every datagram of a run reuses the first one's
    /// transaction id, which is what makes duplicates safe.
    async fn udp_nameserver_losing(
        mut losses: usize,
        answer: Ipv4Addr,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let mut first_id = None;
            loop {
                let (len, peer) = server.recv_from(&mut buf).await.unwrap();
                let id = Packet::parse(&buf[..len]).unwrap().id();
                assert_eq!(
                    id,
                    *first_id.get_or_insert(id),
                    "a retransmit reuses the transaction id"
                );
                if losses == 0 {
                    server.send_to(&a_reply(id, answer), peer).await.unwrap();
                    return;
                }
                losses -= 1;
            }
        });
        (addr, handle)
    }

    /// Spawns a nameserver that answers one query over TCP and never over UDP.
    ///
    /// The shape of a network that permits TCP/53 and drops datagrams, so a
    /// lookup only completes once the TCP query joins them at
    /// [`TCP_JOIN_DELAY`].
    ///
    /// The UDP port is bound and then ignored rather than left closed. A closed
    /// port makes a datagram refused rather than dropped, and Windows reports
    /// that back on the socket as `WSAECONNRESET` where Unix delivers nothing to
    /// an unconnected socket. The refusal leaves nothing in flight, the resolver
    /// correctly brings TCP forward, and there is no join delay left to measure.
    async fn tcp_only_nameserver(answer: Ipv4Addr) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        // Both transports share one port, and a test running in parallel may
        // hold the UDP side of whichever port the OS picks for TCP.
        let (listener, udp) = 'bind: {
            for _ in 0..16 {
                let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                if let Ok(udp) = tokio::net::UdpSocket::bind(tcp.local_addr().unwrap()).await {
                    break 'bind (tcp, udp);
                }
            }
            panic!("no port free for both UDP and TCP");
        };
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            // Held open for the lifetime of the task so the datagrams are
            // dropped rather than refused.
            let _udp = udp;
            let (mut stream, _) = listener.accept().await.unwrap();
            let len = stream.read_u16().await.unwrap() as usize;
            let mut buf = vec![0u8; len];
            stream.read_exact(&mut buf).await.unwrap();
            let bytes = a_reply(Packet::parse(&buf).unwrap().id(), answer);
            stream
                .write_all(&(bytes.len() as u16).to_be_bytes())
                .await
                .unwrap();
            stream.write_all(&bytes).await.unwrap();
            stream.flush().await.unwrap();
        });
        (addr, handle)
    }

    /// Names under `localhost` are recognized whatever their case.
    ///
    /// Only the last label decides; `localhost` elsewhere in the name does not.
    #[test]
    fn localhost_is_matched_case_insensitively() {
        for host in ["localhost", "LOCALHOST.", "foo.LocalHost", "a.b.localhost."] {
            assert!(super::is_localhost(host), "{host}");
        }
        for host in ["localhost.example", "notlocalhost", "", "."] {
            assert!(!super::is_localhost(host), "{host}");
        }
    }

    /// A FORMERR response triggers an EDNS-less retry to the same server.
    ///
    /// RFC 6891. The mock rejects the EDNS query with FORMERR, then answers the
    /// retry that carries no OPT record.
    #[tokio::test]
    async fn formerr_retries_without_edns() {
        let expected = Ipv4Addr::new(198, 51, 100, 9);
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            // First query carries EDNS: reply FORMERR (RCODE 1, low nibble of
            // header byte 3).
            let (n, peer) = server.recv_from(&mut buf).await.unwrap();
            let first = Packet::parse(&buf[..n]).unwrap();
            assert!(first.opt().is_some(), "first query should carry EDNS");
            let mut formerr = Packet::new_reply(first.id()).build_bytes_vec().unwrap();
            formerr[3] = (formerr[3] & 0xF0) | 0x01;
            server.send_to(&formerr, peer).await.unwrap();

            // The retry drops EDNS: answer it with an A record.
            let (n, peer) = server.recv_from(&mut buf).await.unwrap();
            let retry = Packet::parse(&buf[..n]).unwrap();
            assert!(retry.opt().is_none(), "retry should drop EDNS");
            server
                .send_to(&a_reply(retry.id(), expected), peer)
                .await
                .unwrap();
        });

        let resolver = with_proto(addr, DnsProtocol::Udp);
        let addrs = resolver.lookup_ipv4("example.com").await.unwrap();
        assert_eq!(addrs, [expected]);
        handle.await.unwrap();
    }

    /// A lost datagram costs a retransmit interval, not a timeout.
    #[tokio::test]
    async fn lost_datagram_recovers_within_a_retransmit_interval() {
        let expected = Ipv4Addr::new(198, 51, 100, 23);
        let (addr, handle) = udp_nameserver_losing(1, expected).await;
        let resolver = with_proto(addr, DnsProtocol::Udp);

        let start = Instant::now();
        let addrs = resolver.lookup_ipv4("example.com").await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(addrs, [expected]);
        assert!(
            elapsed < UDP_RETRANSMIT_MIN * 2,
            "recovery took {elapsed:?}, expected about {UDP_RETRANSMIT_MIN:?}"
        );
        handle.await.unwrap();
    }

    /// A prompt UDP answer cancels the scheduled TCP fallback before it dials.
    #[tokio::test]
    async fn udp_answer_does_not_start_tcp() {
        let answer = Ipv4Addr::new(198, 51, 100, 24);
        let (listener, udp) = 'bind: {
            for _ in 0..16 {
                let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                if let Ok(udp) = tokio::net::UdpSocket::bind(tcp.local_addr().unwrap()).await {
                    break 'bind (tcp, udp);
                }
            }
            panic!("no port free for both UDP and TCP");
        };
        let addr = listener.local_addr().unwrap();
        let udp_task = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let (len, peer) = udp.recv_from(&mut buf).await.unwrap();
            let id = Packet::parse(&buf[..len]).unwrap().id();
            udp.send_to(&a_reply(id, answer), peer).await.unwrap();
        });
        let (tcp_started, mut tcp_started_rx) = tokio::sync::oneshot::channel();
        let tcp_task = tokio::spawn(async move {
            let _ = listener.accept().await;
            let _ = tcp_started.send(());
        });

        let resolver = with_proto(addr, DnsProtocol::Udp);
        assert_eq!(resolver.lookup_ipv4("example.com").await.unwrap(), [answer]);
        udp_task.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut tcp_started_rx)
                .await
                .is_err(),
            "a UDP answer before the join delay must not open TCP"
        );
        tcp_task.abort();
    }

    /// Datagrams keep going once the paced run is used up.
    ///
    /// The paced run is all swallowed and nothing listens on TCP, so the query
    /// that joins at [`TCP_JOIN_DELAY`] is refused. Only a later datagram can
    /// carry this lookup. That is the case the harness meets under heavy random
    /// loss, where a datagram is likelier to get through than a TCP exchange.
    #[tokio::test]
    async fn datagrams_continue_past_the_paced_run() {
        let expected = Ipv4Addr::new(198, 51, 100, 41);
        let (addr, handle) = udp_nameserver_losing(UDP_PACED_DATAGRAMS, expected).await;
        let resolver = with_proto(addr, DnsProtocol::Udp);

        let start = Instant::now();
        let addrs = resolver.lookup_ipv4("example.com").await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(addrs, [expected]);
        // Due one doubled interval after the paced run, so about four floors in,
        // well short of the limit a resolver that had stopped would wait out.
        assert!(
            elapsed < UDP_RETRANSMIT_MIN * 8,
            "recovered after {elapsed:?}, expected about {:?}",
            UDP_RETRANSMIT_MIN * 4
        );
        handle.await.unwrap();
    }

    /// A nameserver that rejects UDP outright does not wait out the TCP delay.
    ///
    /// Port 0 is not a valid destination, so every send fails at once and
    /// nothing is ever in flight. Waiting there would only delay the TCP query
    /// that is about to fail too, and behind it the next nameserver in the race.
    /// This is the shape of an interface with no route to the server.
    #[tokio::test]
    async fn udp_rejected_outright_skips_the_tcp_delay() {
        let resolver = with_proto("127.0.0.1:0".parse().unwrap(), DnsProtocol::Udp);
        let start = Instant::now();
        assert!(resolver.lookup_ipv4("example.com").await.is_err());
        let elapsed = start.elapsed();
        assert!(
            elapsed < TCP_JOIN_DELAY,
            "gave up after {elapsed:?}, expected well inside the {TCP_JOIN_DELAY:?} TCP join delay"
        );
    }

    /// A server that fails over UDP is retried over TCP.
    ///
    /// This keeps lookups working on a network that drops UDP/53 but allows
    /// TCP/53.
    #[tokio::test]
    async fn udp_failure_falls_back_to_tcp() {
        let expected = Ipv4Addr::new(93, 184, 216, 34);
        let (addr, server) = tcp_only_nameserver(expected).await;
        let resolver = with_proto(addr, DnsProtocol::Udp);
        let addrs = resolver.lookup_ipv4("example.com").await.unwrap();
        assert_eq!(addrs, [expected]);
        server.await.unwrap();
    }

    /// A lookup the TCP query rescued is priced at what it cost.
    ///
    /// This nameserver leaves UDP unanswered, so it spends [`TCP_JOIN_DELAY`] on
    /// every lookup. Ordering has to see that, or a server reachable only over
    /// TCP outranks one answering datagrams in 40ms and gets dialled first
    /// forever. Pacing must not see it. Both estimates come from this one
    /// attempt, and this is the case that separates them.
    #[tokio::test]
    async fn tcp_rescued_lookup_is_priced_at_what_it_cost() {
        let (addr, server) = tcp_only_nameserver(Ipv4Addr::new(93, 184, 216, 34)).await;
        let resolver = with_proto(addr, DnsProtocol::Udp);
        let rtt_map = &resolver.state().rtt_map;
        let (untried_cost, untried_pacing) = (rtt_map.get_decayed(0), rtt_map.get_datagram(0));

        let start = Instant::now();
        resolver.lookup_ipv4("example.com").await.unwrap();
        let elapsed = start.elapsed();
        server.await.unwrap();

        // A lower bound, so a loaded runner only makes it truer. It states the
        // premise: without the join delay there is no cost to price.
        assert!(
            elapsed >= TCP_JOIN_DELAY,
            "lookup took {elapsed:?}, so it never spent the {TCP_JOIN_DELAY:?} \
             join delay this test is about"
        );
        assert!(
            rtt_map.get_decayed(0) > untried_cost * 4.0,
            "a TCP-rescued lookup should rank the server well below an untried one, \
             got {} against a {untried_cost} baseline",
            rtt_map.get_decayed(0)
        );
        assert_eq!(
            rtt_map.get_datagram(0),
            untried_pacing,
            "no datagram came back, so there is nothing to pace off"
        );
    }

    /// Serve-stale answers from an expired entry when no nameserver responds.
    ///
    /// RFC 8767: a brief upstream outage returns the stale answer rather than
    /// an error.
    #[tokio::test]
    async fn serve_stale_returns_expired_answer_on_failure() {
        let expected = Ipv4Addr::new(203, 0, 113, 7);
        // A nameserver on a closed port: the TCP connect is refused at once, so
        // live resolution fails fast and the stale fallback runs.
        let resolver = DnsResolver::builder()
            .serve_stale(Duration::from_secs(3600))
            .nameserver(Nameserver::new(
                "127.0.0.1:1".parse().unwrap(),
                DnsProtocol::Tcp,
            ))
            .build();
        // Seed a positive entry that expired 5s ago.
        resolver.cache.insert_expired(
            "stale.test",
            RecordKind::A,
            CachedResult::Positive(vec![Record::A(expected)]),
            Duration::from_secs(10),
            Duration::from_secs(5),
        );

        let addrs = resolver.lookup_ipv4("stale.test").await.unwrap();
        assert_eq!(addrs, [expected]);
    }

    #[tokio::test]
    async fn resolve_ipv4_udp() {
        assert_resolves_ipv4(&with_proto(GOOGLE_DNS, DnsProtocol::Udp), "google.com").await;
    }

    #[tokio::test]
    async fn resolve_ipv6_udp() {
        let resolver = with_proto(GOOGLE_DNS, DnsProtocol::Udp);
        let addrs = resolver.lookup_ipv6("google.com").await.unwrap();
        assert!(!addrs.is_empty());
    }

    #[tokio::test]
    async fn resolve_ipv4_tcp() {
        assert_resolves_ipv4(&with_proto(CLOUDFLARE_DNS, DnsProtocol::Tcp), "google.com").await;
    }

    #[cfg(transport_tls)]
    #[tokio::test]
    async fn resolve_ipv4_tls() {
        assert_resolves_ipv4(&with_proto(GOOGLE_DNS_TLS, DnsProtocol::Tls), "google.com").await;
    }

    #[cfg(transport_https)]
    #[tokio::test]
    async fn resolve_ipv4_https() {
        assert_resolves_ipv4(
            &with_proto(CLOUDFLARE_DNS_HTTPS, DnsProtocol::Https),
            "google.com",
        )
        .await;
    }

    #[tokio::test]
    async fn resolve_txt_udp() {
        let resolver = with_proto(GOOGLE_DNS, DnsProtocol::Udp);
        let records = resolver.lookup_txt("google.com").await.unwrap();
        assert!(!records.is_empty());
    }

    #[tokio::test]
    async fn resolve_system_defaults() {
        assert_resolves_ipv4(&system_resolver(), "google.com").await;
    }

    #[tokio::test]
    async fn resolve_multiple_sites() {
        let resolver = system_resolver();
        for host in ["google.com", "cloudflare.com", "example.com"] {
            assert_resolves_ipv4(&resolver, host).await;
        }
    }

    #[tokio::test]
    async fn resolve_success_and_nxdomain() {
        let _ = tracing_subscriber::fmt::try_init();
        let resolver = with_proto(GOOGLE_DNS, DnsProtocol::Udp);

        info!("--- resolving example.com (first, expect network query) ---");
        let addrs = resolver.lookup_ipv4("example.com").await.unwrap();
        assert!(!addrs.is_empty());

        info!("--- resolving example.com (second, expect cache hit) ---");
        let addrs2 = resolver.lookup_ipv4("example.com").await.unwrap();
        assert_eq!(addrs, addrs2);

        info!("--- resolving nonexistent domain (expect NXDOMAIN) ---");
        let err = resolver
            .lookup_ipv4("this-domain-does-not-exist.example.invalid")
            .await;
        assert!(err.is_err(), "expected NXDOMAIN, got {err:?}");
    }

    mod search_names {
        use super::*;

        fn resolver_with_search(domains: &[&str]) -> DnsResolver {
            let mut r = empty_resolver();
            r.set_search(domains.iter().map(|s| s.to_string()).collect(), 1);
            r
        }

        #[test]
        fn no_search_domains() {
            let r = empty_resolver();
            assert_eq!(r.search_names("myhost"), vec!["myhost"]);
        }

        #[test]
        fn invalid_search_expansion_is_dropped() {
            // A search domain with an over-long (>63 byte) label makes the
            // expansion an invalid DNS name. It must be skipped, not carried into
            // the candidate list where it would abort the lookup, and the bare
            // name must still be tried.
            let long_label = "a".repeat(64);
            let r = resolver_with_search(&[long_label.as_str(), "example.com"]);
            assert_eq!(
                r.search_names("myhost"),
                vec!["myhost.example.com", "myhost"]
            );
        }

        #[test]
        fn fqdn_bypasses_search() {
            let r = resolver_with_search(&["example.com"]);
            assert_eq!(
                r.search_names("myhost.example.com."),
                vec!["myhost.example.com."]
            );
        }

        #[test]
        fn short_name_tries_search_first() {
            let r = resolver_with_search(&["example.com", "test.local"]);
            // "myhost" has 0 dots (< ndots=1), so search domains come first.
            assert_eq!(
                r.search_names("myhost"),
                vec!["myhost.example.com", "myhost.test.local", "myhost"]
            );
        }

        #[test]
        fn dotted_name_tries_bare_first() {
            let r = resolver_with_search(&["example.com"]);
            // "foo.bar" has 1 dot (>= ndots=1), so bare name comes first.
            assert_eq!(
                r.search_names("foo.bar"),
                vec!["foo.bar", "foo.bar.example.com"]
            );
        }

        #[test]
        fn multi_dot_name_tries_bare_first() {
            let r = resolver_with_search(&["example.com"]);
            assert_eq!(r.search_names("a.b.c"), vec!["a.b.c", "a.b.c.example.com"]);
        }

        #[test]
        fn high_ndots_k8s_style() {
            let mut r = empty_resolver();
            r.set_search(
                vec!["ns.svc.cluster.local".into(), "svc.cluster.local".into()],
                5,
            );
            // 4 dots < ndots=5, so search domains come first (Kubernetes behavior).
            assert_eq!(
                r.search_names("my-svc.my-ns.svc.cluster.local"),
                vec![
                    "my-svc.my-ns.svc.cluster.local.ns.svc.cluster.local",
                    "my-svc.my-ns.svc.cluster.local.svc.cluster.local",
                    "my-svc.my-ns.svc.cluster.local",
                ]
            );
        }

        #[test]
        fn ndots_two_short_name_tries_search_first() {
            let mut r = empty_resolver();
            r.set_search(vec!["example.com".into(), "test.local".into()], 2);
            // "foo.bar" has 2 labels, not more than ndots=2, so search first.
            assert_eq!(
                r.search_names("foo.bar"),
                vec!["foo.bar.example.com", "foo.bar.test.local", "foo.bar"]
            );
        }

        #[test]
        fn ndots_two_long_name_tries_bare_first() {
            let mut r = empty_resolver();
            r.set_search(vec!["example.com".into()], 2);
            // "a.b.c" has 3 labels, more than ndots=2, so the bare name comes first.
            assert_eq!(r.search_names("a.b.c"), vec!["a.b.c", "a.b.c.example.com"]);
        }

        #[test]
        fn duplicate_search_expansion_is_suppressed() {
            // A repeated search domain would regenerate the same candidate; the
            // deduplication keeps each name once while preserving order.
            let r = resolver_with_search(&["example.com", "example.com"]);
            assert_eq!(r.search_names("foo"), vec!["foo.example.com", "foo"]);
        }
    }

    /// Spawns a mock UDP nameserver that answers a single query.
    ///
    /// It replies with `rcode`, echoes the question, and adds `answer` as an A
    /// record when one is given.
    async fn spawn_mock_ns(
        rcode: simple_dns::RCODE,
        answer: Option<Ipv4Addr>,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 512];
            let (len, peer) = socket.recv_from(&mut buf).await.unwrap();
            let query = Packet::parse(&buf[..len]).unwrap();
            let question = query.questions[0].clone();
            let mut reply = Packet::new_reply(query.id());
            reply.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
            *reply.rcode_mut() = rcode;
            if let Some(ip) = answer {
                reply.answers.push(ResourceRecord::new(
                    question.qname.clone(),
                    CLASS::IN,
                    300,
                    RData::A(A {
                        address: u32::from(ip),
                    }),
                ));
            }
            reply.questions.push(question);
            socket
                .send_to(&reply.build_bytes_vec().unwrap(), peer)
                .await
                .unwrap();
        });
        (addr, handle)
    }

    /// A SERVFAIL or REFUSED answer must not end the lookup.
    ///
    /// Those mean this server will not answer for the name, so the resolver
    /// races on to one that can rather than returning the failure.
    #[tokio::test]
    async fn servfail_winner_falls_through_to_next_nameserver() {
        let (bad, bad_handle) = spawn_mock_ns(simple_dns::RCODE::ServerFailure, None).await;
        let (good, good_handle) =
            spawn_mock_ns(simple_dns::RCODE::NoError, Some(Ipv4Addr::new(10, 1, 2, 3))).await;

        // `bad` is listed first, so it is the fastest by default ordering and
        // wins the race with a SERVFAIL; the lookup must fall through to `good`.
        let resolver = DnsResolver::builder()
            .nameservers([
                Nameserver::new(bad, DnsProtocol::Udp),
                Nameserver::new(good, DnsProtocol::Udp),
            ])
            .build();

        let addrs = resolver.lookup_ipv4("test.example").await.unwrap();
        assert_eq!(addrs, [IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))]);

        bad_handle.await.unwrap();
        good_handle.await.unwrap();
    }

    /// A nameserver address that is never actually queried.
    ///
    /// Used to check how the builder lays out the primary and fallback tiers.
    const DUMMY: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 53);

    /// The default `Deferred` mode keeps the fallback behind the primary tier.
    ///
    /// They form a second tier rather than racing with the primary one.
    #[test]
    fn deferred_keeps_fallback_in_second_tier() {
        let r = DnsResolver::builder()
            .nameserver(Nameserver::new(DUMMY, DnsProtocol::Udp))
            .fallback_nameservers([Nameserver::new(DUMMY, DnsProtocol::Udp)])
            .build();
        assert_eq!(r.state().primary_count, 1);
        assert_eq!(r.state().config.nameservers.len(), 2);
    }

    /// `FallbackMode::Eager` merges the fallback into the primary tier.
    ///
    /// The two then race from the start rather than forming two tiers.
    #[test]
    fn eager_merges_tiers() {
        let r = DnsResolver::builder()
            .nameserver(Nameserver::new(DUMMY, DnsProtocol::Udp))
            .fallback_nameservers([Nameserver::new(DUMMY, DnsProtocol::Udp)])
            .fallback_mode(FallbackMode::Eager)
            .build();
        assert_eq!(r.state().primary_count, 2);
        assert_eq!(r.state().config.nameservers.len(), 2);
    }

    /// A builder with no fallback nameservers has no second tier.
    ///
    /// The mode then never comes into play, whichever one is set.
    #[test]
    fn empty_fallback_leaves_a_single_tier() {
        for mode in [
            FallbackMode::Deferred,
            FallbackMode::Eager,
            FallbackMode::IfSystemEmpty,
        ] {
            let r = DnsResolver::builder()
                .nameserver(Nameserver::new(DUMMY, DnsProtocol::Udp))
                .fallback_mode(mode)
                .build();
            assert_eq!(r.state().primary_count, 1, "{mode:?}");
            assert_eq!(r.state().config.nameservers.len(), 1, "{mode:?}");
        }
    }

    /// `default_fallback_nameservers` fills the fallback tier only.
    ///
    /// The public resolvers land there, leaving the primary tier untouched.
    #[test]
    fn default_fallback_nameservers_fills_second_tier() {
        let r = DnsResolver::builder()
            .nameserver(Nameserver::new(DUMMY, DnsProtocol::Udp))
            .default_fallback_nameservers()
            .build();
        assert_eq!(r.state().primary_count, 1);
        assert_eq!(
            r.state().config.nameservers.len(),
            1 + public_resolvers::default_order(public_resolvers::Provider::ALL.to_vec()).len()
        );
    }

    /// `IfSystemEmpty` uses the fallback when there is no system configuration.
    ///
    /// It merges them into the primary tier rather than deferring them.
    #[test]
    fn if_system_empty_includes_fallback_when_system_empty() {
        let r = DnsResolver::builder()
            .fallback_mode(FallbackMode::IfSystemEmpty)
            .fallback_nameservers([Nameserver::new(DUMMY, DnsProtocol::Udp)])
            .build();
        assert_eq!(r.state().config.nameservers.len(), 1);
        assert_eq!(r.state().primary_count, 1);
    }

    /// A lookup escalates to the fallback tier when every primary one fails.
    ///
    /// The fallback then resolves the name.
    #[tokio::test]
    async fn escalates_to_fallback_when_primary_fails() {
        let (bad, bad_handle) = spawn_mock_ns(simple_dns::RCODE::ServerFailure, None).await;
        let (good, good_handle) =
            spawn_mock_ns(simple_dns::RCODE::NoError, Some(Ipv4Addr::new(10, 4, 5, 6))).await;

        // The primary tier is only `bad`, which SERVFAILs; the fallback tier is
        // `good`, reached only after the primary tier is exhausted.
        let resolver = DnsResolver::builder()
            .nameserver(Nameserver::new(bad, DnsProtocol::Udp))
            .fallback_nameservers([Nameserver::new(good, DnsProtocol::Udp)])
            .build();

        let addrs = resolver.lookup_ipv4("test.example").await.unwrap();
        assert_eq!(addrs, [IpAddr::V4(Ipv4Addr::new(10, 4, 5, 6))]);

        bad_handle.await.unwrap();
        good_handle.await.unwrap();
    }

    /// A hosts-file entry overrides DNS and resolves without a network query.
    ///
    /// This is how the old hickory-backed resolver honored `/etc/hosts`.
    #[tokio::test]
    async fn hosts_file_overrides_dns() {
        let mut resolver = empty_resolver();
        resolver.set_hosts(Hosts::from_content(
            "10.0.1.10 myrelay.test\n::1 myrelay.test\n",
        ));

        let v4 = resolver.lookup_ipv4("myrelay.test").await.unwrap();
        assert_eq!(v4, [Ipv4Addr::new(10, 0, 1, 10)]);

        // A trailing dot (FQDN form) still matches the hosts entry.
        let v6 = resolver.lookup_ipv6("myrelay.test.").await.unwrap();
        assert_eq!(v6, [Ipv6Addr::LOCALHOST]);
    }

    /// [`DnsResolver::reset`] carries the cache across a network change.
    ///
    /// Reconnects then keep resolving while the new nameservers settle, rather
    /// than starting DNS cold (issue #4037).
    #[test]
    fn cache_survives_reset() {
        let r = empty_resolver();
        r.cache.insert(
            "example.com",
            RecordKind::A,
            CachedResult::Positive(vec![Record::A(Ipv4Addr::LOCALHOST)]),
            300,
        );

        let reset = r.reset();

        let cached = reset.cache.get("example.com", RecordKind::A);
        let survived = matches!(
            cached,
            Some(CachedResult::Positive(ref records))
                if matches!(records.as_slice(), [Record::A(addr)] if *addr == Ipv4Addr::LOCALHOST)
        );
        assert!(survived, "cache entry should survive reset, got {cached:?}");
    }
}
