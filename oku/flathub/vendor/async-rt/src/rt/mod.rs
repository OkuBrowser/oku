#[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
pub mod threadpool;

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub mod tokio;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
pub mod compio;

pub mod dummy;

#[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
pub mod lite;

#[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
pub mod smol;
