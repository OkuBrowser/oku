use bincode_1_3::{deserialize, serialize, Error};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Bincode;

impl<T: Serialize> super::Encode<T> for Bincode {
    type Error = Error;
    fn encode(obj: &T) -> Result<Vec<u8>, Error> {
        Ok(serialize(obj)?)
    }
}

impl<T: for<'a> Deserialize<'a>> super::Decode<T> for Bincode {
    type Error = Error;
    fn decode(data: Vec<u8>) -> Result<T, Error> {
        Ok(deserialize(&data[..])?)
    }
}
