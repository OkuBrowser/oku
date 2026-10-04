//! Structures for representing bencoded values with Rust data types.

use serde::de;
use serde::ser::{self, SerializeMap, SerializeSeq};
use serde_bytes::{ByteBuf, Bytes};
use std::collections::HashMap;
use std::fmt;

/// All possible values which may be serialized in bencode.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Value {
    /// A generic list of bytes.
    Bytes(Vec<u8>),

    /// An integer.
    Int(i64),

    /// A list of other bencoded values.
    List(Vec<Value>),

    /// A map of (key, value) pairs.
    Dict(HashMap<Vec<u8>, Value>),
}

impl ser::Serialize for Value {
    #[inline]
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            Value::Bytes(ref v) => s.serialize_bytes(v),
            Value::Int(v) => s.serialize_i64(v),
            Value::List(ref v) => {
                let mut seq = s.serialize_seq(Some(v.len()))?;
                for e in v {
                    seq.serialize_element(e)?;
                }
                seq.end()
            }
            Value::Dict(ref vs) => {
                let mut map = s.serialize_map(Some(vs.len()))?;
                for (k, v) in vs {
                    map.serialize_entry(&Bytes::new(k), v)?;
                }
                map.end()
            }
        }
    }
}

struct ValueVisitor;

impl<'de> de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any valid BEncode value")
    }

    #[inline]
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Int(value))
    }

    #[allow(clippy::cast_possible_wrap)]
    #[inline]
    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Int(value as i64))
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Bytes(value.into()))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::Bytes(value.into()))
    }

    #[inline]
    fn visit_bytes<E>(self, value: &[u8]) -> Result<Value, E> {
        Ok(Value::Bytes(value.into()))
    }

    #[inline]
    fn visit_seq<V>(self, mut access: V) -> Result<Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let mut seq = Vec::new();
        while let Some(e) = access.next_element()? {
            seq.push(e);
        }
        Ok(Value::List(seq))
    }

    #[inline]
    fn visit_map<V>(self, mut access: V) -> Result<Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut map = HashMap::new();
        while let Some((k, v)) = access.next_entry::<ByteBuf, _>()? {
            map.insert(k.into_vec(), v);
        }
        Ok(Value::Dict(map))
    }
}

impl<'de> de::Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Value {
        Value::Int(v)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::Bytes(s.into_bytes())
    }
}

impl<'a> From<&'a str> for Value {
    fn from(v: &str) -> Value {
        Value::Bytes(v.as_bytes().to_vec())
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Value {
        Value::Bytes(v)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Value {
        Value::List(v)
    }
}

impl From<HashMap<Vec<u8>, Value>> for Value {
    fn from(v: HashMap<Vec<u8>, Value>) -> Value {
        Value::Dict(v)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_test::{assert_tokens, Token};

    use super::Value;

    fn assert_bytes_eq(actual: &[u8], expected: &[u8]) {
        assert_eq!(
            actual,
            expected,
            "expected {:?} to equal {:?}",
            String::from_utf8_lossy(actual),
            String::from_utf8_lossy(expected)
        );
    }

    #[test]
    fn test_ser_de_bytes() {
        let int = Value::Bytes(b"1".to_vec());

        assert_tokens(&int, &[Token::Bytes(b"1")]);
    }

    #[test]
    fn test_ser_de_int() {
        let int = Value::Int(1);
        assert_tokens(&int, &[Token::I64(1)]);
    }

    #[test]
    fn test_ser_de_list() {
        let int = Value::List(vec![Value::Int(1)]);

        assert_tokens(
            &int,
            &[Token::Seq { len: Some(1) }, Token::I64(1), Token::SeqEnd],
        );
    }

    #[test]
    fn test_ser_de_dict() {
        let dict = Value::Dict(HashMap::from([(b"key".to_vec(), Value::Int(1))]));

        assert_tokens(
            &dict,
            &[
                Token::Map { len: Some(1) },
                Token::Bytes(b"key"),
                Token::I64(1),
                Token::MapEnd,
            ],
        );
    }

    mod it_should_be_converted_from {
        use std::collections::HashMap;

        use crate::value::Value;

        #[test]
        fn an_i64() {
            let value: Value = 11i64.into();
            assert_eq!(value, Value::Int(11));
        }

        #[test]
        fn a_string() {
            let value: Value = "11".into();
            assert_eq!(value, Value::Bytes(b"11".to_vec()));
        }

        #[test]
        fn a_str_reference() {
            let value: Value = "11".to_string().into();
            assert_eq!(value, Value::Bytes(b"11".to_vec()));
        }

        #[test]
        fn a_byte_vector() {
            let value: Value = vec![b'1', b'1'].into();
            assert_eq!(value, Value::Bytes(b"11".to_vec()));
        }

        #[test]
        fn a_vector_of_other_values() {
            let value: Value = vec![Value::Bytes(b"11".to_vec())].into();
            assert_eq!(value, Value::List(vec!(Value::Bytes(b"11".to_vec()))));
        }

        #[test]
        fn a_hash_map_of_other_values() {
            let value: Value = HashMap::from([(b"key".to_vec(), Value::Int(3))]).into();
            assert_eq!(
                value,
                Value::Dict(HashMap::from([(b"key".to_vec(), Value::Int(3))]))
            );
        }
    }

    mod for_serialization_and_deserialization_of_a {
        mod byte_string {

            mod empty {
                use crate::{from_bytes, Serializer};

                use crate::value::tests::assert_bytes_eq;
                use crate::value::Value;
                use serde::Serialize;

                #[test]
                fn serialization() {
                    let mut ser = Serializer::new();

                    let value = Value::Bytes(b"".to_vec());
                    let _unused = value.serialize(&mut ser);

                    assert_bytes_eq(ser.as_ref(), b"0:");
                }

                #[test]
                fn deserialization() {
                    let value: Value = from_bytes(b"0:").unwrap();

                    assert_eq!(value, Value::Bytes(b"".to_vec()));
                }
            }

            mod non_empty {
                use crate::{from_bytes, Serializer};

                use crate::value::tests::assert_bytes_eq;
                use crate::value::Value;
                use serde::Serialize;

                #[test]
                fn serialization() {
                    let mut ser = Serializer::new();

                    let value = Value::Bytes(b"spam".to_vec());
                    let _unused = value.serialize(&mut ser);

                    assert_bytes_eq(ser.as_ref(), b"4:spam");
                }

                #[test]
                fn deserialization() {
                    let value: Value = from_bytes(b"4:spam").unwrap();

                    assert_eq!(value, Value::Bytes(b"spam".to_vec()));
                }
            }
        }

        mod integer {

            mod positive {
                use serde::Serialize;

                use crate::{
                    from_bytes,
                    value::{tests::assert_bytes_eq, Value},
                    Serializer,
                };

                #[test]
                fn serialization() {
                    let mut ser = Serializer::new();

                    let value = Value::Int(3);
                    let _unused = value.serialize(&mut ser);

                    assert_bytes_eq(ser.as_ref(), b"i3e");
                }

                #[test]
                fn deserialization() {
                    let value: Value = from_bytes(b"i3e").unwrap();

                    assert_eq!(value, Value::Int(3));
                }
            }

            mod negative {
                use serde::Serialize;

                use crate::{
                    from_bytes,
                    value::{tests::assert_bytes_eq, Value},
                    Serializer,
                };

                #[test]
                fn serialization() {
                    let mut ser = Serializer::new();

                    let value = Value::Int(-3);
                    let _unused = value.serialize(&mut ser);

                    assert_bytes_eq(ser.as_ref(), b"i-3e");
                }

                #[test]
                fn deserialization() {
                    let value: Value = from_bytes(b"i-3e").unwrap();

                    assert_eq!(value, Value::Int(-3));
                }
            }
        }

        mod list {

            mod empty {
                use serde::Serialize;

                use crate::{
                    from_bytes,
                    value::{tests::assert_bytes_eq, Value},
                    Serializer,
                };

                #[test]
                fn serialization() {
                    let mut ser = Serializer::new();

                    let value = Value::List(vec![]);
                    let _unused = value.serialize(&mut ser);

                    assert_bytes_eq(ser.as_ref(), b"le");
                }

                #[test]
                fn deserialization() {
                    let value: Value = from_bytes(b"le").unwrap();

                    assert_eq!(value, Value::List(vec![]));
                }
            }

            mod with_integers {

                mod with_one_integer {
                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::List(vec![Value::Int(3)]);
                        let _unused = value.serialize(&mut ser);

                        assert_bytes_eq(ser.as_ref(), b"li3ee");
                    }

                    #[test]
                    fn deserialization() {
                        let value: Value = from_bytes(b"li3ee").unwrap();

                        assert_eq!(value, Value::List(vec![Value::Int(3)]));
                    }
                }

                mod with_multiple_integers {
                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::List(vec![Value::Int(1), Value::Int(2)]);
                        let _unused = value.serialize(&mut ser);

                        assert_bytes_eq(ser.as_ref(), b"li1ei2ee");
                    }

                    #[test]
                    fn deserialization() {
                        let value: Value = from_bytes(b"li1ei2ee").unwrap();

                        assert_eq!(value, Value::List(vec![Value::Int(1), Value::Int(2)]));
                    }
                }
            }

            mod with_byte_strings {

                mod empty {
                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::List(vec![Value::Bytes(b"".to_vec())]);
                        let _unused = value.serialize(&mut ser);

                        assert_bytes_eq(ser.as_ref(), b"l0:e");
                    }

                    #[test]
                    fn deserialization() {
                        let value: Value = from_bytes(b"l0:e").unwrap();

                        assert_eq!(value, Value::List(vec![Value::Bytes(b"".to_vec())]));
                    }
                }

                mod one_string {
                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::List(vec![Value::Bytes(b"spam".to_vec())]);
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"l4:spame");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"l4:spame").unwrap();

                        assert_eq!(value, Value::List(vec![Value::Bytes(b"spam".to_vec())]));
                    }
                }

                mod multiple_strings {
                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::List(vec![
                            Value::Bytes(b"spam1".to_vec()),
                            Value::Bytes(b"spam1".to_vec()),
                        ]);
                        let _unused = value.serialize(&mut ser);

                        assert_bytes_eq(ser.as_ref(), b"l5:spam15:spam1e");
                    }

                    #[test]
                    fn deserialization() {
                        let value: Value = from_bytes(b"l5:spam15:spam1e").unwrap();

                        assert_eq!(
                            value,
                            Value::List(vec![
                                Value::Bytes(b"spam1".to_vec()),
                                Value::Bytes(b"spam1".to_vec()),
                            ])
                        );
                    }
                }
            }

            mod with_dictionaries {

                mod empty {

                    use std::collections::HashMap;

                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::List(vec![Value::Dict(HashMap::new())]);
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"ldee");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"ldee").unwrap();

                        assert_eq!(value, Value::List(vec![Value::Dict(HashMap::new())]));
                    }
                }

                mod non_empty {
                    use std::collections::HashMap;

                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::List(vec![Value::Dict(HashMap::from([(
                            b"key".to_vec(),
                            Value::Int(3),
                        )]))]);
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"ld3:keyi3eee");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"ld3:keyi3eee").unwrap();

                        assert_eq!(
                            value,
                            Value::List(vec![Value::Dict(HashMap::from([(
                                b"key".to_vec(),
                                Value::Int(3),
                            )]))])
                        );
                    }
                }
            }
        }

        mod dictionary {

            mod empty {
                use std::collections::HashMap;

                use serde::Serialize;

                use crate::{
                    from_bytes,
                    value::{tests::assert_bytes_eq, Value},
                    Serializer,
                };

                #[test]
                fn serialization() {
                    let mut ser = Serializer::new();

                    let value = Value::Dict(HashMap::new());
                    let _unused = value.serialize(&mut ser);

                    assert_bytes_eq(ser.as_ref(), b"de");
                }

                #[test]
                fn deserialization() {
                    let value: Value = from_bytes(b"de").unwrap();

                    assert_eq!(value, Value::Dict(HashMap::new()));
                }
            }

            mod with_integer_keys {
                mod one_key {
                    use std::collections::HashMap;

                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::Dict(HashMap::from([(b"key".to_vec(), Value::Int(3))]));
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"d3:keyi3ee");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"d3:keyi3ee").unwrap();

                        assert_eq!(
                            value,
                            Value::Dict(HashMap::from([(b"key".to_vec(), Value::Int(3))]))
                        );
                    }
                }

                mod multiple_keys {
                    use std::collections::HashMap;

                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::Dict(HashMap::from([
                            (b"key1".to_vec(), Value::Int(1)),
                            (b"key2".to_vec(), Value::Int(2)),
                        ]));
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"d4:key1i1e4:key2i2ee");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"d4:key1i1e4:key2i2ee").unwrap();

                        assert_eq!(
                            value,
                            Value::Dict(HashMap::from([
                                (b"key1".to_vec(), Value::Int(1)),
                                (b"key2".to_vec(), Value::Int(2)),
                            ]))
                        );
                    }
                }
            }

            mod with_byte_string_keys {
                mod one_key {
                    use std::collections::HashMap;

                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::Dict(HashMap::from([(
                            b"key".to_vec(),
                            Value::Bytes(b"spam".to_vec()),
                        )]));
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"d3:key4:spame");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"d3:key4:spame").unwrap();

                        assert_eq!(
                            value,
                            Value::Dict(HashMap::from([(
                                b"key".to_vec(),
                                Value::Bytes(b"spam".to_vec()),
                            )]))
                        );
                    }
                }

                mod multiple_keys {
                    use std::collections::HashMap;

                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::Dict(HashMap::from([
                            (b"key1".to_vec(), Value::Bytes(b"spam1".to_vec())),
                            (b"key2".to_vec(), Value::Bytes(b"spam2".to_vec())),
                        ]));
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"d4:key15:spam14:key25:spam2e");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"d4:key15:spam14:key25:spam2e").unwrap();

                        assert_eq!(
                            value,
                            Value::Dict(HashMap::from([
                                (b"key1".to_vec(), Value::Bytes(b"spam1".to_vec())),
                                (b"key2".to_vec(), Value::Bytes(b"spam2".to_vec())),
                            ]))
                        );
                    }
                }
            }

            mod with_list_keys {
                mod empty {
                    use std::collections::HashMap;

                    use serde::Serialize;

                    use crate::{
                        from_bytes,
                        value::{tests::assert_bytes_eq, Value},
                        Serializer,
                    };

                    #[test]
                    fn serialization() {
                        let mut ser = Serializer::new();

                        let value = Value::Dict(HashMap::from([(
                            b"key".to_vec(),
                            Value::List(vec![Value::Int(1)]),
                        )]));
                        let _unused = value.serialize(&mut ser);

                        // cspell: disable-next-line
                        assert_bytes_eq(ser.as_ref(), b"d3:keyli1eee");
                    }

                    #[test]
                    fn deserialization() {
                        // cspell: disable-next-line
                        let value: Value = from_bytes(b"d3:keyli1eee").unwrap();

                        assert_eq!(
                            value,
                            Value::Dict(HashMap::from([(
                                b"key".to_vec(),
                                Value::List(vec![Value::Int(1)]),
                            )]))
                        );
                    }
                }

                mod non_empty {}
            }
        }
    }
}
