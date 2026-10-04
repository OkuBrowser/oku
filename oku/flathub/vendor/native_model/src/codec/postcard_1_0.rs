use postcard_1_0::{from_bytes, to_allocvec, Error};
use serde::{Deserialize, Serialize};

pub struct PostCard;

impl<T: Serialize> super::Encode<T> for PostCard {
    type Error = Error;
    fn encode(obj: &T) -> Result<Vec<u8>, Error> {
        Ok(to_allocvec(obj)?)
    }
}

impl<T: for<'a> Deserialize<'a>> super::Decode<T> for PostCard {
    type Error = Error;
    fn decode(data: Vec<u8>) -> Result<T, Error> {
        Ok(from_bytes(&data)?)
    }
}
