use serde_derive::{Deserialize, Serialize};

use serde_ipld_dagcbor::{from_slice, to_vec};

fn to_binary(s: &'static str) -> Vec<u8> {
    assert!(s.len() % 2 == 0);
    let mut b = Vec::with_capacity(s.len() / 2);
    for i in 0..s.len() / 2 {
        b.push(u8::from_str_radix(&s[i * 2..(i + 1) * 2], 16).unwrap());
    }
    b
}

macro_rules! testcase {
    ($name:ident, f64, $expr:expr, $s:expr) => {
        #[test]
        fn $name() {
            let expr: f64 = $expr;
            let serialized = to_binary($s);
            assert_eq!(to_vec(&expr).unwrap(), serialized);
            let parsed: f64 = from_slice(&serialized[..]).unwrap();
            if !expr.is_nan() {
                assert_eq!(expr, parsed);
            } else {
                assert!(parsed.is_nan())
            }

            #[cfg(feature = "std")]
            {
                let parsed: f64 = serde_ipld_dagcbor::from_reader(&mut &serialized[..]).unwrap();
                if !expr.is_nan() {
                    assert_eq!(expr, parsed);
                } else {
                    assert!(parsed.is_nan())
                }
            }
        }
    };
    ($name:ident, $ty:ty, $expr:expr, $s:expr) => {
        #[test]
        fn $name() {
            let expr: $ty = $expr;
            let serialized = to_binary($s);
            assert_eq!(
                to_vec(&expr).expect("ser1 works"),
                serialized,
                "serialization differs"
            );
            let parsed: $ty = from_slice(&serialized[..]).expect("de1 works");
            assert_eq!(parsed, expr, "parsed result differs");
        }
    };
}

testcase!(test_bool_false, bool, false, "f4");
testcase!(test_bool_true, bool, true, "f5");
testcase!(test_isize_neg_256, isize, -256, "38ff");
testcase!(test_isize_neg_257, isize, -257, "390100");
testcase!(test_isize_255, isize, 255, "18ff");
testcase!(test_i8_5, i8, 5, "05");
testcase!(test_i8_23, i8, 23, "17");
testcase!(test_i8_24, i8, 24, "1818");
testcase!(test_i8_neg_128, i8, -128, "387f");
testcase!(test_u32_98745874, u32, 98745874, "1a05e2be12");
// In DAG-CBOR you cannot deserialize into f32, it's always f64.
//testcase!(test_f32_1234_point_5, f32, 1234.5, "fb40934a0000000000");
testcase!(test_f64_12345_point_6, f64, 12345.6, "fb40c81ccccccccccd");
testcase!(test_char_null, char, '\x00', "6100");
testcase!(test_char_broken_heart, char, '💔', "64f09f9294");
testcase!(
    test_str_pangram_de,
    String,
    "aâø↓é".to_owned(),
    "6a61c3a2c3b8e28693c3a9"
);

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct NewtypeStruct(bool);
testcase!(
    test_newtype_struct,
    NewtypeStruct,
    NewtypeStruct(true),
    "f5"
);

testcase!(test_option_none, Option<u8>, None, "f6");
testcase!(test_option_some, Option<u8>, Some(42), "182a");

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Person {
    name: String,
    year_of_birth: u16,
    profession: Option<String>,
}

testcase!(test_person_struct,
    Person,
    Person {
        name: "Grace Hopper".to_string(),
        year_of_birth: 1906,
        profession: Some("computer scientist".to_string()),
    },
    "a3646e616d656c477261636520486f707065726a70726f66657373696f6e72636f6d707574657220736369656e746973746d796561725f6f665f6269727468190772");

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct OptionalPerson {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    year_of_birth: Option<u16>,
    profession: Option<String>,
}

testcase!(test_optional_person_struct,
    OptionalPerson,
    OptionalPerson {
        name: "Grace Hopper".to_string(),
        year_of_birth: None,
        profession: Some("computer scientist".to_string()),
    },
    "a2646e616d656c477261636520486f707065726a70726f66657373696f6e72636f6d707574657220736369656e74697374");

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum Color {
    Red,
    Blue,
    Yellow,
    Other(u64),
    Alpha(u64, u8),
}

testcase!(test_color_enum, Color, Color::Blue, "64426c7565");
testcase!(
    test_color_enum_transparent,
    Color,
    Color::Other(42),
    "a1654f74686572182a"
);
testcase!(
    test_color_enum_with_alpha,
    Color,
    Color::Alpha(234567, 60),
    "a165416c706861821a00039447183c"
);
testcase!(test_i128_a, i128, -1i128, "20");
testcase!(test_u128, u128, 17, "11");

// f32 round-trip tests
#[test]
fn test_f32_roundtrip() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct SimpleF32 {
        value: f32,
    }

    // Test various f32 values
    let test_values = vec![
        0.0f32,
        -0.0f32,
        1.0f32,
        -1.0f32,
        1.5f32,
        f32::MIN,
        f32::MAX,
        f32::EPSILON,
        f32::MIN_POSITIVE,
    ];

    for value in test_values {
        let original = SimpleF32 { value };
        let encoded = to_vec(&original).expect("encoding should succeed");
        let decoded: SimpleF32 = from_slice(&encoded).expect("decoding should succeed");
        assert_eq!(original.value.to_bits(), decoded.value.to_bits());
    }
}

#[test]
fn test_f32_encoding_is_f64() {
    // Verify that f32 values are actually encoded as f64 in DAG-CBOR
    let value = 1.5f32;
    let encoded = to_vec(&value).expect("encoding should succeed");

    // CBOR marker for f64 is 0xfb
    assert_eq!(encoded[0], 0xfb);
    // The encoded value should be 8 bytes (f64) + 1 byte marker = 9 bytes total
    assert_eq!(encoded.len(), 9);
}

#[test]
fn test_accept_f32_cbor_marker_for_compatibility() {
    // Test that we accept CBOR f32 encoding (0xfa marker) for compatibility,
    // even though it's not valid in strict DAG-CBOR (please don't take this
    // test as permission to use f32 encoding in new data!).

    // Manually construct CBOR with f32 encoding
    // 0xfa = f32 marker, followed by 4 bytes of IEEE 754 single precision
    let f32_cbor = vec![
        0xfa, // f32 marker (not valid in strict DAG-CBOR, but we accept it)
        0x3f, 0xc0, 0x00, 0x00, // 1.5 in IEEE 754 single precision
    ];

    // Should successfully decode for compatibility
    let result: Result<f32, _> = from_slice(&f32_cbor);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1.5f32);
}

#[test]
fn test_f32_strict_precision_rejection() {
    // Test that f64 values with more precision than f32 can represent are rejected
    // when deserializing to f32

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct F32Wrapper {
        value: f32,
    }

    // Value that loses precision when converted to f32
    let precise_f64 = 0.1f64;
    let encoded = to_vec(&precise_f64).expect("encoding should succeed");

    // Try to decode as f32 - should fail due to precision loss
    let result: Result<f32, _> = from_slice(&encoded);
    assert!(result.is_err());
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(
            error_msg.contains("precision"),
            "Expected precision error, got: {}",
            error_msg
        );
    }

    // Value that is exactly representable in f32 should work
    let exact_f64 = 1.5f64;
    let encoded = to_vec(&exact_f64).expect("encoding should succeed");
    let result: Result<f32, _> = from_slice(&encoded);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1.5f32);

    // Test with struct containing f32
    let original = F32Wrapper { value: 1.5f32 };
    let encoded = to_vec(&original).expect("encoding should succeed");
    let decoded: Result<F32Wrapper, _> = from_slice(&encoded);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap(), original);

    // Negative zero should preserve sign bit and work
    let neg_zero_f64 = -0.0f64;
    let encoded = to_vec(&neg_zero_f64).expect("encoding should succeed");
    let result: Result<f32, _> = from_slice(&encoded);
    assert!(result.is_ok());
    let decoded_f32 = result.unwrap();
    assert_eq!(decoded_f32, -0.0f32);
    // Check that sign bit is preserved
    assert_eq!(decoded_f32.to_bits(), (-0.0f32).to_bits());
}
