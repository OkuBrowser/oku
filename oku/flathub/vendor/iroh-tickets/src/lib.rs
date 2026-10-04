#![doc = include_str!("../README.md")]

use n0_error::{e, stack_error};

pub mod endpoint;

/// A ticket is a serializable object combining information required for an operation.
///
/// Tickets are convertible to and from a byte representation via [`encode_bytes`] /
/// [`decode_bytes`], and to and from a canonical string form (the lowercase [`KIND`]
/// prefix followed by base32 of the bytes) via [`encode_string`] / [`decode_string`].
/// Implementers only need to provide [`KIND`], [`encode_bytes`], and [`decode_bytes`].
///
/// Versioning is left to the implementer. Some kinds of tickets might need
/// versioning, others might not.
///
/// The serialization format for converting the ticket from and to bytes is left
/// to the implementer. We recommend using [postcard] for serialization.
///
/// [`KIND`]: Ticket::KIND
/// [`encode_bytes`]: Ticket::encode_bytes
/// [`decode_bytes`]: Ticket::decode_bytes
/// [`encode_string`]: Ticket::encode_string
/// [`decode_string`]: Ticket::decode_string
/// [postcard]: https://docs.rs/postcard/latest/postcard/
pub trait Ticket: Sized {
    /// String prefix describing the kind of iroh ticket.
    ///
    /// This should be lower case ascii characters.
    const KIND: &'static str;

    /// Encode the ticket into its byte representation.
    fn encode_bytes(&self) -> Vec<u8>;

    /// Decode a ticket from its byte representation.
    fn decode_bytes(bytes: &[u8]) -> Result<Self, ParseError>;

    /// Encode the ticket into its canonical string form.
    ///
    /// The default implementation produces the lowercase [`KIND`](Self::KIND) prefix
    /// followed by base32 (no padding) of [`encode_bytes`](Self::encode_bytes).
    /// Implementers may override this to use a different string encoding, in which
    /// case [`decode_string`](Self::decode_string) must be overridden to match.
    fn encode_string(&self) -> String {
        let mut out = Self::KIND.to_string();
        data_encoding::BASE32_NOPAD.encode_append(&self.encode_bytes(), &mut out);
        out.make_ascii_lowercase();
        out
    }

    /// Decode a ticket from its canonical string form.
    ///
    /// The default implementation expects the lowercase [`KIND`](Self::KIND) prefix
    /// followed by base32 (no padding) of the bytes accepted by
    /// [`decode_bytes`](Self::decode_bytes). Implementers that override
    /// [`encode_string`](Self::encode_string) must override this to match.
    fn decode_string(s: &str) -> Result<Self, ParseError> {
        let expected = Self::KIND;
        let Some(rest) = s.strip_prefix(expected) else {
            return Err(e!(ParseError::Kind { expected }));
        };
        let bytes = data_encoding::BASE32_NOPAD.decode(rest.to_ascii_uppercase().as_bytes())?;
        Self::decode_bytes(&bytes)
    }
}

/// An error deserializing an iroh ticket.
#[stack_error(derive, add_meta, from_sources)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum ParseError {
    /// Found a ticket with the wrong prefix, indicating the wrong kind.
    #[error("wrong prefix, expected {expected}")]
    Kind {
        /// The expected prefix.
        expected: &'static str,
    },
    /// This looks like a ticket, but postcard deserialization failed.
    #[error(transparent)]
    Postcard {
        #[error(source, std_err)]
        source: postcard::Error,
    },
    /// This looks like a ticket, but base32 decoding failed.
    #[error(transparent)]
    Encoding {
        #[error(source, std_err)]
        source: data_encoding::DecodeError,
    },
    /// Verification of the deserialized bytes failed.
    #[error("verification failed: {message}")]
    Verify { message: &'static str },
}

impl ParseError {
    /// Returns a [`ParseError`] that indicates the given ticket has the wrong
    /// prefix.
    ///
    /// Indicate the expected prefix.
    pub fn wrong_prefix(expected: &'static str) -> Self {
        e!(ParseError::Kind { expected })
    }

    /// Return a `ParseError` variant that indicates verification of the
    /// deserialized bytes failed.
    pub fn verification_failed(message: &'static str) -> Self {
        e!(ParseError::Verify { message })
    }
}
