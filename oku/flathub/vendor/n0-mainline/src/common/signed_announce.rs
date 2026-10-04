//! Helper functions and structs for announcing signed peers.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, time::SystemTime};

use crate::Id;

const MAX_TIMESTAMP_TOLERANCE: u64 = 45 * 1000 * 1000; // 45 seconds in micro seconds

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// [BEP_????](https://github.com/nuhvi/mainline/beps/bep_xxxx.html)'s `announce_signed_peer`.
pub struct SignedAnnounce {
    /// ed25519 public key
    pub(crate) key: [u8; 32],
    /// timestamp of the signed announcement
    pub(crate) timestamp: u64,
    /// ed25519 signature
    #[serde(with = "serde_bytes")]
    pub(crate) signature: [u8; 64],
}

#[allow(dead_code)]
impl SignedAnnounce {
    /// Create a new SignedAnnounce for a info_hash.
    pub fn new(signer: &SigningKey, info_hash: &Id) -> Self {
        let timestamp = system_time();

        Self::new_with_timestamp(signer, info_hash, timestamp)
    }

    pub(crate) fn new_with_timestamp(signer: &SigningKey, info_hash: &Id, timestamp: u64) -> Self {
        let signable = encode_signable(info_hash, timestamp);
        let signature = signer.sign(&signable);

        Self {
            key: signer.verifying_key().as_bytes().to_owned(),
            timestamp,
            signature: signature.to_bytes(),
        }
    }

    pub(crate) fn from_dht_request(
        info_hash: &Id,
        key: &[u8],
        timestamp: u64,
        signature: &[u8],
    ) -> Result<Self, SignedAnnounceError> {
        Self::from_dht_message(info_hash, key, timestamp, signature, true)
    }

    pub(crate) fn from_dht_response(
        info_hash: &Id,
        key: &[u8],
        timestamp: u64,
        signature: &[u8],
    ) -> Result<Self, SignedAnnounceError> {
        Self::from_dht_message(info_hash, key, timestamp, signature, false)
    }

    fn from_dht_message(
        info_hash: &Id,
        key: &[u8],
        timestamp: u64,
        signature: &[u8],
        validate_timestamp: bool,
    ) -> Result<Self, SignedAnnounceError> {
        let key = VerifyingKey::try_from(key).map_err(|_| SignedAnnounceError::PublicKey)?;

        let signature =
            Signature::from_slice(signature).map_err(|_| SignedAnnounceError::Signature)?;

        key.verify(&encode_signable(info_hash, timestamp), &signature)
            .map_err(|_| SignedAnnounceError::Signature)?;

        let now = system_time();

        if validate_timestamp && now.abs_diff(timestamp) > MAX_TIMESTAMP_TOLERANCE {
            return Err(SignedAnnounceError::Timestamp);
        }

        Ok(Self {
            key: key.to_bytes(),
            timestamp,
            signature: signature.to_bytes(),
        })
    }

    // === Getters ===

    /// Returns a reference to the 32 bytes Ed25519 public key of this item.
    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    /// Returns the `timestamp` of this announcement.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the signature over this announcement's `info_hash` (infohash) and timestamp.
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
}

fn system_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time drift")
        .as_micros() as u64
}

pub fn encode_signable(info_hash: &Id, timestamp: u64) -> Box<[u8]> {
    let mut signable = vec![];

    signable.extend(info_hash.as_bytes());
    signable.extend(timestamp.to_be_bytes());

    signable.into()
}

#[n0_error::stack_error(derive, std_sources)]
/// Mainline crate error enum.
pub enum SignedAnnounceError {
    #[error("Invalid signed announce signature")]
    /// Invalid signed announce signature
    Signature,

    #[error("Invalid signed announce public key")]
    /// Invalid signed announce public key
    PublicKey,

    #[error("Invalid signed announce timestamp (too far in the future or the past)")]
    /// Invalid signed announce timestamp
    Timestamp,
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[test]
    fn more_than_time_tolerance() {
        let mut secret_key = [0; 32];
        rand::rng().fill_bytes(&mut secret_key);
        let signer = SigningKey::from_bytes(&secret_key);

        let info_hash = Id::random();

        let now = system_time();
        let announce =
            SignedAnnounce::new_with_timestamp(&signer, &info_hash, now + 50 * 1000 * 1000);

        let result = SignedAnnounce::from_dht_request(
            &info_hash,
            announce.key(),
            announce.timestamp,
            &announce.signature,
        );

        assert!(matches!(result, Err(SignedAnnounceError::Timestamp)));

        let now = system_time();
        let announce =
            SignedAnnounce::new_with_timestamp(&signer, &info_hash, now - 50 * 1000 * 1000);

        let result = SignedAnnounce::from_dht_request(
            &info_hash,
            announce.key(),
            announce.timestamp,
            &announce.signature,
        );

        assert!(matches!(result, Err(SignedAnnounceError::Timestamp)));
    }

    #[test]
    fn invalid_signature() {
        let mut secret_key = [0; 32];
        rand::rng().fill_bytes(&mut secret_key);
        let signer = SigningKey::from_bytes(&secret_key);

        let info_hash = Id::random();

        let announce = SignedAnnounce::new(&signer, &info_hash);

        SignedAnnounce::from_dht_request(
            &info_hash,
            announce.key(),
            announce.timestamp,
            &announce.signature,
        )
        .unwrap();

        let result = SignedAnnounce::from_dht_request(
            &info_hash,
            announce.key(),
            announce.timestamp,
            &[0; 64],
        );

        assert!(matches!(result, Err(SignedAnnounceError::Signature)));

        let result = SignedAnnounce::from_dht_request(
            &info_hash,
            &[0; 30],
            announce.timestamp,
            &announce.signature,
        );

        assert!(matches!(result, Err(SignedAnnounceError::PublicKey)));
    }
}
