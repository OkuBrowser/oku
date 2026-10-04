#![doc = include_str!("../README.md")]

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod arc;
#[cfg(feature = "alloc")]
mod rc;
mod either;

#[cfg(feature = "alloc")]
pub use arc::ArcError;
#[cfg(feature = "alloc")]
pub use rc::RcError;
pub use either::EitherError;