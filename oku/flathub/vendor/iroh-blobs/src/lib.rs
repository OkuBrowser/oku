#![cfg_attr(iroh_blobs_docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]
//! # Module docs
//!
//! The crate is designed to be used from the [iroh] crate.
//!
//! It implements a [protocol] for streaming content-addressed data transfer using
//! [BLAKE3] verified streaming.
//!
//! It also provides a [store] module for storage of blobs and outboards,
//! as well as a [persistent](crate::store::fs) and a [memory](crate::store::mem)
//! store implementation.
//!
//! To implement a server, the [provider] module provides helpers for handling
//! connections and individual requests given a store.
//!
//! To perform get requests, the [get] module provides utilities to perform
//! requests and store the result in a store, as well as a low level state
//! machine for executing requests.
//!
//! The client API is available in the [api] module. You can get a client
//! either from one of the [store] implementations, or from the [BlobsProtocol]
//! via a
//!
//! The [downloader](api::downloader) module provides a component to download blobs from
//! multiple sources and store them in a store.
//!
//! # Features:
//!
//! - `fs-store`: Enables the filesystem based store implementation. This comes with a few additional dependencies such as `redb` and `reflink-copy`.
//! - `metrics`: Enables prometheus metrics for stores and the protocol.
//!
//! [BLAKE3]: https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf
//! [iroh]: https://docs.rs/iroh
mod hash;
pub mod store;
pub use hash::{BlobFormat, Hash, HashAndFormat};
pub mod api;

pub mod format;
pub mod get;
pub mod hashseq;
mod metrics;
mod net_protocol;
pub use net_protocol::BlobsProtocol;
pub mod protocol;
pub mod provider;
pub mod ticket;

#[doc(hidden)]
pub mod test;
pub mod util;

#[cfg(test)]
#[cfg(feature = "fs-store")]
mod tests;

pub use protocol::ALPN;
