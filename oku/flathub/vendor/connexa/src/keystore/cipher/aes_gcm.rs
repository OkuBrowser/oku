use crate::keystore::{Cipher, Error, Result};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::Rng;
use zeroize::Zeroizing;

const NONCE_LEN: usize = 12;

/// AES-256-GCM [`Cipher`] backend.
pub struct AesGcmCipher {
    cipher: Aes256Gcm,
}

impl AesGcmCipher {
    pub fn new(key: [u8; 32]) -> Self {
        let key = Zeroizing::new(key);
        let cipher = Aes256Gcm::new_from_slice(key.as_slice()).expect("32-byte key");
        Self { cipher }
    }
}

#[allow(deprecated)]
impl Cipher for AesGcmCipher {
    fn encrypt(&self, aad: Option<&[u8]>, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce);
        let ciphertext = self
            .cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: aad.unwrap_or(&[]),
                },
            )
            .map_err(|_| Error::EncryptFailed)?;
        let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ciphertext);
        Ok(blob)
    }

    fn decrypt(&self, aad: Option<&[u8]>, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < NONCE_LEN {
            return Err(Error::DecryptFailed);
        }
        let (nonce, ct) = ciphertext.split_at(NONCE_LEN);
        self.cipher
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ct,
                    aad: aad.unwrap_or(&[]),
                },
            )
            .map_err(|_| Error::DecryptFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> AesGcmCipher {
        AesGcmCipher::new([7u8; 32])
    }

    #[test]
    fn round_trip_with_aad() {
        let c = cipher();
        let blob = c.encrypt(Some(b"label"), b"secret").unwrap();
        assert_eq!(c.decrypt(Some(b"label"), &blob).unwrap(), b"secret");
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let c = cipher();
        let mut blob = c.encrypt(Some(b"label"), b"secret").unwrap();
        *blob.last_mut().unwrap() ^= 0xff;
        assert!(matches!(
            c.decrypt(Some(b"label"), &blob),
            Err(Error::DecryptFailed)
        ));
    }

    #[test]
    fn truncated_blob_below_nonce_len_is_rejected() {
        let c = cipher();
        assert!(matches!(
            c.decrypt(Some(b"label"), &[0u8; NONCE_LEN - 1]),
            Err(Error::DecryptFailed)
        ));
    }

    #[test]
    fn mismatched_aad_fails() {
        let c = cipher();
        let blob = c.encrypt(Some(b"label-a"), b"secret").unwrap();
        assert!(matches!(
            c.decrypt(Some(b"label-b"), &blob),
            Err(Error::DecryptFailed)
        ));
    }
}
