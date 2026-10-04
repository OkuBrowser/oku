//! The error type returned by the resolver.

use std::fmt;

use n0_error::stack_error;

use crate::resolver::TransportError;

/// An error returned while resolving a DNS record.
///
/// Each variant is a distinct, matchable failure reason. A transport-level
/// failure carries a typed [`TransportError`] source you can match on for the
/// specific cause, rather than an opaque error.
#[stack_error(derive, add_meta, std_sources)]
#[non_exhaustive]
pub enum Error {
    /// No nameservers were configured or discovered to query.
    #[error("no nameservers configured to query")]
    #[non_exhaustive]
    NoNameservers {},
    /// A nameserver did not answer within the per-attempt timeout.
    #[error("request timed out")]
    #[non_exhaustive]
    Timeout {},
    /// Every nameserver was tried and none returned a usable response.
    #[error("no nameserver returned a usable response")]
    #[non_exhaustive]
    NoResponse {},
    /// The domain name does not exist (NXDOMAIN).
    #[error("domain name does not exist (NXDOMAIN)")]
    #[non_exhaustive]
    NxDomain {},
    /// A nameserver answered with an error response code.
    #[error("nameserver returned error response code: {code}")]
    #[non_exhaustive]
    ServerError {
        /// The response code the nameserver returned.
        code: ResponseCode,
    },
    /// The response was malformed or did not match the query.
    #[error("invalid or malformed DNS response")]
    #[non_exhaustive]
    InvalidResponse {},
    /// The hostname could not be built into a valid DNS query.
    #[error("invalid domain name: {name}")]
    #[non_exhaustive]
    InvalidName {
        /// The hostname that could not be used as a DNS name.
        name: String,
    },
    /// A DoT or DoH nameserver was configured without a TLS client config.
    ///
    /// None could be built from a crypto provider either, so the query has no
    /// TLS settings to run with.
    #[error("no TLS config for DNS-over-TLS or DNS-over-HTTPS")]
    #[non_exhaustive]
    MissingTlsConfig {},
    /// A network or transport-level failure while talking to a nameserver.
    #[error("transport failure")]
    #[non_exhaustive]
    Transport {
        /// The specific transport failure.
        source: TransportError,
    },
}

/// A DNS response code (RCODE) explaining why a nameserver could not answer.
///
/// Only error codes reach this type. A successful `NoError` response is not an
/// error, and a non-existent name is reported as [`Error::NxDomain`] rather than
/// as a response code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResponseCode {
    /// The nameserver failed to process the query (SERVFAIL).
    ServerFailure,
    /// The nameserver refused to answer, for example by policy (REFUSED).
    Refused,
    /// The query was malformed (FORMERR).
    FormatError,
    /// The nameserver does not support the requested operation (NOTIMP).
    NotImplemented,
    /// Another, less common response code.
    Other,
}

impl fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ResponseCode::ServerFailure => "SERVFAIL",
            ResponseCode::Refused => "REFUSED",
            ResponseCode::FormatError => "FORMERR",
            ResponseCode::NotImplemented => "NOTIMP",
            ResponseCode::Other => "other response code",
        };
        f.write_str(s)
    }
}
