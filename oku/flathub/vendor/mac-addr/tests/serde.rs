// Serde round-trip tests (only built when the `serde` feature is enabled).

#![cfg(feature = "serde")]

use mac_addr::MacAddr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Wrapper {
    mac: MacAddr,
}

#[test]
fn serde_human_readable_roundtrip() {
    let w = Wrapper {
        mac: MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff),
    };
    let s = serde_json::to_string(&w).unwrap();
    // Expect human-readable hex with colons
    assert!(s.contains("aa:bb:cc:dd:ee:ff"), "json={}", s);

    let back: Wrapper = serde_json::from_str(&s).unwrap();
    assert_eq!(back, w);
}

#[test]
fn serde_compact_bincode_roundtrip() {
    // Binary serializers should use raw 6 bytes per address.
    let w = Wrapper {
        mac: MacAddr::new(1, 2, 3, 4, 5, 6),
    };
    let bin = bincode::serialize(&w).unwrap();
    // Sanity check on size: struct Wrapper has 6 bytes for MacAddr + some overhead for the struct itself.
    assert!(bin.len() <= 16, "unexpected binary size: {}", bin.len());
    let back: Wrapper = bincode::deserialize(&bin).unwrap();
    assert_eq!(back, w);
}
