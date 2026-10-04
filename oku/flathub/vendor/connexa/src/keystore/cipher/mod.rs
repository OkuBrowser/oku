#[cfg(feature = "aes-gcm")]
pub mod aes_gcm;

#[cfg(feature = "plaintext")]
pub mod plaintext;

// TODO: determine if we should put behind a feature flag as well?
pub mod xchacha20poly1305;
