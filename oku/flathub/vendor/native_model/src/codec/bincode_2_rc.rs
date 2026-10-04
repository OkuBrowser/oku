use bincode_2_rc::{
    config,
    error::{DecodeError, EncodeError},
    serde::{decode_from_slice, encode_to_vec},
};
use serde::{Deserialize, Serialize};

pub struct Bincode;

impl<T: Serialize> super::Encode<T> for Bincode {
    type Error = EncodeError;
    fn encode(obj: &T) -> Result<Vec<u8>, EncodeError> {
        encode_to_vec(obj, config::standard())
    }
}

impl<T: for<'a> Deserialize<'a>> super::Decode<T> for Bincode {
    type Error = DecodeError;
    fn decode(data: Vec<u8>) -> Result<T, DecodeError> {
        Ok(decode_from_slice(&data, config::standard())?.0)
    }
}
