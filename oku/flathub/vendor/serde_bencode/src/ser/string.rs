//! Serializer for serializing *just* strings.

use crate::error::{Error, Result};
use serde::de;
use serde::ser;
use std::fmt;
use std::str;

struct Expected;
impl de::Expected for Expected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a string or bytes")
    }
}

fn unexpected<T>(unexp: de::Unexpected<'_>) -> Result<T> {
    Err(de::Error::invalid_type(unexp, &Expected))
}

#[doc(hidden)]
/// StringSerializer for serializing *just* strings (bytes are also strings in bencode).
/// The string is returned as Result<Vec<u8>>::Ok without any prefixing (without bencode string
/// length prefix).
// todo: This should be pub(crate).
pub struct Serializer;

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = Vec<u8>;
    type Error = Error;
    type SerializeSeq = ser::Impossible<Vec<u8>, Error>;
    type SerializeTuple = ser::Impossible<Vec<u8>, Error>;
    type SerializeTupleStruct = ser::Impossible<Vec<u8>, Error>;
    type SerializeTupleVariant = ser::Impossible<Vec<u8>, Error>;
    type SerializeMap = ser::Impossible<Vec<u8>, Error>;
    type SerializeStruct = ser::Impossible<Vec<u8>, Error>;
    type SerializeStructVariant = ser::Impossible<Vec<u8>, Error>;

    fn serialize_bool(self, value: bool) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::Bool(value))
    }
    fn serialize_i8(self, value: i8) -> Result<Vec<u8>> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i16(self, value: i16) -> Result<Vec<u8>> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i32(self, value: i32) -> Result<Vec<u8>> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i64(self, value: i64) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::Signed(value))
    }
    fn serialize_u8(self, value: u8) -> Result<Vec<u8>> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u16(self, value: u16) -> Result<Vec<u8>> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u32(self, value: u32) -> Result<Vec<u8>> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u64(self, value: u64) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::Unsigned(value))
    }
    fn serialize_f32(self, value: f32) -> Result<Vec<u8>> {
        self.serialize_f64(f64::from(value))
    }
    fn serialize_f64(self, value: f64) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::Float(value))
    }
    fn serialize_char(self, value: char) -> Result<Vec<u8>> {
        self.serialize_bytes(&[value as u8])
    }
    fn serialize_str(self, value: &str) -> Result<Vec<u8>> {
        self.serialize_bytes(value.as_bytes())
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<Vec<u8>> {
        Ok(value.into())
    }
    fn serialize_unit(self) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::Unit)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Vec<u8>> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::UnitVariant)
    }
    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::NewtypeStruct)
    }
    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::NewtypeVariant)
    }
    fn serialize_none(self) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::Option)
    }
    fn serialize_some<T: ?Sized + ser::Serialize>(self, _value: &T) -> Result<Vec<u8>> {
        unexpected(de::Unexpected::Option)
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<ser::Impossible<Vec<u8>, Error>> {
        unexpected(de::Unexpected::Seq)
    }
    fn serialize_tuple(self, _size: usize) -> Result<ser::Impossible<Vec<u8>, Error>> {
        unexpected(de::Unexpected::Seq)
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<ser::Impossible<Vec<u8>, Error>> {
        unexpected(de::Unexpected::NewtypeStruct)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<ser::Impossible<Vec<u8>, Error>> {
        unexpected(de::Unexpected::TupleVariant)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<ser::Impossible<Vec<u8>, Error>> {
        unexpected(de::Unexpected::Map)
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<ser::Impossible<Vec<u8>, Error>> {
        unexpected(de::Unexpected::NewtypeStruct)
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<ser::Impossible<Vec<u8>, Error>> {
        unexpected(de::Unexpected::StructVariant)
    }
}
