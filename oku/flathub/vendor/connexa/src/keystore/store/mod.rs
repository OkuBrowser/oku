#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod fs;
#[cfg(all(feature = "indexeddb", target_arch = "wasm32"))]
pub mod indexeddb;
pub mod memory;
#[cfg(all(feature = "redb", not(target_arch = "wasm32")))]
pub mod redb;
#[cfg(all(feature = "sqlite", not(target_arch = "wasm32")))]
pub mod sqlite;
