#[cfg(all(feature = "serde", feature = "bincode_1_3"))]
pub mod bincode_1_3;
#[cfg(all(feature = "serde", feature = "bincode_2_rc"))]
pub mod bincode_2_rc;
#[cfg(all(feature = "serde", feature = "postcard_1_0"))]
pub mod postcard_1_0;

/// Encode trait for your own encoding method.
///
/// Example:
/// ```rust
///  use bincode_2_rc::{error::EncodeError,serde::encode_to_vec, config::standard};
/// use serde::Serialize;
/// pub struct Bincode;
///
/// impl<T: Serialize> native_model::Encode<T> for Bincode {
///     type Error = EncodeError;
///     fn encode(obj: &T) -> Result<Vec<u8>, EncodeError> {
///         Ok(encode_to_vec(&obj, standard())?)
///     }
/// }
/// ```
pub trait Encode<T> {
    type Error;
    fn encode(obj: &T) -> Result<Vec<u8>, Self::Error>;
}

/// Decode trait for your own decoding method.
///
/// Example:
/// ```rust
/// use bincode_2_rc::{error::DecodeError,serde::decode_from_slice, config::standard};
/// use serde::Deserialize;
/// pub struct Bincode;
///
/// impl<T: for<'a> Deserialize<'a>> native_model::Decode<T> for Bincode {
///     type Error = DecodeError;
///     fn decode(data: Vec<u8>) -> Result<T, DecodeError> {
///         Ok(decode_from_slice(&data, standard())?.0)
///     }
/// }
pub trait Decode<T> {
    type Error;
    fn decode(data: Vec<u8>) -> Result<T, Self::Error>;
}
