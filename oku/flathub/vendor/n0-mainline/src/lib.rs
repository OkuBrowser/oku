#![doc = include_str!("../README.md")]
#![cfg_attr(n0_mainline_docsrs, feature(doc_cfg))]
#![deny(missing_docs, unused_must_use)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(
    clippy::panic,
    clippy::unwrap_used,
    // clippy::expect_used,
    clippy::await_holding_lock,
    // clippy::indexing_slicing,
    clippy::await_holding_refcell_ref
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

/// Single threaded Actor model node
mod actor;
mod common;
/// Functional core testable separately from I/O
mod core;
mod dht;

pub use common::{
    ClosestNodes, Id, MutableItem, Node,
    messages::{PutRequestSpecific, RequestSpecific},
};
pub use core::server::{RequestFilter, ServerSettings};
pub use dht::{ActorShutdown, Dht, DhtBuilder, GetStream, Testnet};

pub use ed25519_dalek::SigningKey;

pub mod errors {
    //! Exported errors
    pub use super::core::{ConcurrencyError, PutError, PutQueryError};
    pub use super::dht::PutMutableError;

    pub use super::common::DecodeIdError;
}
