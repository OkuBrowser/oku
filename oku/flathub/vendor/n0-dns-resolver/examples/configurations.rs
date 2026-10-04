//! Five ways to configure a [`DnsResolver`], resolved side by side.
//!
//! The builder starts empty: it reads nothing from the host and queries no
//! nameservers until you add them. This walks from the ready-made default down
//! to a hand-assembled nameserver list, running the same lookup through each so
//! the differences are visible rather than theoretical.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example configurations -- example.com
//! ```

use std::env;

use n0_dns_resolver::{
    DnsProtocol, DnsResolver, Error, FallbackMode, interleave_nameservers,
    public_resolvers::{self, Provider},
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    let host = env::args().nth(1).unwrap_or_else(|| "example.com".into());

    // The common case, and the only constructor: the host's DNS configuration,
    // with the default public resolvers (Cloudflare, Google, Quad9) as a
    // fallback tier behind it.
    let system_with_fallback = DnsResolver::system_with_fallback();

    // The same thing spelled out on the builder, but with the fallback tier
    // narrowed to Quad9 and Cloudflare, with no Google. `default_order` applies
    // crate's order to whichever providers you pick, so the selection keeps the
    // property that makes the default work: a DNS-over-HTTPS entry inside the
    // first raced wave, for networks that silently drop UDP/53.
    let system_and_two_providers = DnsResolver::builder()
        .use_system_config()
        .fallback_nameservers(public_resolvers::default_order(vec![
            Provider::Quad9,
            Provider::Cloudflare,
        ]))
        .build();

    // No system configuration at all: Quad9 over plain UDP, as the primary
    // tier. Nothing is read from /etc/resolv.conf or /etc/hosts, and no other
    // provider is ever contacted. This is useful when resolution must behave
    // the same regardless of the host it runs on.
    let quad9_only = DnsResolver::builder()
        .nameservers(Provider::Quad9.nameservers(DnsProtocol::Udp))
        .build();

    // `default_order` is only one opinion, and it is assembled from public
    // pieces. Here is a different one, built by hand: DNS-over-TLS to Quad9 and
    // Cloudflare first, alternating between them, with plain UDP to the same
    // two behind it as a fallback tier. Encrypted transport is preferred
    // whenever it works, and unencrypted DNS is the last resort rather than the
    // first attempt.
    let providers = [Provider::Quad9, Provider::Cloudflare];
    let encrypted = interleave_nameservers(providers.map(|p| p.nameservers(DnsProtocol::Tls)));
    let plain = interleave_nameservers(providers.map(|p| p.nameservers(DnsProtocol::Udp)));
    let dot_first = DnsResolver::builder()
        .nameservers(encrypted)
        .fallback_nameservers(plain)
        .build();

    // The system configuration with the public resolvers raced alongside it
    // instead of behind it. This gives up the host's precedence for lower
    // worst-case latency on networks that silently drop plain DNS.
    let raced = DnsResolver::builder()
        .use_system_config()
        .default_fallback_nameservers()
        .fallback_mode(FallbackMode::Eager)
        .build();

    let resolvers = [
        ("system + all default fallbacks", system_with_fallback),
        ("system + quad9 and cloudflare", system_and_two_providers),
        ("quad9 only, no system config", quad9_only),
        ("hand-rolled: DoT, then plain", dot_first),
        ("system + fallbacks, raced", raced),
    ];

    for (label, resolver) in resolvers {
        match resolver.lookup_ipv4(&host).await {
            Ok(addrs) => println!("{label:<32} {addrs:?}"),
            Err(err) => println!("{label:<32} failed: {err:#}"),
        }
    }

    Ok(())
}
