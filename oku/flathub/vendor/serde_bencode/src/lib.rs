//! This crate is a Rust library for using the [Serde](https://github.com/serde-rs/serde)
//! serialization framework with bencode data.
//!
//! # Examples
//!
//! ```
//! use serde_derive::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
//! struct Product {
//!     name: String,
//!     price: u32,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let apple = Product {
//!         name: "Apple".to_string(),
//!         price: 130,
//!     };
//!
//!     let serialized = serde_bencode::to_string(&apple)?;
//!     
//!     // cspell:disable-next-line
//!     assert_eq!(serialized, "d4:name5:Apple5:pricei130ee".to_string());
//!
//!     let deserialized: Product = serde_bencode::from_str(&serialized)?;
//!
//!     assert_eq!(
//!         deserialized,
//!         Product {
//!             name: "Apple".to_string(),
//!             price: 130,
//!         }
//!     );
//!
//!     Ok(())
//! }
//! ```

pub mod de;
pub mod error;
pub mod ser;
pub mod value;

pub use de::{from_bytes, from_str, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_bytes, to_string, Serializer};
