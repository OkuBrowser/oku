use std::fmt;

use serde::{de, ser};

#[derive(Debug)]
pub enum EncodeError {
    Message(String),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for EncodeError {}

impl ser::Error for EncodeError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Message(message.to_string())
    }
}

impl From<serde_json::Error> for EncodeError {
    fn from(error: serde_json::Error) -> Self {
        Self::Message(error.to_string())
    }
}

#[derive(Debug)]
pub enum DecodeError {
    Message(String),
    TrailingData,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(message) => write!(f, "{}", message),
            Self::TrailingData => write!(f, "trailing data"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl de::Error for DecodeError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Message(message.to_string())
    }
}

impl From<serde_json::Error> for DecodeError {
    fn from(error: serde_json::Error) -> Self {
        Self::Message(error.to_string())
    }
}

/// Encode and Decode error combined.
#[derive(Debug)]
pub enum CodecError {
    /// A decoding error.
    Decode(DecodeError),
    /// An encoding error.
    Encode(EncodeError),
    /// An error from within `serde_json`.
    SerdeJson(String),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "decode error: {}", error),
            Self::Encode(error) => write!(f, "encode error: {}", error),
            Self::SerdeJson(error) => write!(f, "serde_json error: {}", error),
        }
    }
}

impl std::error::Error for CodecError {}

impl From<DecodeError> for CodecError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}

impl From<EncodeError> for CodecError {
    fn from(error: EncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<serde_json::Error> for CodecError {
    fn from(error: serde_json::Error) -> Self {
        Self::SerdeJson(error.to_string())
    }
}
