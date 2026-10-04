//! Example that shows how to create a collection, and transfer it to another
//! node. It also shows patterns for defining a "Node" struct in higher-level
//! code that abstracts over these operations with an API that feels closer to
//! what an application would use.
//!
//! Run the entire example in one command:
//!  $ cargo run --example transfer-collection
use std::collections::HashMap;

use anyhow::{Context, Result};
use iroh::{
    address_lookup::MemoryLookup, endpoint::presets, protocol::Router, Endpoint, EndpointAddr,
    RelayMode,
};
use iroh_blobs::{
    api::{downloader::Shuffled, Store, TempTag},
    format::collection::Collection,
    store::mem::MemStore,
    BlobsProtocol, Hash, HashAndFormat,
};

/// Node is something you'd define in your application. It can contain whatever
/// shared state you'd want to couple with network operations.
struct Node {
    store: Store,
    /// Router with the blobs protocol registered, to accept blobs requests.
    /// We can always get the endpoint with router.endpoint()
    router: Router,
}

impl Node {
    async fn new(disc: &MemoryLookup) -> Result<Self> {
        let endpoint = Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Default)
            .address_lookup(disc.clone())
            .bind()
            .await?;

        let store = MemStore::new();

        // this BlobsProtocol accepts connections from other nodes and serves blobs from the store
        // we pass None to skip subscribing to request events
        let blobs = BlobsProtocol::new(&store, None);
        // Routers group one or more protocols together to accept connections from other nodes,
        // here we're only using one, but could add more in a real world use case as needed
        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, blobs)
            .spawn();

        Ok(Self {
            store: store.into(),
            router,
        })
    }

    // get address of this node. Has the side effect of waiting for the node
    // to be online & ready to accept connections
    async fn node_addr(&self) -> Result<EndpointAddr> {
        self.router.endpoint().online().await;
        let addr = self.router.endpoint().addr();
        Ok(addr)
    }

    async fn list_hashes(&self) -> Result<Vec<Hash>> {
        self.store
            .blobs()
            .list()
            .hashes()
            .await
            .context("Failed to list hashes")
    }

    /// creates a collection from a given set of named blobs, adds it to the local store
    /// and returns the hash of the collection.
    async fn create_collection(&self, named_blobs: Vec<(&str, Vec<u8>)>) -> Result<Hash> {
        let mut collection_items: HashMap<&str, TempTag> = HashMap::new();

        let tx = self.store.batch().await?;
        for (name, data) in named_blobs {
            let tmp_tag = tx.add_bytes(data).await?;
            collection_items.insert(name, tmp_tag);
        }

        let collection_items = collection_items
            .iter()
            .map(|(name, tag)| (name.to_string(), tag.hash()))
            .collect::<Vec<_>>();

        let collection = Collection::from_iter(collection_items);

        let tt = collection.store(&self.store).await?;
        self.store.tags().create(tt.hash_and_format()).await?;
        Ok(tt.hash())
    }

    /// retrieve an entire collection from a given hash and provider
    async fn get_collection(&self, hash: Hash, provider: EndpointAddr) -> Result<()> {
        let req = HashAndFormat::hash_seq(hash);
        let addrs = Shuffled::new(vec![provider.id]);
        self.store
            .downloader(self.router.endpoint())
            .download(req, addrs)
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create a local provider for nodes to discover each other.
    // outside of a development environment, production apps would
    // use `Endpoint::bind()` or a similar method
    let disc = MemoryLookup::new();

    // create a sending node
    let send_node = Node::new(&disc).await?;
    let send_node_addr = send_node.node_addr().await?;
    // add a collection with three files
    let hash = send_node
        .create_collection(vec![
            ("a.txt", b"this is file a".into()),
            ("b.txt", b"this is file b".into()),
            ("c.txt", b"this is file c".into()),
        ])
        .await?;

    // create the receiving node
    let recv_node = Node::new(&disc).await?;

    // add the send node to the address lookup provider so the recv node can find it
    disc.add_endpoint_info(send_node_addr.clone());
    // fetch the collection and all contents
    recv_node.get_collection(hash, send_node_addr).await?;

    // when listing hashes, you'll see 5 hashes in total:
    // - one hash for each of the three files
    // - hash of the collection's metadata (this is where the "a.txt" filenames live)
    // - the hash of the entire collection which is just the above 4 hashes concatenated, then hashed
    let send_hashes = send_node.list_hashes().await?;
    let recv_hashes = recv_node.list_hashes().await?;
    assert_eq!(send_hashes.len(), recv_hashes.len());

    println!("Transfer complete!");
    send_node.router.shutdown().await?;
    recv_node.router.shutdown().await?;
    Ok(())
}
