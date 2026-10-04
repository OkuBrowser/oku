//! A small async DNS stub resolver built on [`simple-dns`] and tokio.
//!
//! The main export is [`DnsResolver`]. It reads the system DNS configuration
//! (or an explicit nameserver list) and resolves the common record kinds (see
//! [`RecordKind`]) through [`DnsResolver::lookup_record`]. Lookups follow CNAME
//! chains, cache their results, and race the configured nameservers
//! happy-eyeballs style, falling back to a second tier of nameservers when the
//! primary ones cannot answer. The resolver speaks plain DNS over UDP and TCP,
//! and, with a crypto provider enabled, DNS-over-TLS (DoT) and DNS-over-HTTPS
//! (DoH).
//!
//! Construct a resolver with [`DnsResolver::system_with_fallback`] for
//! cross-platform defaults: the system configuration, backed by the public
//! resolvers in [`public_resolvers`]. Every other setup goes through
//! [`DnsResolver::builder`], which starts empty, with no system configuration
//! and no nameservers, so that each source is added explicitly. See [`Builder`]
//! for the available settings.
//!
//! # Examples
//!
//! ```no_run
//! # async fn run() -> Result<(), n0_dns_resolver::Error> {
//! use n0_dns_resolver::{DnsResolver, RecordKind};
//!
//! let resolver = DnsResolver::system_with_fallback();
//!
//! let addrs = resolver.lookup_ipv4("example.com").await?;
//! let records = resolver
//!     .lookup_record("example.com", RecordKind::Aaaa)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! [`simple-dns`]: https://docs.rs/simple-dns
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(n0_dns_resolver_docsrs, feature(doc_cfg))]

mod builder;
mod config;
mod error;
pub mod public_resolvers;
mod records;
mod resolver;
mod system_config;

#[cfg(test)]
mod tests;

#[cfg(any(target_os = "android", doc))]
pub use self::system_config::install_android_jni_context;
pub use self::{
    builder::{Builder, DnsProtocol, FallbackMode, Nameserver, interleave_nameservers},
    error::{Error, ResponseCode},
    records::{
        CaaRecordData, HttpsRecordData, MxRecordData, Record, RecordKind, SrvRecordData,
        SvcbRecordData, TxtRecordData,
    },
    resolver::{DnsResolver, TransportError},
};
