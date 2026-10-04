//! Serialize a Rust data structure into bencode data.

mod string;

use crate::error::{Error, Result};
use serde::ser;
use std::mem;
use std::str;

/// A structure for serializing Rust values into bencode.
#[derive(Default, Debug)]
pub struct Serializer {
    buf: Vec<u8>,
}

impl Serializer {
    /// Create a new serializer.
    #[must_use]
    pub fn new() -> Serializer {
        Self::default()
    }

    /// Consume the serializer and return the contents as a byte vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }

    fn push<T: AsRef<[u8]>>(&mut self, token: T) {
        self.buf.extend_from_slice(token.as_ref());
    }
}

impl AsRef<[u8]> for Serializer {
    fn as_ref(&self) -> &[u8] {
        self.buf.as_ref()
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        self.push("e");
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        self.push("ee");
        Ok(())
    }
}

#[doc(hidden)]
// todo: This should be pub(crate).
pub struct SerializeMap<'a> {
    ser: &'a mut Serializer,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    cur_key: Option<Vec<u8>>,
}

impl<'a> SerializeMap<'a> {
    pub fn new(ser: &'a mut Serializer, len: usize) -> SerializeMap<'_> {
        SerializeMap {
            ser,
            entries: Vec::with_capacity(len),
            cur_key: None,
        }
    }

    fn end_map(&mut self) -> Result<()> {
        if self.cur_key.is_some() {
            return Err(Error::InvalidValue(
                "`serialize_key` called without calling  `serialize_value`".to_string(),
            ));
        }
        let mut entries = mem::take(&mut self.entries);
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));
        self.ser.push("d");
        for (k, v) in entries {
            ser::Serializer::serialize_bytes(&mut *self.ser, k.as_ref())?;
            self.ser.push(v);
        }
        self.ser.push("e");
        Ok(())
    }
}

impl<'a> ser::SerializeMap for SerializeMap<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ?Sized + ser::Serialize>(&mut self, key: &T) -> Result<()> {
        if self.cur_key.is_some() {
            return Err(Error::InvalidValue(
                "`serialize_key` called multiple times without calling  `serialize_value`"
                    .to_string(),
            ));
        }
        self.cur_key = Some(key.serialize(&mut string::Serializer)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<()> {
        let key = self.cur_key.take().ok_or_else(|| {
            Error::InvalidValue(
                "`serialize_value` called without calling `serialize_key`".to_string(),
            )
        })?;
        let mut ser = Serializer::new();
        value.serialize(&mut ser)?;
        let value = ser.into_vec();
        if !value.is_empty() {
            self.entries.push((key, value));
        }
        Ok(())
    }
    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        if self.cur_key.is_some() {
            return Err(Error::InvalidValue(
                "`serialize_key` called multiple times without calling  `serialize_value`"
                    .to_string(),
            ));
        }
        let key = key.serialize(&mut string::Serializer)?;
        let mut ser = Serializer::new();
        value.serialize(&mut ser)?;
        let value = ser.into_vec();
        if !value.is_empty() {
            self.entries.push((key, value));
        }
        Ok(())
    }
    fn end(mut self) -> Result<()> {
        self.end_map()
    }
}

impl<'a> ser::SerializeStruct for SerializeMap<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + ser::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        ser::SerializeMap::serialize_entry(self, key, value)
    }
    fn end(mut self) -> Result<()> {
        self.end_map()
    }
}

impl<'a> ser::SerializeStructVariant for SerializeMap<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + ser::Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        ser::SerializeMap::serialize_entry(self, key, value)
    }
    fn end(mut self) -> Result<()> {
        self.end_map()?;
        self.ser.push("e");
        Ok(())
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = SerializeMap<'a>;
    type SerializeStruct = SerializeMap<'a>;
    type SerializeStructVariant = SerializeMap<'a>;

    fn serialize_bool(self, value: bool) -> Result<()> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i8(self, value: i8) -> Result<()> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i16(self, value: i16) -> Result<()> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i32(self, value: i32) -> Result<()> {
        self.serialize_i64(i64::from(value))
    }
    fn serialize_i64(self, value: i64) -> Result<()> {
        self.push("i");
        self.push(value.to_string());
        self.push("e");
        Ok(())
    }
    fn serialize_u8(self, value: u8) -> Result<()> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u16(self, value: u16) -> Result<()> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u32(self, value: u32) -> Result<()> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u64(self, value: u64) -> Result<()> {
        self.push("i");
        self.push(value.to_string());
        self.push("e");
        Ok(())
    }
    fn serialize_f32(self, _value: f32) -> Result<()> {
        Err(Error::InvalidValue("Cannot serialize f32".to_string()))
    }
    fn serialize_f64(self, _value: f64) -> Result<()> {
        Err(Error::InvalidValue("Cannot serialize f64".to_string()))
    }
    fn serialize_char(self, value: char) -> Result<()> {
        let mut buffer = [0; 4];
        self.serialize_bytes(value.encode_utf8(&mut buffer).as_bytes())
    }

    fn serialize_str(self, value: &str) -> Result<()> {
        self.serialize_bytes(value.as_bytes())
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        self.push(value.len().to_string());
        self.push(":");
        self.push(value);
        Ok(())
    }
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + ser::Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.push("d");
        self.serialize_bytes(variant.as_bytes())?;
        value.serialize(&mut *self)?;
        self.push("e");
        Ok(())
    }
    fn serialize_none(self) -> Result<()> {
        Ok(())
    }
    fn serialize_some<T: ?Sized + ser::Serialize>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self> {
        self.push("l");
        Ok(self)
    }
    fn serialize_tuple(self, size: usize) -> Result<Self> {
        self.serialize_seq(Some(size))
    }
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.push("d");
        self.serialize_bytes(variant.as_bytes())?;
        self.push("l");
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap::new(self, len.unwrap_or(0)))
    }
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.push("d");
        self.serialize_bytes(variant.as_bytes())?;
        Ok(SerializeMap::new(self, len))
    }
}

/// Serialize the given data into a bencode byte vector.
///
/// # Examples
/// ```
/// # fn main() -> Result<(), serde_bencode::Error> {
/// use serde_derive::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
/// struct Address {
///     street: String,
///     city: String,
/// }
///
/// let address = Address {
///     street: "1313 Webfoot Walk".to_string(),
///     city: "Duckburg, Calisota".to_string(),
/// };
///
/// let bytes = serde_bencode::to_bytes(&address)?;
/// assert_eq!(
///     String::from_utf8(bytes).unwrap(),
///     "d4:city18:Duckburg, Calisota6:street17:1313 Webfoot Walke",
/// );
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to fail or `T` contains
/// floating point values, which bencode cannot serialize.
pub fn to_bytes<T: ser::Serialize>(b: &T) -> Result<Vec<u8>> {
    let mut ser = Serializer::new();
    b.serialize(&mut ser)?;
    Ok(ser.into_vec())
}

/// Serialize the given data into a String of bencode.
///
/// # Examples
/// ```
/// # fn main() -> Result<(), serde_bencode::Error> {
/// use serde_derive::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
/// struct Address {
///     street: String,
///     city: String,
/// }
///
/// let address = Address {
///     street: "1313 Webfoot Walk".to_string(),
///     city: "Duckburg, Calisota".to_string(),
/// };
///
/// assert_eq!(
///     serde_bencode::to_string(&address)?,
///     "d4:city18:Duckburg, Calisota6:street17:1313 Webfoot Walke".to_string(),
/// );
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to fail or `T` contains
/// floating point values, which bencode cannot serialize.
pub fn to_string<T: ser::Serialize>(b: &T) -> Result<String> {
    let mut ser = Serializer::new();
    b.serialize(&mut ser)?;
    str::from_utf8(ser.as_ref())
        .map(std::string::ToString::to_string)
        .map_err(|_| Error::InvalidValue("Not an UTF-8".to_string()))
}
