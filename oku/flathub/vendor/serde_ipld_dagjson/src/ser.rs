//! Serialization.
use std::{collections::BTreeMap, fmt, io};

use ipld_core::cid::{multibase::Base, serde::CID_SERDE_PRIVATE_IDENTIFIER, Cid};
use serde::{
    ser::{self, SerializeMap},
    Serialize,
};

use crate::{
    error::EncodeError,
    shared::{ReservedKeyMap, ReservedKeyValue},
};

/// Serializes a value to a vector.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, EncodeError>
where
    T: ser::Serialize + ?Sized,
{
    let mut writer = Vec::new();
    let mut json_serializer = serde_json::Serializer::new(&mut writer);
    let serializer = Serializer::new(&mut json_serializer);
    value.serialize(serializer)?;
    Ok(writer)
}

/// Serializes a value to a writer.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<(), EncodeError>
where
    W: io::Write,
    T: ser::Serialize,
{
    let mut json_serializer = serde_json::Serializer::new(writer);
    let serializer = Serializer::new(&mut json_serializer);
    Ok(value.serialize(serializer)?)
}

pub struct Serializer<S> {
    ser: S,
}

impl<S> Serializer<S> {
    pub fn new(serializer: S) -> Self {
        Self { ser: serializer }
    }
}

impl<S> ser::Serializer for Serializer<S>
where
    S: ser::Serializer,
{
    type Ok = S::Ok;
    type Error = S::Error;

    type SerializeSeq = Serializer<S::SerializeSeq>;
    type SerializeTuple = Serializer<S::SerializeTuple>;
    type SerializeTupleStruct = Serializer<S::SerializeTupleStruct>;
    type SerializeTupleVariant = Serializer<S::SerializeTupleVariant>;
    type SerializeMap = Serializer<S::SerializeMap>;
    type SerializeStruct = SerializeStructSorted<S>;
    type SerializeStructVariant = Serializer<S::SerializeStructVariant>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_bool(v)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_i8(v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_i16(v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_i32(v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_i64(v)
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_i128(v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_u8(v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_u16(v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_u32(v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_u64(v)
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_u128(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        // In DAG-JSON only finite floats are supported.
        if !v.is_finite() {
            Err(ser::Error::custom(
                "Float must be a finite number, not Infinity or NaN".to_string(),
            ))
        } else {
            self.ser.serialize_f32(v)
        }
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        // In DAG-JSON only finite floats are supported.
        if !v.is_finite() {
            Err(ser::Error::custom(
                "Float must be a finite number, not Infinity or NaN".to_string(),
            ))
        } else {
            self.ser.serialize_f64(v)
        }
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_char(v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_str(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let bytes = ReservedKeyMap {
            _slash: ReservedKeyValue::Bytes {
                bytes: Base::Base64.encode(v),
            },
        };
        bytes.serialize(self.ser)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_none()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_some(&SerializeRef::new(value))
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_unit()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.ser.serialize_unit_struct(name)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.ser
            .serialize_unit_variant(name, variant_index, variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        if name == CID_SERDE_PRIVATE_IDENTIFIER {
            value.serialize(CidSerializer(self.ser))
        } else {
            self.ser
                .serialize_newtype_struct(name, &SerializeRef::new(value))
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser
            .serialize_newtype_variant(name, variant_index, variant, &SerializeRef::new(value))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(Self::SerializeSeq::new(self.ser.serialize_seq(len)?))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(Self::SerializeTuple::new(self.ser.serialize_tuple(len)?))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(Self::SerializeTupleStruct::new(
            self.ser.serialize_tuple_struct(name, len)?,
        ))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(Self::SerializeTupleVariant::new(
            self.ser
                .serialize_tuple_variant(name, variant_index, variant, len)?,
        ))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(Self::SerializeMap::new(self.ser.serialize_map(len)?))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeStructSorted {
            ser: self.ser,
            map: BTreeMap::new(),
        })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(Self::SerializeStructVariant::new(
            self.ser
                .serialize_struct_variant(name, variant_index, variant, len)?,
        ))
    }

    fn collect_seq<I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        I: IntoIterator,
        I::Item: ser::Serialize,
    {
        let iter = iter.into_iter().map(SerializeSized::new);
        self.ser.collect_seq(iter)
    }

    fn collect_map<K, V, I>(self, iter: I) -> Result<Self::Ok, Self::Error>
    where
        K: ser::Serialize,
        V: ser::Serialize,
        I: IntoIterator<Item = (K, V)>,
    {
        let iter = iter
            .into_iter()
            .map(|(k, v)| (SerializeSized::new(k), SerializeSized::new(v)));
        self.ser.collect_map(iter)
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + fmt::Display,
    {
        self.ser.collect_str(value)
    }

    fn is_human_readable(&self) -> bool {
        self.ser.is_human_readable()
    }
}

struct SerializeRef<'a, T: ?Sized> {
    value: &'a T,
}

impl<'a, T: ?Sized> SerializeRef<'a, T> {
    fn new(value: &'a T) -> Self {
        Self { value }
    }
}

impl<'a, T> ser::Serialize for SerializeRef<'a, T>
where
    T: ?Sized + ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        ser::Serialize::serialize(self.value, Serializer::new(serializer))
    }
}

struct SerializeSized<T> {
    value: T,
}

impl<T> SerializeSized<T> {
    fn new(value: T) -> Self {
        SerializeSized { value }
    }
}

impl<T> ser::Serialize for SerializeSized<T>
where
    T: ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        ser::Serialize::serialize(&self.value, Serializer::new(serializer))
    }
}

impl<S> ser::SerializeSeq for Serializer<S>
where
    S: ser::SerializeSeq,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_element(&SerializeRef::new(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.end()
    }
}

impl<S> ser::SerializeTuple for Serializer<S>
where
    S: ser::SerializeTuple,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_element(&SerializeRef::new(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.end()
    }
}

impl<S> ser::SerializeTupleStruct for Serializer<S>
where
    S: ser::SerializeTupleStruct,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_field(&SerializeRef::new(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.end()
    }
}

impl<S> ser::SerializeTupleVariant for Serializer<S>
where
    S: ser::SerializeTupleVariant,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_field(&SerializeRef::new(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.end()
    }
}

impl<S> ser::SerializeMap for Serializer<S>
where
    S: ser::SerializeMap,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_key(&SerializeRef::new(key))
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_value(&SerializeRef::new(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.end()
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.ser
            .serialize_entry(&SerializeRef::new(key), &SerializeRef::new(value))
    }
}

pub struct SerializeStructSorted<S: ser::Serializer> {
    ser: S,
    map: BTreeMap<&'static str, Box<serde_json::value::RawValue>>,
}

impl<S> ser::SerializeStruct for SerializeStructSorted<S>
where
    S: ser::Serializer,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let bytes = to_vec(value).map_err(ser::Error::custom)?;
        let json_str = String::from_utf8(bytes).map_err(ser::Error::custom)?;
        let raw = serde_json::value::RawValue::from_string(json_str).map_err(ser::Error::custom)?;
        self.map.insert(key, raw);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = self.ser.serialize_map(Some(self.map.len()))?;
        for (key, raw) in &self.map {
            map.serialize_entry(key, raw.as_ref())?;
        }
        map.end()
    }
}

impl<S> ser::SerializeStructVariant for Serializer<S>
where
    S: ser::SerializeStructVariant,
{
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.ser.serialize_field(key, &SerializeRef::new(value))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.end()
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), Self::Error> {
        self.ser.skip_field(key)
    }
}

/// Serializing a CID correctly as DAG-JSON.
struct CidSerializer<S>(S);

impl<S> ser::Serializer for CidSerializer<S>
where
    S: ser::Serializer,
{
    type Ok = S::Ok;
    type Error = S::Error;

    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_i64(self, _value: i64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_u64(self, _value: u64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        let cid = Cid::try_from(value).map_err(|_| ser::Error::custom("Invalid CID"))?;
        let cid_json = ReservedKeyMap {
            _slash: ReservedKeyValue::Cid(cid.to_string()),
        };
        SerializeSized::new(cid_json).serialize(self.0)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_some<T: ?Sized + ser::Serialize>(
        self,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_unit_struct(self, _name: &str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_unit_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }

    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
        self,
        _name: &str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_tuple_struct(
        self,
        _name: &str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_struct(
        self,
        _name: &str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
    fn serialize_struct_variant(
        self,
        _name: &str,
        _variant_index: u32,
        _variant: &str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom("unreachable"))
    }
}
