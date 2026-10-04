# n0-dns-resolver

A small async DNS stub resolver built on [`simple-dns`] and tokio, used in
[iroh].

It resolves DNS records against the system's configured nameservers 
(or an explicit list), and is meant to be a lightweight, dependency-light
yet fully-featured DNS resolver. It does not perform recursive resolution.

## Features

- Reads the system DNS configuration on request: `/etc/resolv.conf` on Unix,
  the SystemConfiguration framework on Apple platforms, the network adapters on
  Windows, and a JNI call on Android. The system nameservers form a primary
  tier. Behind it sits a fallback tier, either the public resolvers
  (Cloudflare, Google, Quad9) or your own, queried only when the primary tier
  cannot answer.
- Consults the system hosts file (`/etc/hosts` and the Windows equivalent)
  ahead of the network, when the system configuration is read.
- Resolves A, AAAA, TXT, NS, SRV, MX, CAA, SVCB, and HTTPS records, follows
  `CNAME` chains, and applies `search`/`ndots` expansion.
- Caches positive results by TTL. Negative caching (NODATA, NXDOMAIN) is off
  by default; enable it with `Builder::negative_max_ttl`. Optionally serves
  expired entries when every nameserver fails (RFC 8767 serve-stale) and
  floors very low TTLs, both off by default.
- Races nameservers happy-eyeballs style, ordered by measured round-trip time.
  A truncated or failed UDP query falls back to TCP on the same server, so
  lookups survive networks that block UDP/53, and a FORMERR triggers a retry
  without EDNS before the next server is tried.
- Speaks DNS-over-TLS and DNS-over-HTTPS (on by default, see feature flags),
  pooling and reusing connections.

It does not perform DNSSEC validation.

## Usage

```rust,no_run
use n0_dns_resolver::{
    DnsProtocol, DnsResolver, Nameserver, interleave_nameservers,
    public_resolvers::{self, Provider},
};

# async fn run() -> Result<(), n0_dns_resolver::Error> {
// Cross-platform defaults: the system configuration, then the public-resolver
// fallback. This is the only constructor; everything else goes through the
// builder.
let resolver = DnsResolver::system_with_fallback();
let addrs: Vec<_> = resolver.lookup_ipv4("example.com").await?.collect();

// The builder starts empty, with no system configuration and no nameservers,
// so each source is added explicitly. Here it is a single nameserver.
let ns = Nameserver::new("1.1.1.1:53".parse().unwrap(), DnsProtocol::Udp);
let resolver = DnsResolver::builder().nameserver(ns).build();

// The system configuration, with Quad9 and Cloudflare (but not Google) behind
// it as a fallback tier.
let resolver = DnsResolver::builder()
    .use_system_config()
    .fallback_nameservers(public_resolvers::default_order(vec![
        Provider::Quad9,
        Provider::Cloudflare,
    ]))
    .build();

// Or assemble the list yourself: DNS-over-TLS to both, alternating between
// them. `default_order` is one opinion, built from these same public pieces.
let resolver = DnsResolver::builder()
    .nameservers(interleave_nameservers([
        Provider::Quad9.nameservers(DnsProtocol::Tls),
        Provider::Cloudflare.nameservers(DnsProtocol::Tls),
    ]))
    .build();
# Ok(())
# }
```

See `examples/configurations.rs` for these and other setups side by side.

By default the fallback tier is used only when the primary nameservers fail or
time out. `Builder::fallback_mode` changes that: `FallbackMode::Eager` races
the fallback alongside the primary servers, and `FallbackMode::IfSystemEmpty`
uses it only when the system configuration yielded no nameservers. To query no
fallback at all, add none.

## Feature flags

Plain DNS over UDP and TCP is always available. The encrypted transports are
feature-gated:

- `transport-tls`: DNS-over-TLS, via rustls.
- `transport-https`: DNS-over-HTTPS, via reqwest with rustls.
- `tls-ring` / `tls-aws-lc-rs`: the rustls crypto provider used to build the
  default TLS client config when none is supplied on the builder.

The default features are `transport-tls`, `transport-https`, and `tls-ring`,
so DoT and DoH work out of the box. With `default-features = false` the
resolver speaks plain DNS only. Enabling a transport without a crypto
provider also works, but then a TLS client config must be supplied on the
builder.


## License

Copyright 2025 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

[`simple-dns`]: https://docs.rs/simple-dns
[iroh]: https://github.com/n0-computer/iroh
