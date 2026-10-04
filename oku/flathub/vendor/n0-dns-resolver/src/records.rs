//! The record kinds and the parsed record data returned by lookups.

use std::{
    fmt,
    net::{Ipv4Addr, Ipv6Addr},
};

use simple_dns::rdata::{SVCB, SVCParam};

/// A DNS record kind the resolver can look up.
///
/// Pass one to [`DnsResolver::lookup_record`] to resolve records of that
/// kind. The typed methods ([`DnsResolver::lookup_ipv4`],
/// [`DnsResolver::lookup_ipv6`], [`DnsResolver::lookup_txt`]) each
/// resolve a fixed kind and are thin wrappers over `lookup_record`.
///
/// [`DnsResolver::lookup_record`]: crate::DnsResolver::lookup_record
/// [`DnsResolver::lookup_ipv4`]: crate::DnsResolver::lookup_ipv4
/// [`DnsResolver::lookup_ipv6`]: crate::DnsResolver::lookup_ipv6
/// [`DnsResolver::lookup_txt`]: crate::DnsResolver::lookup_txt
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum RecordKind {
    /// An IPv4 address record (A).
    A,
    /// An IPv6 address record (AAAA).
    Aaaa,
    /// A text record (TXT).
    Txt,
    /// A name server record (NS).
    Ns,
    /// A service location record (SRV).
    Srv,
    /// A mail exchange record (MX).
    Mx,
    /// A certification authority authorization record (CAA).
    Caa,
    /// A service binding record (SVCB).
    Svcb,
    /// An HTTPS service binding record (HTTPS).
    Https,
}

/// Parsed record data returned by [`DnsResolver::lookup_record`].
///
/// Each variant corresponds to a [`RecordKind`]. A single lookup returns only
/// records of the requested kind, so matching one variant is enough in
/// practice, but the enum is [`non_exhaustive`] so that adding kinds later is
/// not a breaking change.
///
/// [`DnsResolver::lookup_record`]: crate::DnsResolver::lookup_record
/// [`non_exhaustive`]: https://doc.rust-lang.org/reference/attributes/type_system.html
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Record {
    /// An IPv4 address from an A record.
    A(Ipv4Addr),
    /// An IPv6 address from an AAAA record.
    Aaaa(Ipv6Addr),
    /// The character strings of a TXT record.
    Txt(TxtRecordData),
    /// The name of an authoritative name server from an NS record.
    Ns(String),
    /// The target of an SRV record.
    Srv(SrvRecordData),
    /// The mail exchange of an MX record.
    Mx(MxRecordData),
    /// The policy of a CAA record.
    Caa(CaaRecordData),
    /// The service binding of an SVCB record.
    Svcb(SvcbRecordData),
    /// The service binding of an HTTPS record.
    Https(HttpsRecordData),
}

/// Record data for an SRV record, as defined in [RFC 2782].
///
/// [RFC 2782]: https://datatracker.ietf.org/doc/html/rfc2782
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrvRecordData {
    /// The priority of this target; lower values are preferred.
    pub priority: u16,
    /// The relative weight for targets that share a priority.
    pub weight: u16,
    /// The port on the target host for this service.
    pub port: u16,
    /// The domain name of the target host.
    pub target: String,
}

/// Record data for an MX record, as defined in [RFC 1035 Section 3.3.9].
///
/// [RFC 1035 Section 3.3.9]: https://datatracker.ietf.org/doc/html/rfc1035#section-3.3.9
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MxRecordData {
    /// The preference given to this mail exchange; lower values are preferred.
    pub preference: u16,
    /// The domain name of a host willing to act as a mail exchange.
    pub exchange: String,
}

/// Record data for a CAA record, as defined in [RFC 8659].
///
/// The resolver returns the record as it appears on the wire and does not
/// validate or interpret the `tag` or `value`.
///
/// [RFC 8659]: https://datatracker.ietf.org/doc/html/rfc8659
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaaRecordData {
    /// The flags byte, whose high bit marks the record as critical.
    pub flag: u8,
    /// The property described by `value`, such as `issue`, `issuewild`, or `iodef`.
    pub tag: String,
    /// The raw value associated with the property, as it appears on the wire.
    pub value: Box<[u8]>,
}

/// Record data for an SVCB or HTTPS record, as defined in [RFC 9460].
///
/// Wraps the parsed record and exposes the commonly used parameters through
/// accessor methods. A [`priority`](Self::priority) of 0 marks the record as
/// being in AliasMode, whose redirection this resolver does not follow, so such
/// a record is returned with its [`target`](Self::target) as-is.
///
/// [RFC 9460]: https://datatracker.ietf.org/doc/html/rfc9460
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvcbRecordData(SVCB<'static>);

impl SvcbRecordData {
    /// Wraps an owned `simple_dns` SVCB record.
    pub(crate) fn new(svcb: SVCB<'static>) -> Self {
        Self(svcb)
    }

    /// Returns the priority of this record.
    ///
    /// Lower values are preferred. A priority of 0 marks AliasMode.
    pub fn priority(&self) -> u16 {
        self.0.priority
    }

    /// Returns the target name.
    ///
    /// In AliasMode this is the alias target; in ServiceMode it is the
    /// alternative endpoint.
    pub fn target(&self) -> String {
        self.0.target.to_string()
    }

    /// Returns the application protocol identifiers from the `alpn` parameter.
    ///
    /// Empty when the parameter is absent. Each identifier is allocated as it is
    /// yielded, because `simple_dns` keeps them in a `CharacterString` that does
    /// not expose its bytes; the other parameter accessors borrow or copy.
    pub fn alpn(&self) -> impl Iterator<Item = String> + '_ {
        self.0
            .iter_params()
            .filter_map(|param| match param {
                SVCParam::Alpn(ids) => Some(ids.iter().map(|id| id.to_string())),
                _ => None,
            })
            .flatten()
    }

    /// Returns the port from the `port` parameter, if present.
    pub fn port(&self) -> Option<u16> {
        self.0.iter_params().find_map(|param| match param {
            SVCParam::Port(port) => Some(*port),
            _ => None,
        })
    }

    /// Returns the IPv4 addresses from the `ipv4hint` parameter.
    ///
    /// Empty when the parameter is absent.
    pub fn ipv4hint(&self) -> impl Iterator<Item = Ipv4Addr> + '_ {
        self.0
            .iter_params()
            .filter_map(|param| match param {
                SVCParam::Ipv4Hint(ips) => Some(ips.iter().map(|ip| Ipv4Addr::from(*ip))),
                _ => None,
            })
            .flatten()
    }

    /// Returns the IPv6 addresses from the `ipv6hint` parameter.
    ///
    /// Empty when the parameter is absent.
    pub fn ipv6hint(&self) -> impl Iterator<Item = Ipv6Addr> + '_ {
        self.0
            .iter_params()
            .filter_map(|param| match param {
                SVCParam::Ipv6Hint(ips) => Some(ips.iter().map(|ip| Ipv6Addr::from(*ip))),
                _ => None,
            })
            .flatten()
    }

    /// Returns the Encrypted ClientHello (ECH) config list, if present.
    ///
    /// The bytes are the raw `ECHConfigList` from the `ech` parameter, returned
    /// for a TLS client to consume. RFC 9460 registers the parameter but leaves
    /// the value opaque to DNS.
    pub fn ech(&self) -> Option<&[u8]> {
        self.0.iter_params().find_map(|param| match param {
            SVCParam::Ech(config) => Some(config.as_ref()),
            _ => None,
        })
    }

    /// Returns the SvcParamKeys the `mandatory` parameter marks as required.
    ///
    /// In ascending order, and empty when the parameter is absent. A client
    /// that does not understand every key listed here must treat the record as
    /// unusable (RFC 9460 Section 8).
    pub fn mandatory(&self) -> impl Iterator<Item = u16> + '_ {
        self.0
            .iter_params()
            .filter_map(|param| match param {
                SVCParam::Mandatory(keys) => Some(keys.iter().copied()),
                _ => None,
            })
            .flatten()
    }

    /// Returns whether the `no-default-alpn` parameter is set.
    ///
    /// When set, the scheme's default Application-Layer Protocol Negotiation
    /// (ALPN) protocol must not be assumed, and only [`Self::alpn`] applies.
    pub fn no_default_alpn(&self) -> bool {
        self.0
            .iter_params()
            .any(|param| matches!(param, SVCParam::NoDefaultAlpn))
    }
}

/// A parsed HTTPS record ([RFC 9460] type 65).
///
/// HTTPS records share the SVCB wire format but add HTTP-scheme semantics. This
/// wraps the underlying [`SvcbRecordData`], reachable through [`Self::svcb`] for
/// the raw parameters, together with the record's owner name. On top of that it
/// layers the HTTPS-specific rules: the AliasMode and ServiceMode distinction
/// (RFC 9460 Section 2.4), the `"."` target rule (Section 2.5), and the default
/// `http/1.1` ALPN (Section 7.1.1 and Section 9.5). It is what
/// [`DnsResolver::lookup_https`] returns.
///
/// [RFC 9460]: https://datatracker.ietf.org/doc/html/rfc9460
/// [`DnsResolver::lookup_https`]: crate::DnsResolver::lookup_https
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsRecordData {
    /// The name the record was found at.
    ///
    /// Needed to resolve a `"."` target, which means the owner (RFC 9460
    /// Section 2.5.2).
    owner: String,
    /// The SVCB-format payload the HTTPS rules are layered on top of.
    data: SvcbRecordData,
}

impl HttpsRecordData {
    /// Wraps `data` with the `owner` name the record was found at.
    pub(crate) fn new(owner: String, data: SvcbRecordData) -> Self {
        Self { owner, data }
    }

    /// The underlying SVCB-format record data, for the raw parameter accessors.
    pub fn svcb(&self) -> &SvcbRecordData {
        &self.data
    }

    /// Returns the `SvcPriority`.
    ///
    /// A value of 0 marks AliasMode; any other value marks ServiceMode (RFC
    /// 9460 Section 2.4).
    pub fn priority(&self) -> u16 {
        self.data.priority()
    }

    /// Returns whether this record is in AliasMode (`SvcPriority == 0`).
    ///
    /// An AliasMode record points at another name to resolve rather than
    /// describing an endpoint (RFC 9460 Section 2.4.2). This resolver does not chase the alias; read where it
    /// points with [`Self::effective_target`].
    pub fn is_alias(&self) -> bool {
        self.priority() == 0
    }

    /// Returns whether this record is in ServiceMode (`SvcPriority != 0`).
    ///
    /// A ServiceMode record describes an endpoint and its parameters (RFC 9460
    /// Section 2.4.3).
    pub fn is_service(&self) -> bool {
        self.priority() != 0
    }

    /// Returns the `TargetName` as it appears in the record.
    ///
    /// The root target (`.`) is reported as an empty string; see
    /// [`Self::effective_target`].
    pub fn target(&self) -> String {
        self.data.target()
    }

    /// The effective target name to connect to.
    ///
    /// A root `TargetName` (`.`, the empty string here) means "use this record's
    /// own owner name" in ServiceMode (RFC 9460 Section 2.5.2), so this returns
    /// the owner in that case and the target otherwise. In AliasMode a root target
    /// instead signals that the service does not exist (Section 2.5.1), so check
    /// [`Self::is_alias`] first when that distinction matters.
    pub fn effective_target(&self) -> String {
        let target = self.data.target();
        if target.is_empty() {
            self.owner.clone()
        } else {
            target
        }
    }

    /// Returns the ALPN protocol identifiers from the `alpn` parameter.
    ///
    /// Exactly as the record carries them, and empty when the parameter is
    /// absent. See [`Self::effective_alpn`] for the set with the HTTPS default
    /// applied.
    pub fn alpn(&self) -> impl Iterator<Item = String> + '_ {
        self.data.alpn()
    }

    /// Returns the effective ALPN protocol identifiers for this endpoint.
    ///
    /// This is the `alpn` list plus the HTTPS scheme default of `http/1.1`,
    /// unless the record sets `no-default-alpn`. RFC 9460 Section 7.1.1 gives
    /// the mechanism and Section 9.5 the `http/1.1` default. The default is not
    /// added when `http/1.1` is already listed. An AliasMode record describes no
    /// endpoint, so this is empty for it.
    ///
    /// Returns a `Vec` rather than an iterator, unlike [`Self::alpn`]: merging
    /// the scheme default in requires knowing whether the record already lists
    /// it, so the set cannot be produced lazily.
    pub fn effective_alpn(&self) -> Vec<String> {
        if self.is_alias() {
            return Vec::new();
        }
        let mut protocols: Vec<String> = self.data.alpn().collect();
        if !self.data.no_default_alpn() && !protocols.iter().any(|p| p == "http/1.1") {
            protocols.push("http/1.1".to_string());
        }
        protocols
    }

    /// Returns whether this endpoint advertises HTTP/2 (ALPN `h2`).
    ///
    /// Searches [`Self::alpn`] rather than [`Self::effective_alpn`]. The two
    /// agree here, because the only identifier the effective set adds is the
    /// `http/1.1` scheme default, and searching the raw list stops at the first
    /// match instead of building the merged set.
    pub fn supports_http2(&self) -> bool {
        !self.is_alias() && self.alpn().any(|protocol| protocol == "h2")
    }

    /// Returns whether this endpoint advertises HTTP/3 (ALPN `h3`).
    ///
    /// Searches [`Self::alpn`] for the same reason as [`Self::supports_http2`].
    pub fn supports_http3(&self) -> bool {
        !self.is_alias() && self.alpn().any(|protocol| protocol == "h3")
    }

    /// The port from the `port` parameter, if present.
    pub fn port(&self) -> Option<u16> {
        self.data.port()
    }

    /// The IPv4 address hints from the `ipv4hint` parameter.
    pub fn ipv4hint(&self) -> impl Iterator<Item = Ipv4Addr> + '_ {
        self.data.ipv4hint()
    }

    /// The IPv6 address hints from the `ipv6hint` parameter.
    pub fn ipv6hint(&self) -> impl Iterator<Item = Ipv6Addr> + '_ {
        self.data.ipv6hint()
    }

    /// The Encrypted ClientHello config list from the `ech` parameter, if present.
    pub fn ech(&self) -> Option<&[u8]> {
        self.data.ech()
    }

    /// Returns the SvcParamKeys the `mandatory` parameter marks as required.
    ///
    /// See RFC 9460 Section 8.
    pub fn mandatory(&self) -> impl Iterator<Item = u16> + '_ {
        self.data.mandatory()
    }

    /// Whether the `no-default-alpn` parameter is set.
    pub fn no_default_alpn(&self) -> bool {
        self.data.no_default_alpn()
    }
}

/// Record data for a TXT record.
///
/// This contains a list of character strings, as defined in [RFC 1035 Section 3.3.14].
///
/// [`TxtRecordData`] implements [`fmt::Display`], so you can call [`ToString::to_string`] to
/// convert the record data into a string. This will parse each character string with
/// [`String::from_utf8_lossy`] and then concatenate all strings without a separator.
///
/// If you want to process each character string individually, use [`Self::iter`].
///
/// [RFC 1035 Section 3.3.14]: https://datatracker.ietf.org/doc/html/rfc1035#section-3.3.14
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtRecordData(Box<[Box<[u8]>]>);

impl TxtRecordData {
    /// Returns an iterator over the character strings contained in this TXT record.
    pub fn iter(&self) -> impl Iterator<Item = &[u8]> {
        self.0.iter().map(|x| x.as_ref())
    }

    /// Consumes the record and returns its character strings as boxed byte slices.
    ///
    /// This hands over the backing storage without copying, so a caller that
    /// wants owned bytes (for example to build its own wrapper type) avoids the
    /// per-string allocation that [`Self::iter`] followed by `collect` incurs.
    pub fn into_boxed_slices(self) -> Box<[Box<[u8]>]> {
        self.0
    }
}

impl<'a> IntoIterator for &'a TxtRecordData {
    type Item = &'a [u8];
    type IntoIter = std::iter::Map<std::slice::Iter<'a, Box<[u8]>>, fn(&'a Box<[u8]>) -> &'a [u8]>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|s| s.as_ref())
    }
}

impl fmt::Display for TxtRecordData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for s in self.iter() {
            write!(f, "{}", String::from_utf8_lossy(s))?
        }
        Ok(())
    }
}

impl FromIterator<Box<[u8]>> for TxtRecordData {
    fn from_iter<T: IntoIterator<Item = Box<[u8]>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl From<Vec<Box<[u8]>>> for TxtRecordData {
    fn from(value: Vec<Box<[u8]>>) -> Self {
        Self(value.into_boxed_slice())
    }
}

impl From<Vec<String>> for TxtRecordData {
    fn from(value: Vec<String>) -> Self {
        Self(
            value
                .into_iter()
                .map(|s| s.into_bytes().into_boxed_slice())
                .collect(),
        )
    }
}
