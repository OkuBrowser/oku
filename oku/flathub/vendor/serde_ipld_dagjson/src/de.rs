//! Deserialization.
use std::{fmt, io};

use ipld_core::cid::serde::CID_SERDE_PRIVATE_IDENTIFIER;
use serde::{
    de::{
        self,
        value::{BytesDeserializer, StringDeserializer},
    },
    Deserialize,
};

use crate::{
    error::DecodeError,
    shared::{ReservedKeyMap, ReservedKeyValue, ReservedKeyValueParsed},
};

/// Decodes a value from DAG-JSON data in a slice.
///
/// # Examples
///
/// Deserialize a `String`
///
/// ```
/// # use serde_ipld_dagjson::de;
/// let input = br#""foobar""#;
/// let value: String = de::from_slice(input).unwrap();
/// assert_eq!(value, "foobar");
/// ```
pub fn from_slice<'a, T>(buf: &'a [u8]) -> Result<T, DecodeError>
where
    T: de::Deserialize<'a>,
{
    let mut json_deserializer = serde_json::Deserializer::from_slice(buf);
    let deserializer = Deserializer::new(&mut json_deserializer);
    let value = T::deserialize(deserializer)?;
    json_deserializer
        .end()
        .map_err(|_| DecodeError::TrailingData)?;
    Ok(value)
}

/// Decodes a value from DAG-JSON data in a reader.
///
/// # Examples
///
/// Deserialize a `String`
///
/// ```
/// # use serde_ipld_dagjson::de;
/// let input = br#""foobar""#;
/// let value: String = de::from_reader(&input[..]).unwrap();
/// assert_eq!(value, "foobar");
/// ```
pub fn from_reader<T, R>(reader: R) -> Result<T, DecodeError>
where
    T: de::DeserializeOwned,
    R: io::Read,
{
    let mut json_deserializer = serde_json::Deserializer::from_reader(reader);
    let deserializer = Deserializer::new(&mut json_deserializer);
    let value = T::deserialize(deserializer)?;
    json_deserializer
        .end()
        .map_err(|_| DecodeError::TrailingData)?;
    Ok(value)
}

/// A Serde `Deserializer` of DAG-JSON data.
#[derive(Debug)]
pub struct Deserializer<D> {
    de: D,
}

impl<'de, D> Deserializer<D>
where
    D: de::Deserializer<'de>,
{
    pub fn new(de: D) -> Self {
        Self { de }
    }

    /// Deserialize a CID.
    fn deserialize_reserved_cid<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        let reserved = ReservedKeyMap::deserialize(self.de)?;
        match reserved._slash.parse()? {
            ReservedKeyValueParsed::Cid(cid) => {
                visitor.visit_newtype_struct(BytesDeserializer::new(&cid.to_bytes()))
            }
            _ => Err(de::Error::custom("Expected a CID")),
        }
    }

    /// Deserialize bytes.
    fn deserialize_reserved_bytes<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        let reserved = ReservedKeyMap::deserialize(self.de)?;
        match reserved._slash.parse()? {
            ReservedKeyValueParsed::Bytes(bytes) => visitor.visit_byte_buf(bytes),
            _ => Err(de::Error::custom("Expected bytes")),
        }
    }
}

impl<'de, D> de::Deserializer<'de> for Deserializer<D>
where
    D: de::Deserializer<'de>,
{
    type Error = D::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_any(Visitor::new(visitor))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_bool(Visitor::new(visitor))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_u8(Visitor::new(visitor))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_u16(Visitor::new(visitor))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_u32(Visitor::new(visitor))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_u64(Visitor::new(visitor))
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_u128(Visitor::new(visitor))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_i8(Visitor::new(visitor))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_i16(Visitor::new(visitor))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_i32(Visitor::new(visitor))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_i64(Visitor::new(visitor))
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_i128(Visitor::new(visitor))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_f32(Visitor::new(visitor))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_f64(Visitor::new(visitor))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_char(Visitor::new(visitor))
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_str(Visitor::new(visitor))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_string(Visitor::new(visitor))
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_reserved_bytes(Visitor::new(visitor))
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_reserved_bytes(Visitor::new(visitor))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_option(Visitor::new(visitor))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_unit(Visitor::new(visitor))
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_unit_struct(name, Visitor::new(visitor))
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        if name == CID_SERDE_PRIVATE_IDENTIFIER {
            self.deserialize_reserved_cid(Visitor::new(visitor))
        } else {
            visitor.visit_newtype_struct(self)
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_seq(Visitor::new(visitor))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_tuple(len, Visitor::new(visitor))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de
            .deserialize_tuple_struct(name, len, Visitor::new(visitor))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_map(Visitor::new(visitor))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de
            .deserialize_struct(name, fields, Visitor::new(visitor))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de
            .deserialize_enum(name, variants, Visitor::new(visitor))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_identifier(Visitor::new(visitor))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.de.deserialize_ignored_any(Visitor::new(visitor))
    }

    fn is_human_readable(&self) -> bool {
        self.de.is_human_readable()
    }
}

struct Visitor<V> {
    visitor: V,
}

impl<V> Visitor<V> {
    fn new(visitor: V) -> Self {
        Self { visitor }
    }
}

impl<'de, V> de::Visitor<'de> for Visitor<V>
where
    V: de::Visitor<'de>,
{
    type Value = V::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.visitor.expecting(formatter)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_bool(value)
    }

    fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_i8(value)
    }

    fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_i16(value)
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_i32(value)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_i64(value)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_i128(value)
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_u8(value)
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_u16(value)
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_u32(value)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_u64(value)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_u128(value)
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_f32(value)
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_f64(value)
    }

    fn visit_char<E>(self, value: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_char(value)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_str(value)
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_borrowed_str(value)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_string(value)
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_bytes(value)
    }

    fn visit_borrowed_bytes<E>(self, value: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_borrowed_bytes(value)
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_byte_buf(value)
    }
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor.visit_some(Deserializer::new(deserializer))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visitor.visit_unit()
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.visitor.visit_newtype_struct(deserializer)
    }

    fn visit_seq<A>(self, visitor: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        self.visitor.visit_seq(SeqAccess::new(visitor))
    }

    fn visit_map<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // Get the first key, if it's the reserved `"/"` one, deserialize in a a special way.
        let maybe_key = visitor.next_key::<String>()?;

        match maybe_key {
            Some(ref key) if key == "/" => {
                let value: ReservedKeyValue = visitor.next_value()?;
                match value.parse()? {
                    ReservedKeyValueParsed::Cid(cid) => self
                        .visitor
                        .visit_newtype_struct(BytesDeserializer::new(&cid.to_bytes())),
                    ReservedKeyValueParsed::Bytes(bytes) => self.visitor.visit_byte_buf(bytes),
                }
            }
            _ => self.visitor.visit_map(MapAccess::new(visitor, maybe_key)),
        }
    }

    fn visit_enum<A>(self, visitor: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        self.visitor.visit_enum(EnumAccess::new(visitor))
    }
}

struct DeserializeSeed<S> {
    seed: S,
}

impl<S> DeserializeSeed<S> {
    fn new(seed: S) -> Self {
        Self { seed }
    }
}

impl<'de, S> de::DeserializeSeed<'de> for DeserializeSeed<S>
where
    S: de::DeserializeSeed<'de>,
{
    type Value = S::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.seed.deserialize(Deserializer::new(deserializer))
    }
}

struct VariantAccess<D> {
    access: D,
}

impl<D> VariantAccess<D> {
    fn new(access: D) -> Self {
        Self { access }
    }
}

impl<'de, D> de::VariantAccess<'de> for VariantAccess<D>
where
    D: de::VariantAccess<'de>,
{
    type Error = D::Error;

    fn unit_variant(self) -> Result<(), D::Error> {
        self.access.unit_variant()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, D::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.access.newtype_variant_seed(DeserializeSeed::new(seed))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.access.tuple_variant(len, Visitor::new(visitor))
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: de::Visitor<'de>,
    {
        self.access.struct_variant(fields, Visitor::new(visitor))
    }
}

struct SeqAccess<D> {
    access: D,
}

impl<D> SeqAccess<D> {
    fn new(access: D) -> Self {
        Self { access }
    }
}

impl<'de, D> de::SeqAccess<'de> for SeqAccess<D>
where
    D: de::SeqAccess<'de>,
{
    type Error = D::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, D::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.access.next_element_seed(DeserializeSeed::new(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.access.size_hint()
    }
}

/// Parsed a map, may take an already parsed key.
///
/// Passing in a key makes it possible to peek into the map. In case you want to proceed parsing it
/// as a normal map, you can just pass thar key in. If you want to use the usual map parsing, pass
/// in `None` as key.
struct MapAccess<D> {
    access: D,
    parsed_key: Option<String>,
}

impl<D> MapAccess<D> {
    fn new(access: D, parsed_key: Option<String>) -> Self {
        Self { access, parsed_key }
    }
}

impl<'de, D> de::MapAccess<'de> for MapAccess<D>
where
    D: de::MapAccess<'de>,
{
    type Error = D::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, D::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        // With `take()` we make sure that only the very first key is a special case, all following
        // keys are just normal JSON.
        if let Some(parsed_key) = self.parsed_key.take() {
            seed.deserialize(StringDeserializer::new(parsed_key))
                .map(Some)
        } else {
            self.access.next_key_seed(DeserializeSeed::new(seed))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, D::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.access.next_value_seed(DeserializeSeed::new(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.access.size_hint()
    }
}

struct EnumAccess<D> {
    access: D,
}

impl<D> EnumAccess<D> {
    fn new(access: D) -> Self {
        EnumAccess { access }
    }
}

impl<'de, D> de::EnumAccess<'de> for EnumAccess<D>
where
    D: de::EnumAccess<'de>,
{
    type Error = D::Error;
    type Variant = VariantAccess<D::Variant>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), D::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.access
            .variant_seed(DeserializeSeed::new(seed))
            .map(|(value, access)| (value, VariantAccess::new(access)))
    }
}
