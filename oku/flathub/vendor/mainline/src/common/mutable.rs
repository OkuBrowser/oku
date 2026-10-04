//! Helper functions and structs for mutable items.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha1_smol::Sha1;
use std::convert::TryFrom;

use crate::Id;

use super::PutMutableRequestArguments;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// [BEP_0044](https://www.bittorrent.org/beps/bep_0044.html)'s Mutable item.
pub struct MutableItem {
    /// hash of the key and optional salt
    target: Id,
    /// ed25519 public key
    key: [u8; 32],
    /// sequence number
    pub(crate) seq: i64,
    /// mutable value
    pub(crate) value: Box<[u8]>,
    /// ed25519 signature
    #[serde(with = "serde_bytes")]
    signature: [u8; 64],
    /// Optional salt
    salt: Option<Box<[u8]>>,
}

impl MutableItem {
    /// Create a new mutable item from a signing key, value, sequence number and optional salt.
    pub fn new(signer: SigningKey, value: &[u8], seq: i64, salt: Option<&[u8]>) -> Self {
        let signable = encode_signable(seq, value, salt);
        let signature = signer.sign(&signable);

        Self::new_signed_unchecked(
            signer.verifying_key().to_bytes(),
            signature.into(),
            value,
            seq,
            salt,
        )
    }

    /// Return the target of a [MutableItem] by hashing its `public_key` and an optional `salt`
    pub fn target_from_key(public_key: &[u8; 32], salt: Option<&[u8]>) -> Id {
        let mut encoded = vec![];

        encoded.extend(public_key);

        if let Some(salt) = salt {
            encoded.extend(salt);
        }

        let mut hasher = Sha1::new();
        hasher.update(&encoded);
        let bytes = hasher.digest().bytes();

        bytes.into()
    }

    /// Create a new mutable item from an already signed value.
    pub fn new_signed_unchecked(
        key: [u8; 32],
        signature: [u8; 64],
        value: &[u8],
        seq: i64,
        salt: Option<&[u8]>,
    ) -> Self {
        Self {
            target: MutableItem::target_from_key(&key, salt),
            key,
            value: value.into(),
            seq,
            signature,
            salt: salt.map(|s| s.into()),
        }
    }

    pub(crate) fn from_dht_message(
        target: Id,
        key: &[u8],
        v: Box<[u8]>,
        seq: i64,
        signature: &[u8],
        salt: Option<Box<[u8]>>,
    ) -> Result<Self, MutableError> {
        let key = VerifyingKey::try_from(key).map_err(|_| MutableError::InvalidMutablePublicKey)?;

        let signature =
            Signature::from_slice(signature).map_err(|_| MutableError::InvalidMutableSignature)?;

        key.verify(&encode_signable(seq, &v, salt.as_deref()), &signature)
            .map_err(|_| MutableError::InvalidMutableSignature)?;

        if Self::target_from_key(&key.to_bytes(), salt.as_deref()) != target {
            return Err(MutableError::InvalidMutableTarget);
        }

        Ok(Self {
            target,
            key: key.to_bytes(),
            value: v,
            seq,
            signature: signature.to_bytes(),
            salt,
        })
    }

    // === Getters ===

    /// Returns the target (info hash) of this item.
    pub fn target(&self) -> &Id {
        &self.target
    }

    /// Returns a reference to the 32 bytes Ed25519 public key of this item.
    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    /// Returns a byte slice of the value of this item.
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    /// Returns the `seq` (sequence) number of this item.
    pub fn seq(&self) -> i64 {
        self.seq
    }

    /// Returns the signature over this item.
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }

    /// Returns the `Salt` value used for generating the
    /// [Self::target] if any.
    pub fn salt(&self) -> Option<&[u8]> {
        self.salt.as_deref()
    }
}

pub fn encode_signable(seq: i64, value: &[u8], salt: Option<&[u8]>) -> Box<[u8]> {
    let mut signable = vec![];

    if let Some(salt) = salt {
        signable.extend(format!("4:salt{}:", salt.len()).into_bytes());
        signable.extend(salt);
    }

    signable.extend(format!("3:seqi{}e1:v{}:", seq, value.len()).into_bytes());
    signable.extend(value);

    signable.into()
}

pub(crate) fn most_recent_mutable_item(
    most_recent: Option<MutableItem>,
    item: MutableItem,
) -> Option<MutableItem> {
    match most_recent {
        Some(mr)
            if mr.seq() > item.seq() || (mr.seq() == item.seq() && mr.value() >= item.value()) =>
        {
            Some(mr)
        }
        _ => Some(item),
    }
}

#[derive(thiserror::Error, Debug)]
/// Mainline crate error enum.
pub enum MutableError {
    #[error("Invalid mutable item signature")]
    /// Invalid mutable item signature
    InvalidMutableSignature,

    #[error("Invalid mutable item public key")]
    /// Invalid mutable item public key
    InvalidMutablePublicKey,

    #[error("Mutable item target does not match its public key and salt")]
    /// Mutable item target does not match its public key and salt
    InvalidMutableTarget,
}

impl PutMutableRequestArguments {
    /// Create a [PutMutableRequestArguments] from a [MutableItem],
    /// and an optional CAS condition, which is usually the [MutableItem::seq]
    /// of the most recent known [MutableItem]
    pub fn from(item: MutableItem, cas: Option<i64>) -> Self {
        Self {
            target: item.target,
            v: item.value,
            k: item.key,
            seq: item.seq,
            sig: item.signature,
            salt: item.salt,
            cas,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ed25519_dalek::SigningKey;

    #[test]
    fn signable_without_salt() {
        let signable = encode_signable(4, b"Hello world!", None);

        assert_eq!(&*signable, b"3:seqi4e1:v12:Hello world!");
    }
    #[test]
    fn signable_with_salt() {
        let signable = encode_signable(4, b"Hello world!", Some(b"foobar"));

        assert_eq!(&*signable, b"4:salt6:foobar3:seqi4e1:v12:Hello world!");
    }

    #[test]
    fn most_recent_mutable_item_selects_by_seq_then_value() {
        let signer = SigningKey::from_bytes(&[
            56, 171, 62, 85, 105, 58, 155, 209, 189, 8, 59, 109, 137, 84, 84, 201, 221, 115, 7,
            228, 127, 70, 4, 204, 182, 64, 77, 98, 92, 215, 27, 103,
        ]);

        let lower_seq = MutableItem::new(signer.clone(), b"lower-seq", 999, None);
        let current = MutableItem::new(signer.clone(), b"current1", 1000, None);
        let higher_seq = MutableItem::new(signer.clone(), b"higher-seq", 1001, None);
        let same_seq_lower_value = MutableItem::new(signer.clone(), b"current0", 1000, None);
        let same_seq_higher_value = MutableItem::new(signer, b"current2", 1000, None);

        assert_eq!(
            most_recent_mutable_item(None, current.clone()),
            Some(current.clone())
        );

        assert_eq!(
            most_recent_mutable_item(Some(current.clone()), higher_seq.clone()),
            Some(higher_seq)
        );

        assert_eq!(
            most_recent_mutable_item(Some(current.clone()), lower_seq),
            Some(current.clone())
        );

        assert_eq!(
            most_recent_mutable_item(Some(current.clone()), same_seq_higher_value.clone()),
            Some(same_seq_higher_value)
        );

        assert_eq!(
            most_recent_mutable_item(Some(current.clone()), same_seq_lower_value),
            Some(current.clone())
        );

        assert_eq!(
            most_recent_mutable_item(Some(current.clone()), current.clone()),
            Some(current)
        );
    }

    #[test]
    fn from_dht_message_rejects_a_signed_item_for_a_different_target() {
        let signer = SigningKey::from_bytes(&[42; 32]);
        let item = MutableItem::new(signer, b"value", 1, Some(b"salt"));
        let mut wrong_target = *item.target().as_bytes();
        wrong_target[0] ^= 1;

        let result = MutableItem::from_dht_message(
            wrong_target.into(),
            item.key(),
            item.value().into(),
            item.seq(),
            item.signature(),
            item.salt().map(Into::into),
        );

        assert!(matches!(result, Err(MutableError::InvalidMutableTarget)));
    }
}
