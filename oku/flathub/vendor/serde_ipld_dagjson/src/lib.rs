//! DAG-JSON serialization and deserialization.
pub mod codec;
pub mod de;
pub mod error;
pub mod ser;
mod shared;

pub use crate::de::{from_reader, from_slice, Deserializer};
pub use crate::error::{DecodeError, EncodeError};
pub use crate::ser::{to_vec, to_writer, Serializer};

/// The multicodec code for DAG-JSON.
pub const DAG_JSON_CODE: u64 = 0x129;
