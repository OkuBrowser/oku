// Core API tests for MacAddr8 (no std required).

use core::str::FromStr;
use mac_addr::{MacAddr, MacAddr8, ParseMacAddrError};

#[test]
fn construct_and_octets_8() {
    let m = MacAddr8::new(0, 1, 2, 3, 4, 5, 6, 7);
    assert_eq!(m.octets(), [0, 1, 2, 3, 4, 5, 6, 7]);

    let from_oct = MacAddr8::from_octets([10, 11, 12, 13, 14, 15, 16, 17]);
    assert_eq!(from_oct.octets(), [10, 11, 12, 13, 14, 15, 16, 17]);

    let arr: [u8; 8] = m.into();
    assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn classification_bits_8() {
    // multicast: least significant bit of first octet = 1
    let mc = MacAddr8::new(0x01, 0, 0, 0, 0, 0, 0, 0);
    assert!(mc.is_multicast());
    assert!(!mc.is_unicast());

    // locally administered: second least significant bit of first octet = 1
    let la = MacAddr8::new(0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00);
    assert!(la.is_locally_administered());
    assert!(la.is_unicast());
    assert!(!la.is_multicast());
}

#[test]
fn oui_prefix_8() {
    let m = MacAddr8::new(0x3c, 0x5a, 0xb4, 0x11, 0x22, 0x33, 0x44, 0x55);
    assert_eq!(m.oui(), [0x3c, 0x5a, 0xb4]);
}

#[test]
fn from_str_ok_lower_and_upper_8() {
    let lower = "00:11:22:33:44:55:66:77".parse::<MacAddr8>().unwrap();
    let upper = "AA:BB:CC:DD:EE:FF:00:11".parse::<MacAddr8>().unwrap();
    assert_eq!(
        lower.octets(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]
    );
    assert_eq!(
        upper.octets(),
        [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11]
    );
}

#[test]
fn from_str_errors_8() {
    // Too few components
    let e = MacAddr8::from_str("00:11:22:33").unwrap_err();
    assert_eq!(e, ParseMacAddrError::TooFewComponents);

    // Too many components
    let e = MacAddr8::from_str("00:11:22:33:44:55:66:77:88").unwrap_err();
    assert_eq!(e, ParseMacAddrError::TooManyComponents);

    // Invalid component (non-hex)
    let e = MacAddr8::from_str("00:GG:22:33:44:55:66:77").unwrap_err();
    assert_eq!(e, ParseMacAddrError::InvalidComponent);
}

#[test]
fn display_is_lower_colon_8() {
    let m = MacAddr8::new(0x0a, 0x0b, 0x0c, 0xfd, 0xfe, 0xff, 0x01, 0x02);
    let s = format!("{}", m);
    assert_eq!(s, "0a:0b:0c:fd:fe:ff:01:02");
}

#[test]
fn as_ref_bytes_8() {
    let m = MacAddr8::new(1, 2, 3, 4, 5, 6, 7, 8);
    let bytes: &[u8; 8] = m.as_ref();
    assert_eq!(bytes, &[1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn eui48_eui64_conversion() {
    // EUI-48 -> EUI-64 by inserting ff:fe
    let m6 = MacAddr::new(0x00, 0x25, 0x96, 0x12, 0x34, 0x56);
    let m8 = MacAddr8::from_eui48(m6);
    assert_eq!(m8.to_string(), "00:25:96:ff:fe:12:34:56");

    // And back only if pattern matches
    assert_eq!(m8.to_eui48().unwrap(), m6);

    // Non-embedded EUI-64 should not convert back
    let non = MacAddr8::new(0, 1, 2, 3, 4, 5, 6, 7);
    assert!(non.to_eui48().is_none());
}

#[cfg(any(feature = "std", feature = "alloc"))]
#[test]
fn address_string_when_alloc_available_8() {
    let m = MacAddr8::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03);
    assert_eq!(m.address(), "de:ad:be:ef:00:01:02:03");
}
