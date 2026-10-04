use std::collections::BTreeMap;

use ipld_core::ipld::Ipld;
use serde::{Deserialize, Serialize};
use serde_ipld_dagcbor::{de, to_vec, DecodeError};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use std::convert::Infallible;

#[test]
fn test_string1() {
    let ipld: Result<Ipld, _> = de::from_slice(&[0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72]);
    assert_eq!(ipld.unwrap(), Ipld::String("foobar".to_string()));
}

#[test]
fn test_string2() {
    let ipld: Result<Ipld, _> = de::from_slice(&[
        0x71, 0x49, 0x20, 0x6d, 0x65, 0x74, 0x20, 0x61, 0x20, 0x74, 0x72, 0x61, 0x76, 0x65, 0x6c,
        0x6c, 0x65, 0x72,
    ]);
    assert_eq!(ipld.unwrap(), Ipld::String("I met a traveller".to_string()));
}

#[test]
fn test_string3() {
    let slice = b"\x78\x2fI met a traveller from an antique land who said";
    let ipld: Result<Ipld, _> = de::from_slice(slice);
    assert_eq!(
        ipld.unwrap(),
        Ipld::String("I met a traveller from an antique land who said".to_string())
    );
}

#[test]
fn test_byte_string() {
    let ipld: Result<Ipld, _> = de::from_slice(&[0x46, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72]);
    assert_eq!(ipld.unwrap(), Ipld::Bytes(b"foobar".to_vec()));
}

#[test]
fn test_numbers1() {
    let ipld: Result<Ipld, _> = de::from_slice(&[0x00]);
    assert_eq!(ipld.unwrap(), Ipld::Integer(0));
}

#[test]
fn test_numbers2() {
    let ipld: Result<Ipld, _> = de::from_slice(&[0x1a, 0x00, 0xbc, 0x61, 0x4e]);
    assert_eq!(ipld.unwrap(), Ipld::Integer(12345678));
}

#[test]
fn test_numbers3() {
    let ipld: Result<Ipld, _> = de::from_slice(&[0x39, 0x07, 0xde]);
    assert_eq!(ipld.unwrap(), Ipld::Integer(-2015));
}

#[test]
fn test_numbers_large_negative() {
    let ipld: Result<Ipld, _> =
        de::from_slice(&[0x3b, 0xa5, 0xf7, 0x02, 0xb3, 0xa5, 0xf7, 0x02, 0xb3]);
    let expected: i128 = -11959030306112471732;
    assert!(expected < i128::from(i64::MIN));
    assert_eq!(ipld.unwrap(), Ipld::Integer(expected));
}

#[test]
fn test_bool() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\xf4");
    assert_eq!(ipld.unwrap(), Ipld::Bool(false));
}

#[test]
fn test_trailing_bytes() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\xf4trailing");
    assert!(matches!(ipld.unwrap_err(), DecodeError::TrailingData));
}

#[test]
fn test_list1() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\x83\x01\x02\x03");
    assert_eq!(
        ipld.unwrap(),
        Ipld::List(vec![Ipld::Integer(1), Ipld::Integer(2), Ipld::Integer(3)])
    );
}

#[test]
fn test_list2() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\x82\x01\x82\x02\x81\x03");
    assert_eq!(
        ipld.unwrap(),
        Ipld::List(vec![
            Ipld::Integer(1),
            Ipld::List(vec![Ipld::Integer(2), Ipld::List(vec![Ipld::Integer(3)])])
        ])
    );
}

#[test]
fn test_object() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\xa5aaaAabaBacaCadaDaeaE");
    let mut object = BTreeMap::new();
    object.insert("a".to_string(), Ipld::String("A".to_string()));
    object.insert("b".to_string(), Ipld::String("B".to_string()));
    object.insert("c".to_string(), Ipld::String("C".to_string()));
    object.insert("d".to_string(), Ipld::String("D".to_string()));
    object.insert("e".to_string(), Ipld::String("E".to_string()));
    assert_eq!(ipld.unwrap(), Ipld::Map(object));
}

#[test]
fn test_indefinite_object_error() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\xbfaa\x01ab\x9f\x02\x03\xff\xff");
    let mut object = BTreeMap::new();
    object.insert("a".to_string(), Ipld::Integer(1));
    object.insert(
        "b".to_string(),
        Ipld::List(vec![Ipld::Integer(2), Ipld::Integer(3)]),
    );
    assert!(matches!(ipld.unwrap_err(), DecodeError::IndefiniteSize));
}

#[test]
fn test_indefinite_list_error() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\x9f\x01\x02\x03\xff");
    assert!(matches!(ipld.unwrap_err(), DecodeError::IndefiniteSize));
}

#[test]
fn test_indefinite_string_error() {
    let ipld: Result<Ipld, _> =
        de::from_slice(b"\x7f\x65Mary \x64Had \x62a \x67Little \x60\x64Lamb\xff");
    assert!(matches!(ipld.unwrap_err(), DecodeError::IndefiniteSize));
}

#[test]
fn test_indefinite_byte_string_error() {
    let ipld: Result<Ipld, _> = de::from_slice(b"\x5f\x42\x01\x23\x42\x45\x67\xff");
    assert!(matches!(ipld.unwrap_err(), DecodeError::IndefiniteSize));
}

#[test]
fn test_multiple_indefinite_strings_error() {
    let input = b"\x82\x7f\x65Mary \x64Had \x62a \x67Little \x60\x64Lamb\xff\x5f\x42\x01\x23\x42\x45\x67\xff";
    let ipld: Result<Ipld, _> = de::from_slice(input);
    assert!(matches!(ipld.unwrap_err(), DecodeError::IndefiniteSize));
}

#[test]
fn test_float() {
    // 0xfb (f64) followed by 8 bytes of IEEE 754 double precision for 100000.0
    let ipld: Result<Ipld, _> = de::from_slice(b"\xfb\x40\xf8\x6a\x00\x00\x00\x00\x00");
    assert_eq!(ipld.unwrap(), Ipld::Float(100000.0));
}

#[test]
fn test_rejected_tag() {
    let ipld: Result<Ipld, _> =
        de::from_slice(&[0xd9, 0xd9, 0xf7, 0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72]);
    assert!(matches!(
        ipld.unwrap_err(),
        DecodeError::TypeMismatch {
            name: "CBOR tag",
            byte: 0xf7
        }
    ));
}

#[test]
fn test_crazy_list() {
    let slice = b"\x86\x1b\x00\x00\x00\x1c\xbe\x99\x1d\xc7\x3b\x00\x7a\xcf\x51\xdc\x51\x70\xdb\x3a\x1b\x3a\x06\xdd\xf5\xf6\xfb\x41\x76\x5e\xb1\xf8\x00\x00\x00";
    let ipld: Vec<Ipld> = de::from_slice(slice).unwrap();
    assert_eq!(
        ipld,
        vec![
            Ipld::Integer(123456789959),
            Ipld::Integer(-34567897654325468),
            Ipld::Integer(-456787678),
            Ipld::Bool(true),
            Ipld::Null,
            Ipld::Float(23456543.5),
        ]
    );
}

#[test]
fn test_nan() {
    let ipld: Result<f64, _> = de::from_slice(b"\xf9\x7e\x00");
    assert!(matches!(
        ipld.unwrap_err(),
        DecodeError::TypeMismatch { .. }
    ));
}

#[test]
// The file was reported as not working by user kie0tauB
// but it parses to a cbor value.
fn test_kietaub_file() {
    let file = include_bytes!("kietaub.cbor");
    let value_result: Result<Ipld, _> = de::from_slice(file);
    value_result.unwrap();
}

#[test]
fn test_option_roundtrip() {
    let obj1 = Some(10u32);

    let v = to_vec(&obj1).unwrap();
    let obj2: Result<Option<u32>, _> = de::from_slice(&v[..]);
    println!("{:?}", obj2);

    assert_eq!(obj1, obj2.unwrap());
}

#[test]
fn test_option_none_roundtrip() {
    let obj1 = None;

    let v = to_vec(&obj1).unwrap();
    println!("{:?}", v);
    let obj2: Result<Option<u32>, _> = de::from_slice(&v[..]);

    assert_eq!(obj1, obj2.unwrap());
}

#[test]
fn test_unit() {
    #[allow(clippy::let_unit_value)]
    let unit = ();
    let v = to_vec(&unit).unwrap();
    assert_eq!(v, [0xf6], "unit is serialized as NULL.");
    let result: Result<(), _> = from_slice(&v);
    assert!(result.is_ok(), "unit was successfully deserialized");
}

#[test]
fn test_variable_length_map_error() {
    let slice = b"\xbf\x67\x6d\x65\x73\x73\x61\x67\x65\x64\x70\x6f\x6e\x67\xff";
    let ipld: Result<Ipld, _> = de::from_slice(slice);
    assert!(matches!(ipld.unwrap_err(), DecodeError::IndefiniteSize));
}

#[test]
fn test_object_determinism_roundtrip() {
    let expected = b"\xa2aa\x01ab\x82\x02\x03";

    // 0.1% chance of not catching failure
    for _ in 0..10 {
        assert_eq!(
            &to_vec(&de::from_slice::<Ipld>(expected).unwrap()).unwrap(),
            expected
        );
    }
}

#[cfg(feature = "std")]
#[test]
fn test_from_reader_once() {
    let v: &[u8] = &[0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72, 0x0a];
    let mut reader = std::io::Cursor::new(v);
    let value_1: String = de::from_reader_once(&mut reader).unwrap();
    assert_eq!(value_1, "foobar");
    let value_2: i32 = de::from_reader_once(&mut reader).unwrap();
    assert_eq!(value_2, 10);
    assert_eq!(v.len(), reader.position() as usize);
}

#[cfg(feature = "std")]
#[test]
fn test_stream_deserializer() {
    let v: &[u8] = &[
        0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72, 0x63, 0x62, 0x61, 0x7A,
    ];
    let reader = std::io::Cursor::new(v);
    let reader = cbor4ii::core::utils::IoReader::new(reader);
    let mut i = de::Deserializer::from_reader(reader).into_iter();
    let value_1: String = i.next().unwrap().unwrap();
    assert_eq!(value_1, "foobar");
    let value_2: String = i.next().unwrap().unwrap();
    assert_eq!(value_2, "baz");
    assert!(i.next().is_none());
}

#[cfg(feature = "std")]
#[test]
fn test_stream_deserializer_marker_traits() {
    use std::rc::Rc;

    fn is_send<T: Send>(_: &T) {}

    let v: &[u8] = &[
        0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72, 0x63, 0x62, 0x61, 0x7A,
    ];
    let reader = std::io::Cursor::new(v);
    let reader = cbor4ii::core::utils::IoReader::new(reader);
    let mut i = de::Deserializer::from_reader(reader).into_iter();
    is_send(&i);
    let value_1: Rc<String> = i.next().unwrap().unwrap();
    assert_eq!(value_1.as_str(), "foobar");
}

#[cfg(feature = "std")]
#[test]
fn test_stream_deserializer_trailing_data() {
    // one byte missing on the end
    let v: &[u8] = &[0x66, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72, 0x63, 0x62, 0x61];
    let reader = std::io::Cursor::new(v);
    let reader = cbor4ii::core::utils::IoReader::new(reader);
    let mut i = de::Deserializer::from_reader(reader).into_iter();
    let value_1: String = i.next().unwrap().unwrap();
    assert_eq!(value_1, "foobar");

    // we should get back an Eof error
    assert!(matches!(i.next(), Some(Err(DecodeError::Eof))));
}

#[test]
fn crash() {
    let file = include_bytes!("crash.cbor");
    let value_result: Result<Ipld, _> = de::from_slice(file);
    assert!(matches!(value_result.unwrap_err(), DecodeError::Eof));
}

use serde_ipld_dagcbor::de::from_slice;
use std::net::{IpAddr, Ipv4Addr};
#[test]
fn test_ipaddr_deserialization() {
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let buf = to_vec(&ip).unwrap();
    let deserialized_ip = from_slice::<IpAddr>(&buf).unwrap();
    assert_eq!(ip, deserialized_ip);
}

#[test]
fn attempt_stack_overflow() {
    // Create a tag 17, followed by 999 more tag 17:
    // 17(17(17(17(17(17(17(17(17(17(17(17(17(17(17(17(17(17(...
    // This causes deep recursion in the decoder and may
    // exhaust the stack and therfore result in a stack overflow.
    let input = vec![0xd1; 1000];
    serde_ipld_dagcbor::from_slice::<Ipld>(&input).expect_err("recursion limit");
}

#[test]
fn truncated_object() {
    let input: Vec<u8> = [
        &b"\x84\x87\xD8\x2A\x58\x27\x00\x01\x71\xA0\xE4\x02\x20\x83\xEC\x9F\x76\x1D"[..],
        &b"\xB5\xEE\xA0\xC8\xE1\xB5\x74\x0D\x1F\x0A\x1D\xB1\x8A\x52\x6B\xCB\x42\x69"[..],
        &b"\xFD\x99\x24\x9E\xCE\xA9\xE8\xFD\x24\xD8\x2A\x58\x27\x00\x01\x71\xA0\xE4"[..],
        &b"\x02\x20\xF1\x9B\xC1\x42\x83\x31\xB1\x39\xB3\x3F\x43\x02\x87\xCC\x1C\x12"[..],
        &b"\xF2\x84\x47\xA3\x9B\x07\x59\x40\x17\x68\xFE\xE8\x09\xBB\xF2\x54\xD8\x2A"[..],
        &b"\x58\x27\x00\x01\x71\xA0\xE4\x02\x20\xB0\x75\x09\x92\x78\x6B\x6B\x4C\xED"[..],
        &b"\xF0\xE1\x50\xA3\x1C\xAB\xDF\x25\xA9\x26\x8C\x63\xDD\xCB\x25\x73\x6B\xF5"[..],
        &b"\x8D\xE8\xA4\x24\x29"[..],
    ]
    .concat();
    serde_ipld_dagcbor::from_slice::<Ipld>(&input).expect_err("truncated");
}

#[test]
fn invalid_string() {
    // Non UTF-8 byte sequence, but using major type 3 (text string)
    let input = [0x63, 0xc5, 0x01, 0x02];
    let result = serde_ipld_dagcbor::from_slice::<Ipld>(&input);
    assert!(matches!(
        result.unwrap_err(),
        DecodeError::InvalidUtf8 { .. }
    ));
}

#[test]
fn error_on_undefined() {
    // CBOR smple type `undefined`
    let input = [0xf7];
    let result = serde_ipld_dagcbor::from_slice::<Ipld>(&input);
    assert!(matches!(
        result.unwrap_err(),
        DecodeError::Unsupported { .. }
    ));
}

// Test for default values inside tuple structs
#[derive(Debug, PartialEq, Deserialize_tuple, Serialize_tuple, Default, Clone)]
struct TupleWithDefaultsStruct {
    #[serde(default)]
    a: u32,
    #[serde(default)]
    b: String,
}

// Test for default values inside tuple structs nested inside other tuple structs
#[derive(Debug, PartialEq, Deserialize_tuple, Serialize_tuple, Clone)]
struct TupleOuterStruct {
    boop: u64,
    inner: TupleWithDefaultsStruct,
    #[serde(default)]
    bop: u64,
}

// Test for default values inside tuple structs nested inside other tuple structs where
// the outer struct has a default value itself
#[derive(Debug, PartialEq, Deserialize_tuple, Serialize_tuple, Clone)]
struct TupleOuterDefaultableStruct {
    boop: u64,
    #[serde(default)]
    inner: TupleWithDefaultsStruct,
}

// Test for tuple structs with overflow where the types are the same so the overflow could
// spill into the next field of the outer struct
#[derive(Debug, PartialEq, Deserialize_tuple, Serialize_tuple, Clone)]
struct TupleIntInner {
    a: u32,
    b: u32,
}

#[derive(Debug, PartialEq, Deserialize_tuple, Serialize_tuple, Clone)]
struct TupleIntOuterWithOverflow {
    inner: TupleIntInner,
    #[serde(default)]
    c: u32,
}

// Test for tuple structs with overflow where the types are the same so the overflow could
// spill into the next field of the outer struct
#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct MapIntInner {
    a: u32,
    b: u32,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct MapIntOuterWithOverflow {
    inner: MapIntInner,
    #[serde(default)]
    c: u32,
}

// The expected result is either an Ok value or an error check closure.
enum Expected<T> {
    Ok(T),
    Err(fn(&DecodeError<Infallible>) -> bool),
}

struct TestCase<T> {
    hex: &'static str,
    expected: Expected<T>,
}

#[test]
fn test_default_values() {
    let basic_cases = [
        // [] -> default
        TestCase {
            hex: "80",
            expected: Expected::Ok(TupleWithDefaultsStruct {
                a: 0,              // default
                b: "".to_string(), // default
            }),
        },
        // [101] -> set a and default b
        TestCase {
            hex: "811865",
            expected: Expected::Ok(TupleWithDefaultsStruct {
                a: 101,
                b: "".to_string(), // default
            }),
        },
        // [202, "yep"]
        TestCase {
            hex: "8218ca63796570",
            expected: Expected::Ok(TupleWithDefaultsStruct {
                a: 202,
                b: "yep".to_string(),
            }),
        },
        // [202,"nup",false] has too many elements so it errors with RequireLength
        TestCase {
            hex: "8318ca636e7570f4",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::RequireLength{ name, expect, value} if *name == "TupleWithDefaultsStruct" && *expect == 2 && *value == 3),
            ),
        },
    ];

    let outer_cases = [
        // [505,[],606]
        TestCase {
            hex: "831901f98019025e",
            expected: Expected::Ok(TupleOuterStruct {
                boop: 505,
                inner: TupleWithDefaultsStruct {
                    a: 0,              // default
                    b: "".to_string(), // default
                },
                bop: 606,
            }),
        },
        // [505,[202,"yep"],606]
        TestCase {
            hex: "831901f98218ca6379657019025e",
            expected: Expected::Ok(TupleOuterStruct {
                boop: 505,
                inner: TupleWithDefaultsStruct {
                    a: 202,
                    b: "yep".to_string(),
                },
                bop: 606,
            }),
        },
        // [505,[202,"nup",false],606] has too many elements on inner so it errors with RequireLength
        TestCase {
            hex: "831901f98318ca636e7570f419025e",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::RequireLength{ name, expect, value} if *name == "TupleWithDefaultsStruct" && *expect == 2 && *value == 3),
            ),
        },
        // [505,[]]
        TestCase {
            hex: "821901f980",
            expected: Expected::Ok(TupleOuterStruct {
                boop: 505,
                inner: TupleWithDefaultsStruct {
                    a: 0,              // default
                    b: "".to_string(), // default
                },
                bop: 0, // default
            }),
        },
        // [505,[202,"yep"]]
        TestCase {
            hex: "821901f98218ca63796570",
            expected: Expected::Ok(TupleOuterStruct {
                boop: 505,
                inner: TupleWithDefaultsStruct {
                    a: 202,
                    b: "yep".to_string(),
                },
                bop: 0, // default
            }),
        },
    ];

    let outer_defaultable_cases = [
        // [404] -> default inner
        TestCase {
            hex: "81190194",
            expected: Expected::Ok(TupleOuterDefaultableStruct {
                boop: 404,
                inner: TupleWithDefaultsStruct {
                    // default
                    a: 0,
                    b: "".to_string(),
                },
            }),
        },
        // [404,[]] -> default inner
        TestCase {
            hex: "8219019480",
            expected: Expected::Ok(TupleOuterDefaultableStruct {
                boop: 404,
                inner: TupleWithDefaultsStruct {
                    a: 0,              // default
                    b: "".to_string(), // default
                },
            }),
        },
        // [] -> error because inner has too few elements
        TestCase {
            hex: "80",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::Msg(ref m) if m == "invalid length 0, expected tuple struct Inner with 2 elements"),
            ),
        },
    ];

    let tuple_overflow_cases = [
        // [[1,2],3] -> expected layout
        TestCase {
            hex: "8282010203",
            expected: Expected::Ok(TupleIntOuterWithOverflow {
                inner: TupleIntInner { a: 1, b: 2 },
                c: 3,
            }),
        },
        // [[1],2] -> error because inner has too few elements
        TestCase {
            hex: "82820102",
            expected: Expected::Err(|err| matches!(err, DecodeError::Eof)),
        },
        // [[1,2,3],4] -> error because inner has too many elements
        TestCase {
            hex: "828301020304",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::RequireLength{ name, expect, value} if *name == "TupleIntInner" && *expect == 2 && *value == 3),
            ),
        },
        // [[1,2]] + 3 -> error because there's a trailing element
        TestCase {
            hex: "8182010203",
            expected: Expected::Err(|err| matches!(err, DecodeError::TrailingData)),
        },
        // [[1,2,3]] -> error because outer has too few elements
        TestCase {
            hex: "8183010203",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::RequireLength{ name, expect, value} if *name == "TupleIntInner" && *expect == 2 && *value == 3),
            ),
        },
    ];

    let map_overflow_cases = [
        // {"inner":{"a":1,"b":2},"c":3} -> expected layout
        TestCase {
            hex: "a261630365696e6e6572a2616101616202",
            expected: Expected::Ok(MapIntOuterWithOverflow {
                inner: MapIntInner { a: 1, b: 2 },
                c: 3,
            }),
        },
        // {"inner":{"a":1},"c":3} -> error because inner has too few elements
        TestCase {
            hex: "a261630365696e6e6572a1616101",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::Msg(ref m) if m == "missing field `b`"),
            ),
        },
        // {"inner":{"a":1,"b":2,"c":3},"c":4} -> error because inner has too many elements
        TestCase {
            hex: "a261630465696e6e6572a3616101616202616303",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::Msg(ref m) if m == "unknown field `c`, expected `a` or `b`"),
            ),
        },
        // {"inner":{"a":1,"b":2}} + "c":3 -> error because there's a trailing element
        TestCase {
            hex: "a165696e6e6572a2616101616202616303",
            expected: Expected::Err(|err| matches!(err, DecodeError::TrailingData)),
        },
        // {"inner":{"a":1,"b":2,"c":3}} -> error because outer has too few elements
        TestCase {
            hex: "a165696e6e6572a3616101616202616303",
            expected: Expected::Err(
                |err| matches!(err, DecodeError::Msg(ref m) if m == "unknown field `c`, expected `a` or `b`"),
            ),
        },
    ];

    check_cases(&basic_cases);
    check_cases(&outer_cases);
    check_cases(&outer_defaultable_cases);
    check_cases(&tuple_overflow_cases);
    check_cases(&map_overflow_cases);

    fn check_cases<T>(test_cases: &[TestCase<T>])
    where
        T: serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
    {
        for case in test_cases {
            let input = const_hex::decode(case.hex).unwrap();
            let result = from_slice::<T>(&input);
            match case.expected {
                Expected::Ok(ref expected_val) => match result {
                    Ok(val) => assert_eq!(val, *expected_val, "for input {}", case.hex),
                    Err(err) => panic!(
                        "for input {} expected success with {:?} but got error: {:?}",
                        case.hex, expected_val, err
                    ),
                },
                Expected::Err(check) => match result {
                    Ok(val) => panic!(
                        "for input {} expected an error but got success with value: {:?}",
                        case.hex, val
                    ),
                    Err(err) => assert!(
                        check(&err),
                        "for input {} got unexpected error: {:?}",
                        case.hex,
                        err
                    ),
                },
            }
        }
    }
}
