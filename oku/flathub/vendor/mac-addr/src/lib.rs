#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(feature = "serde")]
extern crate serde;

// EUI-48 (6 bytes)
mod addr; 
// EUI-64 (8 bytes)
mod addr8; 
mod error; 

pub use addr::MacAddr;
pub use addr8::MacAddr8;
pub use error::ParseMacAddrError;
