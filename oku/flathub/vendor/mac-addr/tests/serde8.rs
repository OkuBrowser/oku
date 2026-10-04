#![cfg(feature = "serde")]

use mac_addr::{MacAddr, MacAddr8};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Wrap8 {
    mac8: MacAddr8,
    mac6: MacAddr,
}

#[test]
fn serde_human_readable_roundtrip_macaddr8() {
    let w = Wrap8 {
        mac8: MacAddr8::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11),
        mac6: MacAddr::new(0x00, 0x25, 0x96, 0x12, 0x34, 0x56),
    };
    let s = serde_json::to_string(&w).unwrap();
    assert!(s.contains("aa:bb:cc:dd:ee:ff:00:11"), "json={}", s);
    assert!(s.contains("00:25:96:12:34:56"), "json={}", s);

    let back: Wrap8 = serde_json::from_str(&s).unwrap();
    assert_eq!(back, w);
}

#[test]
fn serde_compact_bincode_roundtrip_macaddr8() {
    let w = Wrap8 {
        mac8: MacAddr8::new(1, 2, 3, 4, 5, 6, 7, 8),
        mac6: MacAddr::new(1, 2, 3, 4, 5, 6),
    };
    let bin = bincode::serialize(&w).unwrap();
    // Sanity check on size: struct Wrap8 has 8 bytes for MacAddr8 + 6 bytes for MacAddr + some overhead for the struct itself.
    assert!(bin.len() <= 32, "unexpected binary size: {}", bin.len());
    let back: Wrap8 = bincode::deserialize(&bin).unwrap();
    assert_eq!(back, w);
}
