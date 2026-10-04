//! Example for adding a custom protocol to a iroh node.
//!
//! We are building a very simple custom protocol here, and make our iroh nodes speak this protocol
//! in addition to a protocol that is provider by number0, iroh-blobs.
//!
//! Our custom protocol allows querying the blob store of other nodes for text matches. For
//! this, we keep a very primitive index of the UTF-8 text of our blobs.
//!
//! The example is contrived - we only use memory nodes, and our database is a hashmap in a mutex,
//! and our queries just match if the query string appears as-is in a blob.
//! Nevertheless, this shows how powerful systems can be built with custom protocols by also using
//! the existing iroh protocols (blobs in this case).
//!
//! ## Usage
//!
//! In one terminal, run
//!
//!     cargo run --example custom-protocol -- listen "hello-world" "foo-bar" "hello-moon"
//!
//! This spawns an iroh node with three blobs. It will print the node's endpoint id.
//!
//! In another terminal, run
//!
//!     cargo run --example custom-protocol -- query <endpoint-id> hello
//!
//! Replace <endpoint-id> with the endpoint id from above. This will connect to the listening node with our
//! custom protocol and query for the string `hello`. The listening node will return a list of
//! blob hashes that contain `hello`. We will then download all these blobs with iroh-blobs,
//! and then print a list of the hashes with their content.
//!
//! For this example, this will print:
//!
//!     7b54d6be55: hello-moon
//!     c92dabdf91: hello-world
//!
//! That's it! Follow along in the code below, we added a bunch of comments to explain things.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use clap::Parser;
use iroh::{
    address_lookup::PkarrResolver,
    endpoint::{presets, Connection},
    protocol::{AcceptError, ProtocolHandler, Router},
    Endpoint, EndpointId,
};
use iroh_blobs::{api::Store, store::mem::MemStore, BlobsProtocol, Hash};
mod common;
use common::{get_or_generate_secret_key, setup_logging};

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    /// Spawn a node in listening mode.
    Listen {
        /// Each text string will be imported as a blob and inserted into the search database.
        text: Vec<String>,
    },
    /// Query a remote node for data and print the results.
    Query {
        /// The endpoint id of the node we want to query.
        endpoint_id: EndpointId,
        /// The text we want to match.
        query: String,
    },
}

/// Each custom protocol is identified by its ALPN string.
///
/// The ALPN, or application-layer protocol negotiation, is exchanged in the connection handshake,
/// and the connection is aborted unless both nodes pass the same bytestring.
const ALPN: &[u8] = b"iroh-example/text-search/0";

async fn listen(text: Vec<String>) -> Result<()> {
    // allow the user to provide a secret so we can have a stable endpoint id.
    // This is only needed for the listen side.
    let secret_key = get_or_generate_secret_key()?;
    // Use an in-memory store for this example. You would use a persistent store in production code.
    let store = MemStore::new();
    // Create an endpoint with the secret key and address lookup publishing to the n0 dns server enabled.
    let endpoint = Endpoint::builder(presets::N0)
        .secret_key(secret_key)
        .bind()
        .await?;
    // Build our custom protocol handler. The `builder` exposes access to various subsystems in the
    // iroh node. In our case, we need a blobs client and the endpoint.
    let proto = BlobSearch::new(&store);
    // Insert the text strings as blobs and index them.
    for text in text.into_iter() {
        proto.insert_and_index(text).await?;
    }
    // Build the iroh-blobs protocol handler, which is used to download blobs.
    let blobs = BlobsProtocol::new(&store, None);

    // create a router that handles both our custom protocol and the iroh-blobs protocol.
    let node = Router::builder(endpoint)
        .accept(ALPN, proto.clone())
        .accept(iroh_blobs::ALPN, blobs.clone())
        .spawn();

    // Print our endpoint id, so clients know how to connect to us.
    let node_id = node.endpoint().id();
    println!("our endpoint id: {node_id}");

    // Wait for Ctrl-C to be pressed.
    tokio::signal::ctrl_c().await?;
    node.shutdown().await?;
    Ok(())
}

async fn query(endpoint_id: EndpointId, query: String) -> Result<()> {
    // Build a in-memory node. For production code, you'd want a persistent node instead usually.
    let store = MemStore::new();
    // Create an endpoint with a random secret key and no address lookup publishing.
    // For a client we just need address lookup resolution via the n0 dns server, which
    // the PkarrResolver provides.
    let endpoint = Endpoint::builder(presets::Minimal)
        .relay_mode(iroh::RelayMode::Default)
        .address_lookup(PkarrResolver::n0_dns())
        .bind()
        .await?;
    // Query the remote node.
    // This will send the query over our custom protocol, read hashes on the reply stream,
    // and download each hash over iroh-blobs.
    let hashes = query_remote(&endpoint, &store, endpoint_id, &query).await?;

    // Print out our query results.
    for hash in hashes {
        read_and_print(&store, hash).await?;
    }

    // Close the endpoint and shutdown the store.
    // Shutting down the store is not needed for a memory store, but would be important for persistent stores
    // to allow them to flush their data to disk.
    endpoint.close().await;
    store.shutdown().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();
    let args = Cli::parse();

    match args.command {
        Command::Listen { text } => {
            listen(text).await?;
        }
        Command::Query {
            endpoint_id,
            query: query_text,
        } => {
            query(endpoint_id, query_text).await?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct BlobSearch {
    blobs: Store,
    index: Arc<Mutex<HashMap<String, Hash>>>,
}

impl ProtocolHandler for BlobSearch {
    /// The `accept` method is called for each incoming connection for our ALPN.
    ///
    /// The returned future runs on a newly spawned tokio task, so it can run as long as
    /// the connection lasts.
    async fn accept(&self, connection: Connection) -> std::result::Result<(), AcceptError> {
        let this = self.clone();
        // We can get the remote's endpoint id from the connection.
        let node_id = connection.remote_id();
        println!("accepted connection from {node_id}");

        // Our protocol is a simple request-response protocol, so we expect the
        // connecting peer to open a single bi-directional stream.
        let (mut send, mut recv) = connection.accept_bi().await?;

        // We read the query from the receive stream, while enforcing a max query length.
        let query_bytes = recv.read_to_end(64).await.map_err(AcceptError::from_err)?;

        // Now, we can perform the actual query on our local database.
        let query = String::from_utf8(query_bytes).map_err(AcceptError::from_err)?;
        let hashes = this.query_local(&query);
        println!("query: {query}, found {} results", hashes.len());

        // We want to return a list of hashes. We do the simplest thing possible, and just send
        // one hash after the other. Because the hashes have a fixed size of 32 bytes, this is
        // very easy to parse on the other end.
        for hash in hashes {
            send.write_all(hash.as_bytes())
                .await
                .map_err(AcceptError::from_err)?;
        }

        // By calling `finish` on the send stream we signal that we will not send anything
        // further, which makes the receive stream on the other end terminate.
        send.finish()?;
        connection.closed().await;
        Ok(())
    }
}

impl BlobSearch {
    /// Create a new protocol handler.
    pub fn new(blobs: &Store) -> Arc<Self> {
        Arc::new(Self {
            blobs: blobs.clone(),
            index: Default::default(),
        })
    }

    /// Query the local database.
    ///
    /// Returns the list of hashes of blobs which contain `query` literally.
    pub fn query_local(&self, query: &str) -> Vec<Hash> {
        let db = self.index.lock().unwrap();
        db.iter()
            .filter_map(|(text, hash)| text.contains(query).then_some(*hash))
            .collect::<Vec<_>>()
    }

    /// Insert a text string into the database.
    ///
    /// This first imports the text as a blob into the iroh blob store, and then inserts a
    /// reference to that hash in our (primitive) text database.
    pub async fn insert_and_index(&self, text: String) -> Result<Hash> {
        let hash = self.blobs.add_bytes(text.into_bytes()).await?.hash;
        self.add_to_index(hash).await?;
        Ok(hash)
    }

    /// Index a blob which is already in our blob store.
    ///
    /// This only indexes complete blobs that are smaller than 1MiB.
    ///
    /// Returns `true` if the blob was indexed.
    async fn add_to_index(&self, hash: Hash) -> Result<bool> {
        let bitfield = self.blobs.observe(hash).await?;
        if !bitfield.is_complete() || bitfield.size() > 1024 * 1024 {
            // If the blob is not complete or too large, we do not index it.
            return Ok(false);
        }
        let data = self.blobs.get_bytes(hash).await?;
        match String::from_utf8(data.to_vec()) {
            Ok(text) => {
                let mut db = self.index.lock().unwrap();
                db.insert(text, hash);
                Ok(true)
            }
            Err(_err) => Ok(false),
        }
    }
}

/// Query a remote node, download all matching blobs and print the results.
pub async fn query_remote(
    endpoint: &Endpoint,
    store: &Store,
    endpoint_id: EndpointId,
    query: &str,
) -> Result<Vec<Hash>> {
    // Establish a connection to our node.
    // We use the default node address lookup in iroh, so we can connect by endpoint id without
    // providing further information.
    let conn = endpoint.connect(endpoint_id, ALPN).await?;
    let blobs_conn = endpoint.connect(endpoint_id, iroh_blobs::ALPN).await?;

    // Open a bi-directional in our connection.
    let (mut send, mut recv) = conn.open_bi().await?;

    // Send our query.
    send.write_all(query.as_bytes()).await?;

    // Finish the send stream, signalling that no further data will be sent.
    // This makes the `read_to_end` call on the accepting side terminate.
    send.finish()?;

    // In this example, we simply collect all results into a vector.
    // For real protocols, you'd usually want to return a stream of results instead.
    let mut out = vec![];

    // The response is sent as a list of 32-byte long hashes.
    // We simply read one after the other into a byte buffer.
    let mut hash_bytes = [0u8; 32];
    loop {
        // Read 32 bytes from the stream.
        match recv.read_exact(&mut hash_bytes).await {
            // FinishedEarly means that the remote side did not send further data,
            // so in this case we break our loop.
            Err(iroh::endpoint::ReadExactError::FinishedEarly(_)) => break,
            // Other errors are connection errors, so we bail.
            Err(err) => return Err(err.into()),
            Ok(_) => {}
        };
        // Upcast the raw bytes to the `Hash` type.
        let hash = Hash::from_bytes(hash_bytes);
        // Download the content via iroh-blobs.
        store.remote().fetch(blobs_conn.clone(), hash).await?;
        out.push(hash);
    }
    conn.close(0u32.into(), b"done");
    blobs_conn.close(0u32.into(), b"done");
    Ok(out)
}

/// Read a blob from the local blob store and print it to STDOUT.
async fn read_and_print(store: &Store, hash: Hash) -> Result<()> {
    let content = store.get_bytes(hash).await?;
    let message = String::from_utf8(content.to_vec())?;
    println!("{}: {message}", hash.fmt_short());
    Ok(())
}
