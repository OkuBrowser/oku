//! Deserialization.
#[cfg(not(feature = "std"))]
use alloc::borrow::Cow;
use core::convert::{Infallible, TryFrom};
use core::marker::PhantomData;
use serde::Deserialize;
#[cfg(feature = "std")]
use std::borrow::Cow;

use cbor4ii::core::dec::{self, Decode};
use cbor4ii::core::{major, types, utils::SliceReader};
use ipld_core::cid::serde::CID_SERDE_PRIVATE_IDENTIFIER;
use serde::de::{self, Visitor};

use crate::cbor4ii_nonpub::{marker, peek_one, pull_one};
use crate::error::DecodeError;
use crate::CBOR_TAGS_CID;
#[cfg(feature = "std")]
use cbor4ii::core::utils::IoReader;

/// Decodes a value from CBOR data in a slice.
///
/// # Examples
///
/// Deserialize a `String`
///
/// ```
/// # use serde_ipld_dagcbor::de;
/// let v: Vec<u8> = vec![0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72];
/// let value: String = de::from_slice(&v[..]).unwrap();
/// assert_eq!(value, "foobar");
/// ```
///
/// Deserialize a borrowed string with zero copies.
///
/// ```
/// # use serde_ipld_dagcbor::de;
/// let v: Vec<u8> = vec![0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72];
/// let value: &str = de::from_slice(&v[..]).unwrap();
/// assert_eq!(value, "foobar");
/// ```
pub fn from_slice<'a, T>(buf: &'a [u8]) -> Result<T, DecodeError<Infallible>>
where
    T: de::Deserialize<'a>,
{
    let reader = SliceReader::new(buf);
    let mut deserializer = Deserializer::from_reader(reader);
    let value = serde::Deserialize::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

/// Decodes a value from CBOR data in a reader.
///
/// # Examples
///
/// Deserialize a `String`
///
/// ```
/// # use serde_ipld_dagcbor::de;
/// let v: Vec<u8> = vec![0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72];
/// let value: String = de::from_reader(&v[..]).unwrap();
/// assert_eq!(value, "foobar");
/// ```
///
/// Note that `from_reader` cannot borrow data:
///
/// ```compile_fail
/// # use serde_ipld_dagcbor::de;
/// let v: Vec<u8> = vec![0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72];
/// let value: &str = de::from_reader(&v[..]).unwrap();
/// assert_eq!(value, "foobar");
/// ```
#[cfg(feature = "std")]
pub fn from_reader<T, R>(reader: R) -> Result<T, DecodeError<std::io::Error>>
where
    T: de::DeserializeOwned,
    R: std::io::BufRead,
{
    let reader = IoReader::new(reader);
    let mut deserializer = Deserializer::from_reader(reader);
    let value = serde::Deserialize::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

/// Decodes a single value from CBOR data in a reader. If there are multiple
/// concatenated values in the reader, this function will succeed. On success,
/// it returns the decoded value. The reader will be left with all trailing
/// data. This function allows a user to decode multiple CBOR values from the
/// same reader that are not the same type. If you have many values of the same
/// type, consider [`StreamDeserializer`].
///
/// # Examples
///
/// Deserialize a `String`
///
/// ```
/// # use serde_ipld_dagcbor::de;
/// let v: &[u8] = &[0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72];
/// let mut reader = std::io::Cursor::new(v);
/// let value: String = de::from_reader_once(&mut reader).unwrap();
/// assert_eq!(value, "foobar");
/// assert_eq!(v.len(), reader.position() as usize);
///
/// let v: &[u8] = &[0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72, 0x0A];
/// let mut reader = std::io::Cursor::new(v);
/// let value_1: String = de::from_reader_once(&mut reader).unwrap();
/// let value_2: i32 = de::from_reader_once(&mut reader).unwrap();
/// assert_eq!(value_1, "foobar");
/// assert_eq!(value_2, 10);
/// assert_eq!(v.len(), reader.position() as usize);
/// ```
#[cfg(feature = "std")]
pub fn from_reader_once<T, R>(reader: R) -> Result<T, DecodeError<std::io::Error>>
where
    T: de::DeserializeOwned,
    R: std::io::BufRead,
{
    let reader = IoReader::new(reader);
    let mut deserializer = Deserializer::from_reader(reader);
    let value = serde::Deserialize::deserialize(&mut deserializer)?;
    Ok(value)
}

/// Create an iterator over the CBOR values in the reader.
///
/// # Examples
///
/// Deserialize several `String` values
///
/// ```
/// # use serde_ipld_dagcbor::de;
/// let v: &[u8] = &[0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72];
/// let mut reader = std::io::Cursor::new(v);
/// let mut iter = de::iter_from_reader::<String, _>(&mut reader);
/// let value: String = iter.next().unwrap().unwrap();
/// assert_eq!(value, "foobar");
/// assert!(iter.next().is_none());
///
/// let v: &[u8] = &[0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72, 0x63, 0x62, 0x61, 0x7A];
/// let mut reader = std::io::Cursor::new(v);
/// let mut iter = de::iter_from_reader::<String, _>(&mut reader);
/// let value_1: String = iter.next().unwrap().unwrap();
/// let value_2: String = iter.next().unwrap().unwrap();
/// assert_eq!(value_1, "foobar");
/// assert_eq!(value_2, "baz");
/// assert!(iter.next().is_none());
/// ```
#[cfg(feature = "std")]
pub fn iter_from_reader<T, R>(reader: R) -> StreamDeserializer<'static, IoReader<R>, T>
where
    T: de::DeserializeOwned,
    R: std::io::BufRead,
{
    let reader = IoReader::new(reader);
    Deserializer::from_reader(reader).into_iter()
}

/// A Serde `Deserialize`r of DAG-CBOR data.
#[derive(Debug)]
pub struct Deserializer<R> {
    reader: R,
}

impl<R> Deserializer<R> {
    /// Constructs a `Deserializer` which reads from a `Read`er.
    pub fn from_reader(reader: R) -> Deserializer<R> {
        Deserializer { reader }
    }
}

impl<'de, R: dec::Read<'de>> Deserializer<R> {
    /// Create an iterator over all the CBOR values in the iterator
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter<T: Deserialize<'de>>(self) -> StreamDeserializer<'de, R, T> {
        // This cannot be an implementation of std::iter::IntoIterator because
        // we need the caller to choose what T is.
        StreamDeserializer::new(self)
    }
}

impl<'a> Deserializer<SliceReader<'a>> {
    /// Constructs a `Deserializer` that reads from a slice.
    pub fn from_slice(buf: &'a [u8]) -> Self {
        Deserializer {
            reader: SliceReader::new(buf),
        }
    }
}

impl<'de, R: dec::Read<'de>> Deserializer<R> {
    #[allow(clippy::type_complexity)]
    #[inline]
    fn try_step<'a>(
        &'a mut self,
    ) -> Result<scopeguard::ScopeGuard<&'a mut Self, fn(&'a mut Self) -> ()>, DecodeError<R::Error>>
    {
        if self.reader.step_in() {
            Ok(scopeguard::guard(self, |de| de.reader.step_out()))
        } else {
            Err(DecodeError::DepthLimit)
        }
    }

    #[inline]
    fn deserialize_cid<V>(&mut self, visitor: V) -> Result<V::Value, DecodeError<R::Error>>
    where
        V: Visitor<'de>,
    {
        let tag = dec::TagStart::decode(&mut self.reader)?;

        match tag.0 {
            CBOR_TAGS_CID => visitor.visit_newtype_struct(&mut CidDeserializer(self)),
            _ => Err(DecodeError::TypeMismatch {
                name: "CBOR tag",
                byte: tag.0 as u8,
            }),
        }
    }

    /// This method should be called after a value has been deserialized to ensure there is no
    /// trailing data in the input source.
    pub fn end(&mut self) -> Result<(), DecodeError<R::Error>> {
        match peek_one(&mut self.reader) {
            Ok(_) => Err(DecodeError::TrailingData),
            Err(DecodeError::Eof) => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn visit_seq<V>(
        &mut self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DecodeError<R::Error>>
    where
        V: Visitor<'de>,
    {
        let mut de = self.try_step()?;
        let mut seq = Accessor::array(&mut de)?;
        let value = seq.len;
        let res = visitor.visit_seq(&mut seq)?;
        match seq.len {
            0 => Ok(res),
            remaining => Err(DecodeError::RequireLength {
                name,
                expect: value - remaining,
                value,
            }),
        }
    }

    fn visit_map<V>(
        &mut self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, DecodeError<R::Error>>
    where
        V: Visitor<'de>,
    {
        let mut de = self.try_step()?;
        let mut map = Accessor::map(&mut de)?;
        let value = map.len;
        let res = visitor.visit_map(&mut map)?;
        match map.len {
            0 => Ok(res),
            remaining => Err(DecodeError::RequireLength {
                name,
                expect: value - remaining,
                value,
            }),
        }
    }
}

macro_rules! deserialize_type {
    ( @ $t:ty , $name:ident , $visit:ident ) => {
        #[inline]
        fn $name<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
        {
            let value = <$t>::decode(&mut self.reader)?;
            visitor.$visit(value)
        }
    };
    ( $( $t:ty , $name:ident , $visit:ident );* $( ; )? ) => {
        $(
            deserialize_type!(@ $t, $name, $visit);
        )*
    };
}

impl<'de, R: dec::Read<'de>> serde::Deserializer<'de> for &mut Deserializer<R> {
    type Error = DecodeError<R::Error>;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut de = self.try_step()?;
        let de = &mut *de;

        let byte = peek_one(&mut de.reader)?;
        if is_indefinite(byte) {
            return Err(DecodeError::IndefiniteSize);
        }
        match dec::if_major(byte) {
            major::UNSIGNED => de.deserialize_u64(visitor),
            major::NEGATIVE => {
                // CBOR supports negative integers up to -2^64 which is less than i64::MIN. Only
                // treat it as i128, if it is outside the i64 range.
                let value = i128::decode(&mut de.reader)?;
                match i64::try_from(value) {
                    Ok(value_i64) => visitor.visit_i64(value_i64),
                    Err(_) => visitor.visit_i128(value),
                }
            }
            major::BYTES => de.deserialize_byte_buf(visitor),
            major::STRING => de.deserialize_string(visitor),
            major::ARRAY => de.deserialize_seq(visitor),
            major::MAP => de.deserialize_map(visitor),
            // The only supported tag is tag 42 (CID).
            major::TAG => de.deserialize_cid(visitor),
            major::SIMPLE => match byte {
                marker::FALSE => {
                    de.reader.advance(1);
                    visitor.visit_bool(false)
                }
                marker::TRUE => {
                    de.reader.advance(1);
                    visitor.visit_bool(true)
                }
                marker::NULL => {
                    de.reader.advance(1);
                    visitor.visit_none()
                }
                marker::F32 => de.deserialize_f32(visitor),
                marker::F64 => de.deserialize_f64(visitor),
                _ => Err(DecodeError::Unsupported { byte }),
            },
            _ => Err(DecodeError::Unsupported { byte }),
        }
    }

    deserialize_type!(
        bool,       deserialize_bool,       visit_bool;

        i8,         deserialize_i8,         visit_i8;
        i16,        deserialize_i16,        visit_i16;
        i32,        deserialize_i32,        visit_i32;
        i64,        deserialize_i64,        visit_i64;
        i128,       deserialize_i128,       visit_i128;

        u8,         deserialize_u8,         visit_u8;
        u16,        deserialize_u16,        visit_u16;
        u32,        deserialize_u32,        visit_u32;
        u64,        deserialize_u64,        visit_u64;
        u128,       deserialize_u128,       visit_u128;

        // f32 deserialize is handled as a special case below
        f64,        deserialize_f64,        visit_f64;
    );

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // DAG-CBOR strictly requires all floats to be encoded as f64.
        // When deserializing to f32, we need to handle f64 encoding.
        // We also accept f32 encoding, although a strict DAG-CBOR
        // implementation should reject it (please don't write new data
        // with f32 encoding).
        let byte = peek_one(&mut self.reader)?;
        match byte {
            marker::F32 => {
                // Note: f32 encoding is not valid in strict DAG-CBOR.
                let value = <f32>::decode(&mut self.reader)?;
                visitor.visit_f32(value)
            }
            marker::F64 => {
                // DAG-CBOR always uses f64 encoding, even for f32 values
                let value = <f64>::decode(&mut self.reader)?;

                let f32_value = value as f32;

                // Check if conversion causes overflow to infinity
                if !f32_value.is_finite() && value.is_finite() {
                    // The f64 value is finite but becomes infinite when converted to f32
                    return Err(DecodeError::Msg("Float value out of range for f32".into()));
                }

                // Check if the f64 value is exactly representable as f32. Reject values that
                // lose precision in the conversion. If you must decode to f32, then you should
                // only encode f32 values.
                if (f32_value as f64).to_bits() != value.to_bits() {
                    return Err(DecodeError::Msg(
                        "Float value has more precision than f32 can represent, loss of precision"
                            .into(),
                    ));
                }

                visitor.visit_f32(f32_value)
            }
            _ => Err(DecodeError::TypeMismatch { name: "f32", byte }),
        }
    }

    #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Treat it as a String.
        // This is a bit wasteful when encountering strings of more than one character,
        // but we are optimistic this is a cold path.
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match <types::Bytes<Cow<[u8]>>>::decode(&mut self.reader)?.0 {
            Cow::Borrowed(buf) => visitor.visit_borrowed_bytes(buf),
            Cow::Owned(buf) => visitor.visit_byte_buf(buf),
        }
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match <Cow<str>>::decode(&mut self.reader)? {
            Cow::Borrowed(buf) => visitor.visit_borrowed_str(buf),
            Cow::Owned(buf) => visitor.visit_string(buf),
        }
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let byte = peek_one(&mut self.reader)?;
        if byte != marker::NULL {
            let mut de = self.try_step()?;
            visitor.visit_some(&mut **de)
        } else {
            self.reader.advance(1);
            visitor.visit_none()
        }
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let byte = pull_one(&mut self.reader)?;
        if byte == marker::NULL {
            visitor.visit_unit()
        } else {
            Err(DecodeError::TypeMismatch { name: "unit", byte })
        }
    }

    #[inline]
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if name == CID_SERDE_PRIVATE_IDENTIFIER {
            self.deserialize_cid(visitor)
        } else {
            visitor.visit_newtype_struct(self)
        }
    }

    #[inline]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.visit_seq("array", visitor)
    }

    #[inline]
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.visit_seq("tuple", visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.visit_seq(name, visitor)
    }

    #[inline]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.visit_map("map", visitor)
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.visit_map(name, visitor)
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut de = self.try_step()?;
        let accessor = EnumAccessor::enum_(&mut de)?;
        visitor.visit_enum(accessor)
    }

    #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ignore = dec::IgnoredAny::decode(&mut self.reader)?;
        visitor.visit_unit()
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
}

/// An iterator over all the CBOR values in the iterator.
pub struct StreamDeserializer<'de, R, T> {
    de: Deserializer<R>,
    output: PhantomData<fn() -> T>,
    lifetime: PhantomData<&'de ()>,
}

impl<R, T> StreamDeserializer<'_, R, T> {
    /// Create a new streaming deserializer.
    pub fn new(de: Deserializer<R>) -> Self {
        Self {
            de,
            output: PhantomData,
            lifetime: PhantomData,
        }
    }
}

impl<'de, R, T> Iterator for StreamDeserializer<'de, R, T>
where
    R: dec::Read<'de>,
    T: de::Deserialize<'de>,
{
    type Item = Result<T, DecodeError<R::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(()) = self.de.end() {
            return None;
        }

        let result = serde::Deserialize::deserialize(&mut self.de);

        match result {
            Ok(value) => Some(Ok(value)),
            Err(err) => Some(Err(err)),
        }
    }
}

struct Accessor<'a, R> {
    de: &'a mut Deserializer<R>,
    len: usize,
}

impl<'de, 'a, R: dec::Read<'de>> Accessor<'a, R> {
    #[inline]
    fn array(de: &'a mut Deserializer<R>) -> Result<Accessor<'a, R>, DecodeError<R::Error>> {
        let array_start = dec::ArrayStart::decode(&mut de.reader)?;
        array_start.0.map_or_else(
            || Err(DecodeError::IndefiniteSize),
            move |len| Ok(Accessor { de, len }),
        )
    }

    #[inline]
    fn map(de: &'a mut Deserializer<R>) -> Result<Accessor<'a, R>, DecodeError<R::Error>> {
        let map_start = dec::MapStart::decode(&mut de.reader)?;
        map_start.0.map_or_else(
            || Err(DecodeError::IndefiniteSize),
            move |len| Ok(Accessor { de, len }),
        )
    }
}

impl<'de, R> de::SeqAccess<'de> for Accessor<'_, R>
where
    R: dec::Read<'de>,
{
    type Error = DecodeError<R::Error>;

    #[inline]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.len > 0 {
            self.len -= 1;
            Ok(Some(seed.deserialize(&mut *self.de)?))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de, R: dec::Read<'de>> de::MapAccess<'de> for Accessor<'_, R> {
    type Error = DecodeError<R::Error>;

    #[inline]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.len > 0 {
            self.len -= 1;
            Ok(Some(seed.deserialize(&mut *self.de)?))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct EnumAccessor<'a, R> {
    de: &'a mut Deserializer<R>,
}

impl<'de, 'a, R: dec::Read<'de>> EnumAccessor<'a, R> {
    #[inline]
    pub fn enum_(
        de: &'a mut Deserializer<R>,
    ) -> Result<EnumAccessor<'a, R>, DecodeError<R::Error>> {
        let byte = peek_one(&mut de.reader)?;
        match dec::if_major(byte) {
            // string
            major::STRING => Ok(EnumAccessor { de }),
            // 1 length map
            major::MAP if byte == (major::MAP << 5) | 1 => {
                de.reader.advance(1);
                Ok(EnumAccessor { de })
            }
            _ => Err(DecodeError::TypeMismatch { name: "enum", byte }),
        }
    }
}

impl<'de, 'a, R> de::EnumAccess<'de> for EnumAccessor<'a, R>
where
    R: dec::Read<'de>,
{
    type Error = DecodeError<R::Error>;
    type Variant = EnumAccessor<'a, R>;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(&mut *self.de)?;
        Ok((variant, self))
    }
}

impl<'de, R> de::VariantAccess<'de> for EnumAccessor<'_, R>
where
    R: dec::Read<'de>,
{
    type Error = DecodeError<R::Error>;

    #[inline]
    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    #[inline]
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use serde::Deserializer;

        self.de.deserialize_tuple(len, visitor)
    }

    #[inline]
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use serde::Deserializer;

        self.de.deserialize_map(visitor)
    }
}

/// Deserialize a DAG-CBOR encoded CID.
///
/// This is without the CBOR tag information. It is only the CBOR byte string identifier (major
/// type 2), the number of bytes, and a null byte prefixed CID.
///
/// The reason for not including the CBOR tag information is the [`Value`] implementation. That one
/// starts to parse the bytes, before we could interfere. If the data only includes a CID, we are
/// parsing over the tag to determine whether it is a CID or not and go from there.
struct CidDeserializer<'a, R>(&'a mut Deserializer<R>);

impl<'de, 'a, R: dec::Read<'de>> de::Deserializer<'de> for &'a mut CidDeserializer<'a, R> {
    type Error = DecodeError<R::Error>;

    #[cfg(not(feature = "no-cid-as-bytes"))]
    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }
    #[cfg(feature = "no-cid-as-bytes")]
    fn deserialize_any<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(de::Error::custom(
            "Only bytes can be deserialized into a CID",
        ))
    }

    #[inline]
    fn deserialize_bytes<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let byte = peek_one(&mut self.0.reader)?;
        match dec::if_major(byte) {
            major::BYTES => {
                // CBOR encoded CIDs have a zero byte prefix we have to remove.
                match <types::Bytes<Cow<[u8]>>>::decode(&mut self.0.reader)?.0 {
                    Cow::Borrowed(buf) => {
                        if buf.len() <= 1 || buf[0] != 0 {
                            Err(DecodeError::Msg("Invalid CID".into()))
                        } else {
                            visitor.visit_borrowed_bytes(&buf[1..])
                        }
                    }
                    Cow::Owned(mut buf) => {
                        if buf.len() <= 1 || buf[0] != 0 {
                            Err(DecodeError::Msg("Invalid CID".into()))
                        } else {
                            buf.remove(0);
                            visitor.visit_byte_buf(buf)
                        }
                    }
                }
            }
            _ => Err(DecodeError::Unsupported { byte }),
        }
    }

    fn deserialize_newtype_struct<V: de::Visitor<'de>>(
        self,
        name: &str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if name == CID_SERDE_PRIVATE_IDENTIFIER {
            self.deserialize_bytes(visitor)
        } else {
            Err(de::Error::custom([
                "This deserializer must not be called on newtype structs other than one named `",
                CID_SERDE_PRIVATE_IDENTIFIER,
                "`"
            ].concat()))
        }
    }

    serde::forward_to_deserialize_any! {
        bool byte_buf char enum f32 f64 i8 i16 i32 i64 identifier ignored_any map option seq str
        string struct tuple tuple_struct u8 u16 u32 u64 unit unit_struct
    }
}

/// Check if byte is a major type with indefinite length.
#[inline]
pub fn is_indefinite(byte: u8) -> bool {
    byte & marker::START == marker::START
}
