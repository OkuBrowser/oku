use crate::{WMIError, de::wbem_class_de::Deserializer, variant::Variant};
use serde::{
    Deserialize,
    de::{self, IntoDeserializer},
    forward_to_deserialize_any,
};
use std::{fmt, vec::IntoIter};

#[derive(Debug)]
struct SeqAccess {
    data: IntoIter<Variant>,
}

impl<'de> de::SeqAccess<'de> for SeqAccess {
    type Error = WMIError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.data.next() {
            Some(variant) => seed.deserialize(variant).map(Some),
            None => Ok(None),
        }
    }
}

impl<'de> serde::Deserializer<'de> for Variant {
    type Error = WMIError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Variant::Null => visitor.visit_none(),
            Variant::Empty => visitor.visit_unit(),
            Variant::String(s) => visitor.visit_string(s),
            Variant::I1(n) => visitor.visit_i8(n),
            Variant::I2(n) => visitor.visit_i16(n),
            Variant::I4(n) => visitor.visit_i32(n),
            Variant::I8(n) => visitor.visit_i64(n),
            Variant::R4(f) => visitor.visit_f32(f),
            Variant::R8(f) => visitor.visit_f64(f),
            Variant::Bool(b) => visitor.visit_bool(b),
            Variant::UI1(n) => visitor.visit_u8(n),
            Variant::UI2(n) => visitor.visit_u16(n),
            Variant::UI4(n) => visitor.visit_u32(n),
            Variant::UI8(n) => visitor.visit_u64(n),
            Variant::Array(v) => visitor.visit_seq(SeqAccess {
                data: v.into_iter(),
            }),
            _ => Err(WMIError::InvalidDeserializationVariantError(format!(
                "{:?}",
                self
            ))),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Variant::Null => visitor.visit_none(),
            Variant::Empty => visitor.visit_none(),
            some => visitor.visit_some(some),
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Variant::Object(o) => {
                Deserializer::from_wbem_class_obj(o).deserialize_struct(name, fields, visitor)
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Variant::Object(o) => {
                Deserializer::from_wbem_class_obj(o).deserialize_enum(name, variants, visitor)
            }
            Variant::String(str) => str
                .into_deserializer()
                .deserialize_enum(name, variants, visitor),
            _ => self.deserialize_any(visitor),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map identifier ignored_any
    }
}

impl<'de> Deserialize<'de> for Variant {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Variant, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VariantVisitor;

        impl<'de> de::Visitor<'de> for VariantVisitor {
            type Value = Variant;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any valid variant value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(Variant::Bool(value))
            }

            #[inline]
            fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E> {
                Ok(Variant::I1(value))
            }

            #[inline]
            fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E> {
                Ok(Variant::I2(value))
            }

            #[inline]
            fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E> {
                Ok(Variant::I4(value))
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(Variant::I8(value))
            }

            #[inline]
            fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E> {
                Ok(Variant::UI1(value))
            }

            #[inline]
            fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E> {
                Ok(Variant::UI2(value))
            }

            #[inline]
            fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E> {
                Ok(Variant::UI4(value))
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(Variant::UI8(value))
            }

            #[inline]
            fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E> {
                Ok(Variant::R4(value))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(Variant::R8(value))
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(Variant::String(value))
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(Variant::Null)
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Variant::Null)
            }

            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();

                while let Some(elem) = visitor.next_element()? {
                    vec.push(elem);
                }

                Ok(Variant::Array(vec))
            }

            fn visit_map<V>(self, mut _visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                // TODO: Add support for map type
                unimplemented!()
            }
        }

        deserializer.deserialize_any(VariantVisitor)
    }
}
