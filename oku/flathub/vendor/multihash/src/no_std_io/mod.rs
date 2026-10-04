//! No-std I/O utilities for the multihash crate.
//!
//! This module provides a minimal compatibility layer for I/O operations in `no_std` environments.
//! Source code is ported and adapted from the [`core2`](https://docs.rs/crate/core2/0.4.0/source/) crate.

mod error;
mod impls;
mod traits;

pub use error::Error;
pub use traits::{Read, Write};
