//! Helper functions for immutable items.

use sha1_smol::Sha1;

use super::ID_SIZE;
use crate::Id;

pub fn validate_immutable(v: &[u8], target: Id) -> bool {
    hash_immutable(v) == *target.as_bytes()
}

pub fn hash_immutable(v: &[u8]) -> [u8; ID_SIZE] {
    let mut encoded = Vec::with_capacity(v.len() + 3);
    encoded.extend(format!("{}:", v.len()).bytes());
    encoded.extend_from_slice(v);

    let mut hasher = Sha1::new();
    hasher.update(&encoded);

    hasher.digest().bytes()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_validate_immutable() {
        let v = vec![
            171, 118, 111, 111, 174, 109, 195, 32, 138, 140, 113, 176, 76, 135, 116, 132, 156, 126,
            75, 173,
        ];

        let target = Id::from_bytes([
            2, 23, 113, 43, 67, 11, 185, 26, 26, 30, 204, 238, 204, 1, 13, 84, 52, 40, 86, 231,
        ])
        .unwrap();

        assert!(validate_immutable(&v, target));
        assert!(!validate_immutable(&v[1..], target));
    }

    #[test]

    fn test_hash_immutable() {
        let v = b"From the river to the sea, Palestine will be free";
        let target = Id::from_str("4238af8aff56cf6e0007d9d2003bf23d33eea7c3").unwrap();

        assert_eq!(hash_immutable(v), *target.as_bytes());
    }
}
