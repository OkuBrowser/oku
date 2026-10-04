//! Compare iroh's real lookups: this crate vs the current iroh hickory glue.
//!
//! iroh resolves relay hostnames (A/AAAA) and `_iroh.<z32>.<origin>` TXT records
//! at the configured DNS discovery origin (`dns.iroh.link.` in production).
//!
//! With no extra args, it publishes a fresh `_iroh` TXT via pkarr (same path
//! as default discovery) and compares that lookup too. Pass an origin and z32
//! to use an existing name instead.
//!
//! Run:
//! ```text
//! cargo run --example compare_iroh_lookups
//! cargo run --example compare_iroh_lookups -- staging-dns.iroh.link. <z32>
//! ```

use std::{
    future::Future,
    panic::catch_unwind,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

use derive_more::Display;
use hickory_resolver::{
    TokioResolver,
    config::{ResolverConfig, ResolverOpts},
    net::runtime::TokioRuntimeProvider,
    proto::rr::{RData, RecordType},
};
use iroh_base::SecretKey;
use n0_dns_resolver::DnsResolver;
use n0_error::StdResultExt;
use n0_future::time;
use simple_dns::{CLASS, Name, Packet, ResourceRecord};
use tracing::{debug, warn};

/// Production DNS discovery origin (`iroh_dns::N0_DNS_ENDPOINT_ORIGIN_PROD`).
const DNS_ORIGIN_PROD: &str = "dns.iroh.link.";

/// Production relay hostnames from `iroh::defaults::prod`.
const RELAY_HOSTS: &[&str] = &[
    "use1-1.relay.n0.iroh.link.",
    "usw1-1.relay.n0.iroh.link.",
    "euc1-1.relay.n0.iroh.link.",
    "aps1-1.relay.n0.iroh.link.",
];

fn main() {
    tracing_subscriber::fmt::init();
    run()
}

/// Runs the comparison.
///
/// The `compare_iroh_lookups_android` example calls this from its Android entry
/// point, which is why it is a separate function: everything below is shared
/// with the on-device run, only the setup above it differs.
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    debug!("start");

    let mut args = std::env::args().skip(1);
    let mut origin = args.next().unwrap_or_else(|| DNS_ORIGIN_PROD.to_string());
    if !origin.ends_with('.') {
        origin.push('.');
    }
    let z32s: Vec<String> = args.collect();

    let hickory = build_hickory_resolver();
    debug!("hickory built");
    let n0 = DnsResolver::system_with_fallback();
    debug!("n0 built");
    // Force the lazy system-config read so the first timed lookup does not
    // include it (hickory reads the config eagerly above).
    let _ = n0.configured_nameservers();
    debug!("n0 init");

    let mut queries = Vec::new();
    for host in RELAY_HOSTS {
        queries.extend([RecordType::A, RecordType::AAAA].map(|kind| Query::new(kind, *host)));
    }
    for z32 in &z32s {
        queries.push(Query::new(RecordType::TXT, format!("_iroh.{z32}.{origin}")));
    }
    if z32s.is_empty()
        && let Ok(name) = publish_iroh_txt(&origin)
            .await
            .inspect_err(|err| warn!("pkarr PUT failed, skipping TXT query: {err:#}"))
    {
        wait_for_txt(&n0, &hickory, &name).await;
        queries.push(Query::new(RecordType::TXT, name));
    }

    println!(
        "{} queries, timings and answers are hickory / n0-dns\n",
        queries.len()
    );
    let mut totals = [Duration::ZERO; 4];
    let mut mismatches = 0;
    for query in &queries {
        let ((h_first, h_out), (n_first, n_out)) = tokio::join!(
            timed(lookup_hickory(&hickory, query)),
            timed(lookup_n0(&n0, query))
        );
        let (h_cached, _) = timed(lookup_hickory(&hickory, query)).await;
        let (n_cached, _) = timed(lookup_n0(&n0, query)).await;
        let samples = [h_first, n_first, h_cached, n_cached];
        totals = std::array::from_fn(|i| totals[i] + samples[i]);

        let times = format!(
            "first {:>7} / {:>7}   cached {:>7} / {:>7}",
            fmt_duration(h_first),
            fmt_duration(n_first),
            fmt_duration(h_cached),
            fmt_duration(n_cached)
        );
        if h_out == n_out {
            println!("  ok  {times}   {query}  {h_out}");
        } else {
            mismatches += 1;
            println!("FAIL  {times}   {query}");
            println!("        hickory  {h_out}");
            println!("        n0-dns   {n_out}");
        }
    }

    let n = queries.len();
    let [h_first, n_first, h_cached, n_cached] = totals.map(fmt_duration);
    println!();
    if mismatches == 0 {
        println!(
            "PASS  {n}/{n} match   totals: first {h_first} / {n_first}   cached {h_cached} / {n_cached}"
        );
    } else {
        println!("FAIL  {mismatches}/{n} queries differ");
        std::process::exit(1);
    }
}

/// A single lookup, run against both resolvers.
#[derive(Debug, Clone, Display)]
#[display("{:<4} {name}", kind.to_string())]
struct Query {
    kind: RecordType,
    name: String,
}

impl Query {
    fn new(kind: RecordType, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
        }
    }
}

/// Comparable lookup result. Records are rendered to strings and sorted so
/// order differences do not count as a mismatch.
#[derive(Debug, Clone, PartialEq, Eq, Display)]
enum Outcome {
    #[display("[{}]", _0.join(", "))]
    Records(Vec<String>),
    #[display("NXDOMAIN")]
    NxDomain,
    #[display("error: {_0}")]
    Error(String),
}

impl Outcome {
    fn records(mut records: Vec<String>) -> Self {
        records.sort();
        Self::Records(records)
    }
}

/// Renders TXT character strings in DNS presentation style (one quoted string
/// per segment), so segment boundaries participate in the comparison.
fn txt_repr<'a>(segments: impl Iterator<Item = &'a [u8]>) -> String {
    segments
        .map(|segment| format!("{:?}", String::from_utf8_lossy(segment)))
        .collect::<Vec<_>>()
        .join(" ")
}

async fn timed<T>(fut: impl Future<Output = T>) -> (Duration, T) {
    let start = Instant::now();
    let value = fut.await;
    (start.elapsed(), value)
}

fn fmt_duration(duration: Duration) -> String {
    if duration < Duration::from_millis(1) {
        // Sub-millisecond (the cached lookups): let Duration pick us/ns units.
        format!("{duration:.0?}")
    } else {
        format!("{:.1}ms", duration.as_secs_f64() * 1000.0)
    }
}

#[tracing::instrument(name = "query", skip_all, fields(r = %"n0", q=query_id()))]
async fn lookup_n0(resolver: &DnsResolver, query: &Query) -> Outcome {
    debug!("query {query}");
    let name = query.name.clone();
    let records: Result<Vec<String>, _> = match query.kind {
        RecordType::A => resolver
            .lookup_ipv4(name)
            .await
            .map(|addrs| addrs.iter().map(|addr| addr.to_string()).collect()),
        RecordType::AAAA => resolver
            .lookup_ipv6(name)
            .await
            .map(|addrs| addrs.iter().map(|addr| addr.to_string()).collect()),
        RecordType::TXT => resolver
            .lookup_txt(name)
            .await
            .map(|records| records.iter().map(|txt| txt_repr(txt.iter())).collect()),
        kind => unreachable!("unsupported record type {kind}"),
    };
    match records {
        Ok(records) => Outcome::records(records),
        Err(n0_dns_resolver::Error::NxDomain { .. }) => Outcome::NxDomain,
        Err(err) => Outcome::Error(err.to_string()),
    }
}

/// Builds a hickory resolver configured like `iroh-dns` `HickoryResolver`:
/// system DNS config, Google only if that read fails, `negative_max_ttl = 0`.
fn build_hickory_resolver() -> TokioResolver {
    let google = || {
        (
            ResolverConfig::udp_and_tcp(&hickory_resolver::config::GOOGLE),
            ResolverOpts::default(),
        )
    };
    // On Android the read goes through `ConnectivityManager` over JNI and
    // panics, rather than returning an error, when `ndk_context` is
    // uninitialized. We catch the panic, like we do in iroh's hickory-resolver wrapper.
    let (config, mut options) = match catch_unwind(hickory_resolver::system_conf::read_system_conf)
    {
        Ok(Ok(system_conf)) => system_conf,
        Ok(Err(reason)) => {
            warn!(
                %reason,
                "hickory-resolver: Failed to read the system's DNS config, using Google DNS servers as fallback."
            );
            google()
        }
        Err(_) => {
            warn!(
                "hickory-resolver: Reading the system's DNS config panicked, using Google DNS servers as fallback."
            );
            google()
        }
    };
    options.negative_max_ttl = Some(Duration::ZERO);

    let mut builder = TokioResolver::builder_with_config(config, TokioRuntimeProvider::default());
    *builder.options_mut() = options;
    builder.build().expect("config works")
}

#[tracing::instrument(name = "query", skip_all, fields(r = %"hi", q=query_id()))]
async fn lookup_hickory(resolver: &TokioResolver, query: &Query) -> Outcome {
    debug!("query {query}");
    match resolver.lookup(query.name.clone(), query.kind).await {
        Ok(lookup) => Outcome::records(
            lookup
                .answers()
                .iter()
                .filter_map(|record| match &record.data {
                    RData::A(addr) => Some(addr.0.to_string()),
                    RData::AAAA(addr) => Some(addr.0.to_string()),
                    RData::TXT(txt) => Some(txt_repr(txt.txt_data.iter().map(AsRef::as_ref))),
                    _ => None,
                })
                .collect(),
        ),
        Err(err) if err.is_nx_domain() => Outcome::NxDomain,
        Err(err) if err.is_no_records_found() => Outcome::Records(Vec::new()),
        Err(err) => Outcome::Error(err.to_string()),
    }
}

/// Publish an `_iroh` TXT the same way default discovery does: a pkarr PUT to
/// the relay served at `origin`. Returns the FQDN to look up.
async fn publish_iroh_txt(origin: &str) -> n0_error::Result<String> {
    debug!("publishing pkarr TXT record to {origin}");
    const TXT_VALUE: &str = "relay=https://use1-1.relay.n0.iroh.link.";

    let secret = SecretKey::generate();
    let z32 = secret.public().to_z32();

    let mut txt = simple_dns::rdata::TXT::new();
    txt.add_string(TXT_VALUE).expect("txt fits");
    let mut packet = Packet::new_reply(0);
    packet.answers.push(ResourceRecord::new(
        Name::new_unchecked(&format!("_iroh.{z32}")).into_owned(),
        CLASS::IN,
        30,
        simple_dns::rdata::RData::TXT(txt.into_owned()),
    ));
    let encoded = packet.build_bytes_vec_compressed().expect("packet builds");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_micros() as u64;
    let mut signable = format!("3:seqi{timestamp}e1:v{}:", encoded.len()).into_bytes();
    signable.extend_from_slice(&encoded);
    let signature = secret.sign(&signable);
    let body = [
        &signature.to_bytes()[..],
        &timestamp.to_be_bytes(),
        &encoded,
    ]
    .concat();

    let provider = rustls::crypto::ring::default_provider();
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .expect("protocols")
        .with_root_certificates(roots)
        .with_no_client_auth();
    let client = reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .build()
        .std_context("Failed to build reqwest client")?;

    let url = format!("https://{}/pkarr/{z32}", origin.trim_end_matches('.'));
    let _res = client
        .put(&url)
        .body(body)
        .send()
        .await
        .with_std_context(|_| format!("pkarr PUT {url}: failed to receive response"))?
        .error_for_status()
        .with_std_context(|_| format!("pkarr PUT {url}: bad response"))?;
    let name = format!("_iroh.{z32}.{origin}");
    debug!("TXT record published under {name}");
    Ok(name)
}

/// Waits until both resolvers see the freshly published TXT, then clears both
/// caches so the timed comparison starts cold.
async fn wait_for_txt(n0: &DnsResolver, hickory: &TokioResolver, name: &str) {
    // dns.iroh.link serves the packet immediately; recursive resolvers may
    // still have an NXDOMAIN for a few hundred milliseconds.
    time::sleep(Duration::from_secs(1)).await;

    let query = Query::new(RecordType::TXT, name);
    let deadline = time::Instant::now() + Duration::from_secs(15);
    loop {
        let n0_out = lookup_n0(n0, &query).await;
        let hickory_out = lookup_hickory(hickory, &query).await;
        let ready = |out: &Outcome| matches!(out, Outcome::Records(records) if !records.is_empty());
        if ready(&n0_out) && ready(&hickory_out) {
            n0.clear_cache();
            hickory.clear_cache();
            return;
        }
        assert!(
            time::Instant::now() < deadline,
            "timed out waiting for TXT at {name}: n0-dns={n0_out} hickory={hickory_out}"
        );
        time::sleep(Duration::from_millis(250)).await;
    }
}

fn query_id() -> u32 {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}
