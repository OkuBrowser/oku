use std::{collections::BTreeMap, str::FromStr};

use ipld_core::cid::Cid;
use serde::Serialize;
use serde_bytes::{ByteBuf, Bytes};
use serde_ipld_dagjson::to_vec;

#[test]
fn test_string() {
    let value = "foobar".to_string();
    let json = to_vec(&value).unwrap();
    assert_eq!(json, br#""foobar""#);
}

#[test]
fn test_list() {
    let value = vec![1, 2, 3];
    let json = to_vec(&value).unwrap();
    assert_eq!(json, b"[1,2,3]");
}

#[test]
fn test_object() {
    let mut object = BTreeMap::new();
    object.insert("a".to_string(), "A".to_string());
    object.insert("b".to_string(), "B".to_string());
    object.insert("c".to_string(), "C".to_string());
    object.insert("d".to_string(), "D".to_string());
    object.insert("e".to_string(), "E".to_string());
    let json = to_vec(&object).unwrap();
    assert_eq!(json, br#"{"a":"A","b":"B","c":"C","d":"D","e":"E"}"#);
}

#[test]
fn test_float() {
    let json = to_vec(&12.3f64).unwrap();
    assert_eq!(json, b"12.3");
}

#[test]
fn test_f32() {
    let json = to_vec(&4000.5f32).unwrap();
    assert_eq!(json, b"4000.5");
}

#[test]
fn test_infinity() {
    let json = to_vec(&f64::INFINITY);
    assert!(json.is_err(), "Only finite numbers are supported.");
}

#[test]
fn test_neg_infinity() {
    let json = to_vec(&f64::NEG_INFINITY);
    assert!(json.is_err(), "Only finite numbers are supported.");
}

#[test]
fn test_nan() {
    let json = to_vec(&f32::NAN);
    assert!(json.is_err(), "Only finite numbers are supported.");
}

#[test]
fn test_integer() {
    // u8
    {
        let json = to_vec(&24).unwrap();
        assert_eq!(json, b"24");
    }
    // i8
    {
        let json = to_vec(&-5).unwrap();
        assert_eq!(json, b"-5");
    }
    // i16
    {
        let json = to_vec(&-300).unwrap();
        assert_eq!(json, b"-300");
    }
    // i32
    {
        let json = to_vec(&-23567997).unwrap();
        assert_eq!(json, b"-23567997");
    }
    // u64
    {
        let json = to_vec(&u64::MAX).unwrap();
        assert_eq!(json, b"18446744073709551615");
    }
    // u128 within u64 range
    {
        let json = to_vec(&(u64::MAX as u128)).unwrap();
        assert_eq!(json, b"18446744073709551615");
    }
    // u128 out of u64 range
    {
        let json = to_vec(&(u64::MAX as u128 + 1)).unwrap();
        assert_eq!(json, b"18446744073709551616");
    }
    // i128 within u64 range
    {
        let json = to_vec(&(u64::MAX as i128)).unwrap();
        assert_eq!(json, b"18446744073709551615");
    }
    // i128 within -u64 range
    {
        let json = to_vec(&(-(u64::MAX as i128))).unwrap();
        assert_eq!(json, b"-18446744073709551615");
    }
    // max 64-bit value with a negative sign
    {
        let json = to_vec(&(-(u64::MAX as i128 + 1))).unwrap();
        assert_eq!(json, b"-18446744073709551616");
    }
    // i128 out of -u64 range
    {
        let json = to_vec(&i128::MIN).unwrap();
        assert_eq!(json, b"-170141183460469231731687303715884105728");
    }
}

#[test]
fn test_ip_addr() {
    use std::net::Ipv4Addr;

    let addr = Ipv4Addr::new(8, 8, 8, 8);
    let json = to_vec(&addr).unwrap();
    assert_eq!(json, br#""8.8.8.8""#);
}

#[test]
fn test_cid() {
    let cid = Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap();
    let json = to_vec(&cid).unwrap();
    assert_eq!(
        json,
        br#"{"/":"bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}"#
    );
}

#[test]
fn test_nested_cid() {
    #[derive(Serialize)]
    struct Nested {
        some: Cid,
    }

    let cid = Cid::from_str("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap();
    let nested = Nested { some: cid };
    let json = to_vec(&nested).unwrap();
    assert_eq!(
        json,
        br#"{"some":{"/":"bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy"}}"#
    );
}

#[test]
fn test_bytes() {
    let bytes = Bytes::new(b"vmx");
    let json = to_vec(&bytes).unwrap();
    assert_eq!(json, br#"{"/":{"bytes":"dm14"}}"#);
}

#[test]
fn test_nested_bytes() {
    #[derive(Serialize)]
    struct Nested {
        some: ByteBuf,
    }

    let bytes = ByteBuf::from(b"vmx");
    let nested = Nested { some: bytes };
    let json = to_vec(&nested).unwrap();
    assert_eq!(json, br#"{"some":{"/":{"bytes":"dm14"}}}"#);
}

#[test]
fn test_ordering_dagjson() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct AB {
        a: u32,
        b: u32,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct BA {
        b: u32,
        a: u32,
    }

    let ab_bytes = serde_ipld_dagjson::ser::to_vec(&AB { a: 1, b: 2 }).unwrap();
    let ab_str = std::str::from_utf8(&ab_bytes);
    let ba_bytes = serde_ipld_dagjson::ser::to_vec(&BA { b: 2, a: 1 }).unwrap();
    let ba_str = std::str::from_utf8(&ba_bytes);

    assert_eq!(ab_str, ba_str);
}
