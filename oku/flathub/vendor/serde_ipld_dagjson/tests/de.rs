use std::{collections::BTreeMap, str::FromStr};

use ipld_core::{cid::Cid, ipld::Ipld};
use serde::Deserialize;
use serde_bytes::{ByteArray, ByteBuf};
use serde_ipld_dagjson::{de, to_vec, DecodeError};

#[test]
fn test_hello_world() {
    let data = br#"{"hello": "world!"}"#;
    let ipld: Ipld = de::from_slice(data).unwrap();
    let expected = Ipld::Map(BTreeMap::from([(
        "hello".to_string(),
        Ipld::String("world!".to_string()),
    )]));
    assert_eq!(ipld, expected);
}

#[test]
fn test_cid() {
    let data = br#"{"/": "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}"#;
    let ipld: Ipld = de::from_slice(data).unwrap();
    let expected = Ipld::Link(
        Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap(),
    );
    assert_eq!(ipld, expected);
}

#[test]
fn test_direct_cid() {
    let data = br#"{"/": "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}"#;
    let cid: Cid = de::from_slice(&data[..]).unwrap();
    let expected =
        Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap();
    assert_eq!(cid, expected);
}

#[test]
fn test_nested_cid() {
    let data =
        br#"{"hello": {"/": "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}}"#;
    let ipld: Ipld = de::from_slice(data).unwrap();
    let expected = Ipld::Map(BTreeMap::from([(
        "hello".to_string(),
        Ipld::Link(
            Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap(),
        ),
    )]));
    assert_eq!(ipld, expected);
}

#[test]
fn test_array_cid() {
    let data = br#"[{"/": "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}]"#;
    let ipld: Ipld = de::from_slice(data).unwrap();
    let expected = Ipld::List(vec![Ipld::Link(
        Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap(),
    )]);
    assert_eq!(ipld, expected);
}

#[test]
fn test_bytes() {
    let data = br#"{"/": { "bytes": "dm14"}}"#;
    let ipld: Ipld = de::from_slice(data).unwrap();
    let expected = Ipld::Bytes(vec![118, 109, 120]);
    assert_eq!(ipld, expected);
}

#[test]
fn test_nested_bytes() {
    let data = br#"{"nested": {"/": {"bytes": "dm14"}}}"#;
    let ipld: Ipld = de::from_slice(data).unwrap();
    let expected = Ipld::Map(BTreeMap::from([(
        "nested".to_string(),
        Ipld::Bytes(vec![118, 109, 120]),
    )]));
    assert_eq!(ipld, expected);
}

#[test]
fn test_direct_bytes() {
    let data = br#"{"/": {"bytes": "dm14"}}"#;
    let bytes: serde_bytes::ByteBuf = de::from_slice(data).unwrap();
    let expected = ByteBuf::from([118, 109, 120]);
    assert_eq!(bytes, expected);
}

#[test]
fn test_direct_byte_array() {
    let data = br#"{"/": {"bytes": "dm14"}}"#;
    let bytes: ByteArray<3> = de::from_slice(data).unwrap();
    let expected = ByteArray::new([118, 109, 120]);
    assert_eq!(bytes, expected);
}

#[test]
fn test_string() {
    let ipld: Ipld = de::from_slice(br#""foobar""#).unwrap();
    assert_eq!(ipld, Ipld::String("foobar".to_string()));
}

#[test]
fn test_numbers1() {
    let ipld: Ipld = de::from_slice(b"0").unwrap();
    assert_eq!(ipld, Ipld::Integer(0));
}

#[test]
fn test_numbers2() {
    let ipld: Ipld = de::from_slice(b"12345678").unwrap();
    assert_eq!(ipld, Ipld::Integer(12345678));
}

#[test]
fn test_numbers3() {
    let ipld: Ipld = de::from_slice(b"-2015").unwrap();
    assert_eq!(ipld, Ipld::Integer(-2015));
}

#[test]
fn test_numbers_large_negative() {
    // With serde_json, large negative numbers become floats.
    let ipld: Ipld = de::from_slice(b"-11959030306112471732").unwrap();
    let expected: i128 = -11959030306112471732;
    assert!(expected < i128::from(i64::MIN));
    assert_eq!(ipld, Ipld::Float(expected as f64));
}

#[test]
fn test_bool() {
    let ipld: Ipld = de::from_slice(b"false").unwrap();
    assert_eq!(ipld, Ipld::Bool(false));
}

#[test]
fn test_null() {
    let ipld: Ipld = de::from_slice(b"null").unwrap();
    assert_eq!(ipld, Ipld::Null);
}

#[test]
fn test_trailing_bytes() {
    let ipld: Result<Ipld, _> = de::from_slice(b"falsetrailing");
    assert!(matches!(ipld.unwrap_err(), DecodeError::TrailingData));
}

#[test]
fn test_list() {
    let ipld: Ipld = de::from_slice(b"[1,2,3]").unwrap();
    assert_eq!(
        ipld,
        Ipld::List(vec![Ipld::Integer(1), Ipld::Integer(2), Ipld::Integer(3)])
    );
}

#[test]
fn test_list_nested() {
    let ipld: Ipld = de::from_slice(b"[1,[2,[3]]]").unwrap();
    assert_eq!(
        ipld,
        Ipld::List(vec![
            Ipld::Integer(1),
            Ipld::List(vec![Ipld::Integer(2), Ipld::List(vec![Ipld::Integer(3)])])
        ])
    );
}

#[test]
fn test_object() {
    let ipld: Ipld = de::from_slice(br#"{"a":"A","b":"B","c":"C","d":"D","e":"E"}"#).unwrap();
    let expected = Ipld::Map(BTreeMap::from([
        ("a".to_string(), Ipld::String("A".to_string())),
        ("b".to_string(), Ipld::String("B".to_string())),
        ("c".to_string(), Ipld::String("C".to_string())),
        ("d".to_string(), Ipld::String("D".to_string())),
        ("e".to_string(), Ipld::String("E".to_string())),
    ]));
    assert_eq!(ipld, expected);
}

#[test]
fn test_empty_map() {
    let ipld: Ipld = de::from_slice(b"{}").unwrap();
    let expected = Ipld::Map(BTreeMap::new());
    assert_eq!(ipld, expected);
}

#[test]
fn test_float() {
    let ipld: Ipld = de::from_slice(b"100000.0").unwrap();
    assert_eq!(ipld, Ipld::Float(100000.0));
}

#[test]
fn test_crazy_list() {
    //let slice = b"[123456789959,-34567897654325468,-456787678,true,null,23456543.5]";
    //let slice = b"[null]";
    let slice = b"[123456789959, -34567897654325468, -456787678, true, null, 23456543.5]";
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
    assert_eq!(v, b"null", "unit is serialized as NULL.");
    let result: Result<(), _> = de::from_slice(&v);
    assert!(result.is_ok(), "unit was successfully deserialized");
}

#[test]
fn test_ipaddr_deserialization() {
    use std::net::{IpAddr, Ipv4Addr};
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let buf = to_vec(&ip).unwrap();
    let deserialized_ip = de::from_slice::<IpAddr>(&buf).unwrap();
    assert_eq!(ip, deserialized_ip);
}

#[test]
fn test_invalid_reserved_cid() {
    let data = br#"{"/": true}"#;
    let ipld: Result<Ipld, _> = de::from_slice(data);
    assert!(ipld.is_err());
}

#[test]
fn test_invalid_reserved_bytes() {
    let data = br#"{"/": {"bytes": false}}"#;
    let ipld: Result<Ipld, _> = de::from_slice(data);
    assert!(ipld.is_err());
}

/// The reserved key `"/"` is deserialized normally if it's not the first element of a map.
#[test]
fn test_reserved_later() {
    let data =
        br#"{"some": "data", "/": "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}"#;
    let ipld: Ipld = de::from_slice(data).unwrap();
    let expected = Ipld::Map(BTreeMap::from([
        ("some".to_string(), Ipld::String("data".to_string())),
        (
            "/".to_string(),
            Ipld::String("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy".to_string()),
        ),
    ]));
    assert_eq!(ipld, expected);
}

/// The reserved key `"/"` must be the only key in a map, else it's an error.
#[test]
fn test_reserved_trailing() {
    let data =
        br#"{"/": "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy", "trailing": 123}"#;
    let ipld: Result<Ipld, _> = de::from_slice(data);
    assert!(ipld.is_err());
}

#[test]
fn test_newtype_bytes() {
    #[derive(Deserialize)]
    struct Test(ByteBuf);

    let data = br#"{"/": {"bytes": "dm14"}}"#;
    let bytes: Test = de::from_slice(data).unwrap();
    let expected = ByteBuf::from([118, 109, 120]);
    assert_eq!(bytes.0, expected);
}

#[test]
fn test_newtype_cid() {
    #[derive(Deserialize)]
    struct Test(Cid);

    let data = br#"{"/": "bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}"#;
    let cid: Test = de::from_slice(data).unwrap();
    let expected =
        Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap();
    assert_eq!(cid.0, expected);
}
