use std::path::PathBuf;

use iroh::{endpoint::presets, protocol::Router, Endpoint};
use iroh_blobs::{store::mem::MemStore, ticket::BlobTicket, BlobsProtocol};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create an endpoint, it allows creating and accepting
    // connections in the iroh p2p world
    let endpoint = Endpoint::bind(presets::N0).await?;

    // We initialize an in-memory backing store for iroh-blobs
    let store = MemStore::new();
    // Then we initialize a struct that can accept blobs requests over iroh connections
    let blobs = BlobsProtocol::new(&store, None);

    // Grab all passed in arguments, the first one is the binary itself, so we skip it.
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Convert to &str, so we can pattern-match easily:
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    match arg_refs.as_slice() {
        ["send", filename] => {
            let filename: PathBuf = filename.parse()?;
            let abs_path = std::path::absolute(&filename)?;

            println!("Hashing file.");

            // When we import a blob, we get back a "tag" that refers to said blob in the store
            // and allows us to control when/if it gets garbage-collected
            let tag = store.blobs().add_path(abs_path).await?;

            let node_id = endpoint.id();
            let ticket = BlobTicket::new(node_id.into(), tag.hash, tag.format);

            println!("File hashed. Fetch this file by running:");
            println!(
                "cargo run --example transfer -- receive {ticket} {}",
                filename.display()
            );

            // For sending files we build a router that accepts blobs connections & routes them
            // to the blobs protocol.
            let router = Router::builder(endpoint)
                .accept(iroh_blobs::ALPN, blobs)
                .spawn();

            tokio::signal::ctrl_c().await?;

            // Gracefully shut down the node
            println!("Shutting down.");
            router.shutdown().await?;
        }
        ["receive", ticket, filename] => {
            let filename: PathBuf = filename.parse()?;
            let abs_path = std::path::absolute(filename)?;
            let ticket: BlobTicket = ticket.parse()?;

            // For receiving files, we create a "downloader" that allows us to fetch files
            // from other nodes via iroh connections
            let downloader = store.downloader(&endpoint);

            println!("Starting download.");

            downloader
                .download(ticket.hash(), Some(ticket.addr().id))
                .await?;

            println!("Finished download.");
            println!("Copying to destination.");

            store.blobs().export(ticket.hash(), abs_path).await?;

            println!("Finished copying.");

            // Gracefully shut down the node
            println!("Shutting down.");
            endpoint.close().await;
        }
        _ => {
            println!("Couldn't parse command line arguments: {args:?}");
            println!("Usage:");
            println!("    # to send:");
            println!("    cargo run --example transfer -- send [FILE]");
            println!("    # this will print a ticket.");
            println!();
            println!("    # to receive:");
            println!("    cargo run --example transfer -- receive [TICKET] [FILE]");
        }
    }

    Ok(())
}
