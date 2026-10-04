//! Random numbers generation utilities required in WebSocket clients.

#[cfg(not(any(
    feature = "fastrand",
    feature = "getrandom",
    feature = "nightly",
    feature = "rand"
)))]
compile_error!(
    "Using the `client` feature requires enabling a random number generator implementation via one of the following features: `fastrand`, `getrandom`, `nightly` or `rand`."
);

/// Random numbers generation utilities using [`std::random`].
#[cfg(all(
    feature = "nightly",
    not(feature = "fastrand"),
    not(feature = "getrandom"),
    not(feature = "rand")
))]
mod imp {
    use std::random::RandomSource;

    /// Generate a random 16-byte WebSocket key.
    pub fn get_key() -> [u8; 16] {
        let mut bytes = [0; 16];
        std::random::DefaultRandomSource.fill_bytes(&mut bytes);
        bytes
    }

    /// Generate a random 4-byte WebSocket mask.
    pub fn get_mask(dst: &mut [u8; 4]) {
        std::random::DefaultRandomSource.fill_bytes(dst);
    }
}

/// Random numbers generation utilities using [`fastrand`].
#[cfg(all(
    feature = "fastrand",
    not(feature = "getrandom"),
    not(feature = "rand")
))]
mod imp {
    /// Generate a random 16-byte WebSocket key.
    pub fn get_key() -> [u8; 16] {
        fastrand::u128(..).to_ne_bytes()
    }

    /// Generate a random 4-byte WebSocket mask.
    pub fn get_mask(dst: &mut [u8; 4]) {
        fastrand::fill(dst);
    }
}

/// Random numbers generation utilities using [`getrandom`].
#[cfg(all(feature = "getrandom", not(feature = "rand")))]
mod imp {

    /// Generate a random 16-byte WebSocket key.
    pub fn get_key() -> [u8; 16] {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).expect("Failed to get random bytes, consider using `rand` or `fastrand` instead of `getrandom` if this persists");
        bytes
    }

    /// Generate a random 4-byte WebSocket mask.
    pub fn get_mask(dst: &mut [u8; 4]) {
        getrandom::fill(dst).expect("Failed to get random bytes, consider using `rand` or `fastrand` instead of `getrandom` if this persists");
    }
}

/// Random numbers generation utilities using [`rand`].
#[cfg(feature = "rand")]
mod imp {
    /// Generate a random 16-byte WebSocket key.
    pub fn get_key() -> [u8; 16] {
        let mut bytes = [0; 16];
        rand::fill(&mut bytes);
        bytes
    }

    /// Generate a random 4-byte WebSocket mask.
    pub fn get_mask(dst: &mut [u8; 4]) {
        rand::fill(dst);
    }
}

pub use imp::{get_key, get_mask};
