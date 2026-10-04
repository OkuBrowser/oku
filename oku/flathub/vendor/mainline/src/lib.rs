#![doc = include_str!("../README.md")]
//! ## Feature flags
#![doc = document_features::document_features!()]
//!

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

mod common;
#[cfg(feature = "node")]
mod dht;
mod rpc;

// Public modules
#[cfg(feature = "async")]
pub mod async_dht;

pub use common::{Id, MutableItem, Node, RoutingTable};

#[cfg(feature = "node")]
pub use dht::{Dht, DhtBuilder, Testnet, TestnetBuilder};
#[cfg(feature = "node")]
pub use rpc::{
    config::Config,
    messages::{MessageType, PutRequestSpecific, RequestSpecific},
    server::{RequestFilter, ServerSettings, MAX_INFO_HASHES, MAX_PEERS, MAX_VALUES},
    ClosestNodes, GetMutableOutcome, PutOutcome, DEFAULT_BOOTSTRAP_NODES, DEFAULT_REQUEST_TIMEOUT,
};

pub use ed25519_dalek::SigningKey;

pub mod errors {
    //! Exported errors
    #[cfg(feature = "node")]
    pub use super::common::ErrorSpecific;
    #[cfg(feature = "node")]
    pub use super::dht::PutMutableError;
    #[cfg(feature = "node")]
    pub use super::rpc::{ConcurrencyError, PutError, PutQueryError};

    pub use super::common::DecodeIdError;
    pub use super::common::MutableError;
}
