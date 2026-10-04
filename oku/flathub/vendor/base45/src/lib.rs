#![cfg_attr(not(feature = "std"), no_std)]
//! Encoder/decoder for base45 that is fully compatible with
//! [`draft-faltstrom-base45-07.txt`](https://www.ietf.org/archive/id/draft-faltstrom-base45-07.txt)
//!
//! ```rust,no_run
//! # fn main() {
//!     let encoded = base45::encode("Hello!!");
//! # }
//! ```
//!
//! Features:
//!
//! - `array_chunks`, which is using experimental array and const-generic features. For information & tracking, see
//!   [rust/rust#74985](https://github.com/rust-lang/rust/issues/74985). If not enabled, this uses
//!   [`core::slice::ChunksExact`](https://doc.rust-lang.org/core/slice/struct.ChunksExact.html).
//!   Ideally, there is no performance penalty using this means.

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod alphabet;
mod decode;
mod encode;

pub use decode::*;
pub use encode::*;

#[cfg(test)]
mod tests;
