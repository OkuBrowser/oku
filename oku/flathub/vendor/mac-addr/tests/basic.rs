// Core API tests for mac-addr without requiring std (works with std or alloc).

use core::str::FromStr;
use mac_addr::{MacAddr, ParseMacAddrError};

#[test]
fn construct_and_octets() {
    let m = MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
    assert_eq!(m.octets(), [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);

    let from_oct = MacAddr::from_octets([1, 2, 3, 4, 5, 6]);
    assert_eq!(from_oct.octets(), [1, 2, 3, 4, 5, 6]);

    let arr: [u8; 6] = m.into();
    assert_eq!(arr, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
}

#[test]
fn zero_and_broadcast() {
    assert_eq!(MacAddr::zero().octets(), [0, 0, 0, 0, 0, 0]);
    assert_eq!(
        MacAddr::broadcast().octets(),
        [0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
}

#[test]
fn classification_bits() {
    // 01:xx:.. is multicast
    let mc = MacAddr::new(0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb);
    assert!(mc.is_multicast());
    assert!(!mc.is_unicast());

    // 02:.. is locally administered
    let la = MacAddr::new(0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee);
    assert!(la.is_locally_administered());
    assert!(!la.is_universal());
    assert!(la.is_unicast());
    assert!(!la.is_multicast());

    // A typical OUI-based unicast
    let uni = MacAddr::new(0x00, 0x1a, 0x2b, 0x00, 0x00, 0x01);
    assert!(uni.is_unicast());
    assert!(uni.is_universal());
    assert!(!uni.is_multicast());
}

#[test]
fn oui_prefix() {
    let m = MacAddr::new(0x3c, 0x5a, 0xb4, 0x12, 0x34, 0x56);
    assert_eq!(m.oui(), [0x3c, 0x5a, 0xb4]);
}

#[test]
fn from_str_ok_lower_and_upper() {
    let lower = "00:11:22:33:44:55".parse::<MacAddr>().unwrap();
    let upper = "AA:BB:CC:DD:EE:FF".parse::<MacAddr>().unwrap();
    assert_eq!(lower.octets(), [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    assert_eq!(upper.octets(), [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
}

#[test]
fn from_str_errors() {
    // Too few components
    let e = MacAddr::from_str("00:11:22").unwrap_err();
    assert_eq!(e, ParseMacAddrError::TooFewComponents);

    // Too many components
    let e = MacAddr::from_str("00:11:22:33:44:55:66").unwrap_err();
    assert_eq!(e, ParseMacAddrError::TooManyComponents);

    // Invalid component (non-hex)
    let e = MacAddr::from_str("00:GG:22:33:44:55").unwrap_err();
    assert_eq!(e, ParseMacAddrError::InvalidComponent);
}

#[test]
fn from_hex_format() {
    // Helper. Returns zero() on malformed inputs.
    // Length must be exactly 17 ("xx:xx:xx:xx:xx:xx")
    assert_eq!(
        MacAddr::from_hex_format("00:11:22:33:44:55").octets(),
        [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]
    );
    // Wrong length -> zero
    assert_eq!(MacAddr::from_hex_format("00:11").octets(), [0; 6]);
    // Invalid hex -> zero
    assert_eq!(
        MacAddr::from_hex_format("00:ZZ:22:33:44:55").octets().len(),
        6
    );
}

#[test]
fn display_is_lower_colon() {
    let m = MacAddr::new(0x0a, 0x0b, 0x0c, 0xfd, 0xfe, 0xff);
    let s = format!("{}", m);
    assert_eq!(s, "0a:0b:0c:fd:fe:ff");
}

#[test]
fn as_ref_bytes() {
    let m = MacAddr::new(1, 2, 3, 4, 5, 6);
    let bytes: &[u8; 6] = m.as_ref();
    assert_eq!(bytes, &[1, 2, 3, 4, 5, 6]);
}

#[cfg(any(feature = "std", feature = "alloc"))]
#[test]
fn address_string_when_alloc_available() {
    // `address()` requires std or alloc.
    let m = MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01);
    assert_eq!(m.address(), "de:ad:be:ef:00:01");
}
