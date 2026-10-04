#[cfg(feature = "kad")]
pub mod dht;
#[cfg(feature = "floodsub")]
pub mod floodsub;
#[cfg(feature = "gossipsub")]
pub mod gossipsub;

use libp2p::PeerId;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Display;
use thiserror::Error;

pub type ConnexaResult<T> = Result<T, Error>;

/// Connexa's structured error type.
#[derive(Debug, Error)]
pub enum Error<C = Infallible> {
    #[error("background task is no longer running")]
    ChannelClosed,

    #[error("{protocol} is not enabled")]
    Disabled { protocol: Protocol },

    #[error("not found: {0}")]
    NotFound(Cow<'static, str>),

    #[error("already exists: {0}")]
    AlreadyExists(Cow<'static, str>),

    #[error("not connected to peer {0}")]
    NotConnected(PeerId),

    #[error("invalid configuration: {0}")]
    InvalidConfig(Cow<'static, str>),

    #[error("operation timed out")]
    Timeout,

    #[cfg(feature = "kad")]
    #[error(transparent)]
    Dht(dht::Error),

    #[cfg(feature = "kad")]
    #[error("kademlia {0:?} failed")]
    Routing(dht::RoutingOp),

    #[cfg(feature = "gossipsub")]
    #[error(transparent)]
    Gossipsub(#[from] gossipsub::Error),

    #[cfg(feature = "floodsub")]
    #[error(transparent)]
    Floodsub(#[from] floodsub::Error),

    #[error(transparent)]
    Dial(other_error::ArcError<libp2p::swarm::DialError>),

    #[cfg(feature = "rendezvous")]
    #[error(transparent)]
    Rendezvous(#[from] rendezvous::Error),

    #[cfg(feature = "request-response")]
    #[error(transparent)]
    RequestResponse(#[from] libp2p::request_response::OutboundFailure),

    #[error(transparent)]
    Keystore(#[from] crate::keystore::Error),

    #[error(transparent)]
    Custom(C),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("keychain not provided")]
    KeychainNotProvided,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Protocol {
    #[cfg(feature = "gossipsub")]
    Gossipsub,
    #[cfg(feature = "floodsub")]
    Floodsub,
    #[cfg(feature = "kad")]
    Kademlia,
    #[cfg(feature = "rendezvous")]
    Rendezvous,
    #[cfg(feature = "request-response")]
    RequestResponse,
    #[cfg(feature = "autonat")]
    Autonat,
    #[cfg(feature = "relay")]
    Relay,
    #[cfg(feature = "relay")]
    Autorelay,
    #[cfg(feature = "stream")]
    Stream,
    Other(&'static str),
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "gossipsub")]
            Protocol::Gossipsub => write!(f, "gossipsub"),
            #[cfg(feature = "floodsub")]
            Protocol::Floodsub => write!(f, "floodsub"),
            #[cfg(feature = "kad")]
            Protocol::Kademlia => write!(f, "kad"),
            #[cfg(feature = "rendezvous")]
            Protocol::Rendezvous => write!(f, "rendezvous"),
            #[cfg(feature = "request-response")]
            Protocol::RequestResponse => write!(f, "request-response"),
            #[cfg(feature = "autonat")]
            Protocol::Autonat => write!(f, "autonat"),
            #[cfg(feature = "stream")]
            Protocol::Stream => write!(f, "stream"),
            #[cfg(feature = "relay")]
            Protocol::Relay => write!(f, "relay"),
            #[cfg(feature = "relay")]
            Protocol::Autorelay => write!(f, "autorelay"),
            Protocol::Other(s) => write!(f, "{s}"),
        }
    }
}

impl<C> From<Error<C>> for std::io::Error
where
    C: std::error::Error + Send + Sync + 'static,
{
    fn from(err: Error<C>) -> Self {
        use std::io::ErrorKind;
        match err {
            Error::Io(e) => e,
            Error::Keystore(crate::keystore::Error::Backend(e)) => e,
            other => {
                let kind = match &other {
                    Error::NotFound(_) => ErrorKind::NotFound,
                    Error::AlreadyExists(_) => ErrorKind::AlreadyExists,
                    Error::NotConnected(_) => ErrorKind::NotConnected,
                    Error::Timeout => ErrorKind::TimedOut,
                    Error::InvalidConfig(_) => ErrorKind::InvalidInput,
                    Error::ChannelClosed => ErrorKind::BrokenPipe,
                    #[cfg(feature = "kad")]
                    Error::Dht(_) | Error::Routing(_) => ErrorKind::Other,
                    Error::Dial(_) => ErrorKind::Other,
                    #[cfg(feature = "rendezvous")]
                    Error::Rendezvous(_) => ErrorKind::Other,
                    #[cfg(feature = "request-response")]
                    Error::RequestResponse(_) => ErrorKind::Other,
                    #[cfg(feature = "gossipsub")]
                    Error::Gossipsub(_) => ErrorKind::Other,
                    #[cfg(feature = "floodsub")]
                    Error::Floodsub(_) => ErrorKind::Other,
                    Error::Keystore(_) => ErrorKind::Other,
                    Error::Disabled { .. } | Error::Custom(_) => ErrorKind::Other,
                    Error::KeychainNotProvided => ErrorKind::Other,
                    Error::Io(_) => unreachable!("handled above"),
                };
                std::io::Error::new(kind, other)
            }
        }
    }
}

#[cfg(feature = "rendezvous")]
pub mod rendezvous {
    // Note: this for creating an Error representing `ErrorCode` since `ErrorCode` does not impl `Error`.
    // Ideally, this is to provide context for the error response, but may be used in the future for
    // a custom Error implementation

    use libp2p::rendezvous::ErrorCode;
    use std::error::Error as StdError;
    use std::fmt::Display;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum Error {
        InvalidNamespace,
        InvalidSignedPeerRecord,
        InvalidTtl,
        InvalidCookie,
        NotAuthorized,
        InternalError,
        Unavailable,
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::InvalidNamespace => write!(f, "Invalid namespace"),
                Error::InvalidSignedPeerRecord => write!(f, "Invalid signed peer record"),
                Error::InvalidTtl => write!(f, "Invalid ttl"),
                Error::InvalidCookie => write!(f, "Invalid cookie"),
                Error::NotAuthorized => write!(f, "Not authorized"),
                Error::InternalError => write!(f, "Internal error"),
                Error::Unavailable => write!(f, "Unavailable"),
            }
        }
    }

    impl StdError for Error {}

    impl From<ErrorCode> for Error {
        fn from(code: ErrorCode) -> Self {
            match code {
                ErrorCode::InvalidNamespace => Error::InvalidNamespace,
                ErrorCode::InvalidSignedPeerRecord => Error::InvalidSignedPeerRecord,
                ErrorCode::InvalidTtl => Error::InvalidTtl,
                ErrorCode::InvalidCookie => Error::InvalidCookie,
                ErrorCode::NotAuthorized => Error::NotAuthorized,
                ErrorCode::InternalError => Error::InternalError,
                ErrorCode::Unavailable => Error::Unavailable,
            }
        }
    }
}
