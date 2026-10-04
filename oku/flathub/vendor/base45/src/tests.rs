use crate::*;
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec};

#[test]
fn encode_ab() {
    assert_eq!(encode("AB"), "BB8")
}

#[test]
fn decode_fail() {
    assert!(decode(":::").is_err());
}

#[test]
fn decode_fail_out_of_range() {
    assert!(decode(":::").is_err());
}

#[test]
fn encode_hello() {
    assert_eq!(encode("Hello!!"), "%69 VD92EX0")
}

#[test]
fn encode_base45() {
    assert_eq!(encode("base-45"), "UJCLQE7W581")
}

#[test]
fn encode_long_string() {
    assert_eq!(
        encode("The quick brown fox jumps over the lazy dog"),
        "8UADZCKFEOEDJOD2KC54EM-DX.CH8FSKDQ$D.OE44E5$CS44+8DK44OEC3EFGVCD2",
    )
}

#[test]
fn encode_unicode() {
    assert_eq!(encode("foo © bar 𝌆 baz"), "X.C82EIROA44GECH74C-J1/GUJCW2")
}

#[test]
fn encode_emoji() {
    assert_eq!(encode("I ❤️  Rust"), "0B98TSD%K.ENY244JA QE")
}

#[test]
fn encode_ietf() {
    assert_eq!(encode("ietf!"), "QED8WEX0")
}

#[test]
fn decode_ab() {
    assert_eq!(String::from_utf8(decode("BB8").unwrap()).unwrap(), "AB")
}

#[test]
fn decode_hello() {
    assert_eq!(
        String::from_utf8(decode("%69 VD92EX0").unwrap()).unwrap(),
        "Hello!!"
    )
}

#[test]
fn decode_base45() {
    assert_eq!(
        String::from_utf8(decode("UJCLQE7W581").unwrap()).unwrap(),
        "base-45"
    )
}

#[test]
fn decode_ietf() {
    assert_eq!(
        String::from_utf8(decode("QED8WEX0").unwrap()).unwrap(),
        "ietf!"
    )
}

const QUICK_BROWN_FOX_ENC: &str =
    "8UADZCKFEOEDJOD2KC54EM-DX.CH8FSKDQ$D.OE44E5$CS44+8DK44OEC3EFGVCD2";
const QUICK_BROWN_FOX_DEC: &str = "The quick brown fox jumps over the lazy dog";
#[test]
fn decode_long_string() {
    assert_eq!(
        String::from_utf8(decode(QUICK_BROWN_FOX_ENC).unwrap()).unwrap(),
        QUICK_BROWN_FOX_DEC,
    )
}

#[test]
fn encode_hello_from_buffer() {
    assert_eq!(encode(vec![72, 101, 108, 108, 111, 33, 33]), "%69 VD92EX0")
}

#[test]
fn encode_full_bytes() {
    let s = encode(b"\xff\xff\xff\xff\xff\xff");
    assert_eq!(s, "FGWFGWFGW");
    let s = encode(b"\xff\xff\xff\xff\xff\xff\xff");
    assert_eq!(s, "FGWFGWFGWU5");
    let s = encode(b"\xff\xff\xff\xff\xff\xff\xff\xff");
    assert_eq!(s, "FGWFGWFGWFGW");
}
#[test]
fn decode_full_bytes() {
    let s = decode("FGWFGWFGW").unwrap();
    assert_eq!(s, b"\xff\xff\xff\xff\xff\xff");
    let s = decode("FGWFGWFGWU5").unwrap();
    assert_eq!(s, b"\xff\xff\xff\xff\xff\xff\xff");
    let s = decode("FGWFGWFGWFGW").unwrap();
    assert_eq!(s, b"\xff\xff\xff\xff\xff\xff\xff\xff");
}
