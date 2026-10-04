//! Adaptation of `iroh-blobs` as an [`iroh`] [`ProtocolHandler`].
//!
//! This is the easiest way to share data from a [`crate::api::Store`] over iroh connections.
//!
//! # Example
//!
//! ```rust
//! # async fn example() -> n0_error::Result<()> {
//! use iroh::{protocol::Router, Endpoint, endpoint::presets};
//! use iroh_blobs::{store, ticket::BlobTicket, BlobsProtocol};
//!
//! // create a store
//! let store = store::fs::FsStore::load("blobs").await?;
//!
//! // add some data
//! let t = store.add_slice(b"hello world").await?;
//!
//! // create an iroh endpoint
//! let endpoint = Endpoint::bind(presets::N0).await?;
//! endpoint.online().await;
//! let addr = endpoint.addr();
//!
//! // create a blobs protocol handler
//! let blobs = BlobsProtocol::new(&store, None);
//!
//! // create a router and add the blobs protocol handler
//! let router = Router::builder(endpoint)
//!     .accept(iroh_blobs::ALPN, blobs)
//!     .spawn();
//!
//! // this data is now globally available using the ticket
//! let ticket = BlobTicket::new(addr, t.hash, t.format);
//! println!("ticket: {}", ticket);
//!
//! // wait for control-c to exit
//! tokio::signal::ctrl_c().await?;
//! #   Ok(())
//! # }
//! ```

use std::{fmt::Debug, ops::Deref, sync::Arc};

use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
};
use tracing::error;

use crate::{api::Store, provider::events::EventSender};

#[derive(Debug)]
pub(crate) struct BlobsInner {
    store: Store,
    events: EventSender,
}

/// A protocol handler for the blobs protocol.
#[derive(Debug, Clone)]
pub struct BlobsProtocol {
    inner: Arc<BlobsInner>,
}

impl Deref for BlobsProtocol {
    type Target = Store;

    fn deref(&self) -> &Self::Target {
        &self.inner.store
    }
}

impl BlobsProtocol {
    pub fn new(store: &Store, events: Option<EventSender>) -> Self {
        Self {
            inner: Arc::new(BlobsInner {
                store: store.clone(),
                events: events.unwrap_or(EventSender::DEFAULT),
            }),
        }
    }

    pub fn store(&self) -> &Store {
        &self.inner.store
    }
}

impl ProtocolHandler for BlobsProtocol {
    async fn accept(&self, conn: Connection) -> std::result::Result<(), AcceptError> {
        let store = self.store().clone();
        let events = self.inner.events.clone();
        crate::provider::handle_connection(conn, store, events).await;
        Ok(())
    }

    async fn shutdown(&self) {
        if let Err(cause) = self.store().shutdown().await {
            error!("error shutting down store: {:?}", cause);
        }
    }
}
