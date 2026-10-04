//! Example that runs an iroh node with local node address_lookup and no relay server.
//!
//! You can think of this as a local version of [sendme](https://www.iroh.computer/sendme)
//! that only works for individual files.
//!
//! **This example is using a non-default feature of iroh, so you need to run it with the
//! examples feature enabled.**
//!
//! Run the follow command to run the "accept" side, that hosts the content:
//!  $ cargo run --example mdns-address_lookup --features examples -- accept [FILE_PATH]
//! Wait for output that looks like the following:
//!  $ cargo run --example mdns-address_lookup --features examples -- connect [NODE_ID] [HASH] -o [FILE_PATH]
//! Run that command on another machine in the same local network, replacing [FILE_PATH] to the path on which you want to save the transferred content.
use std::path::{Path, PathBuf};

use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use iroh::{endpoint::presets, protocol::Router, Endpoint, PublicKey, RelayMode, SecretKey};
use iroh_blobs::{store::mem::MemStore, BlobsProtocol, Hash};
use iroh_mdns_address_lookup::MdnsAddressLookup;

mod common;
use common::{get_or_generate_secret_key, setup_logging};

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    /// Launch an iroh node and provide the content at the given path
    Accept {
        /// path to the file you want to provide
        path: PathBuf,
    },
    /// Get the node_id and hash string from a node running accept in the local network
    /// Download the content from that node.
    Connect {
        /// Endpoint ID of a node on the local network
        endpoint_id: PublicKey,
        /// Hash of content you want to download from the node
        hash: Hash,
        /// save the content to a file
        #[clap(long, short)]
        out: Option<PathBuf>,
    },
}

async fn accept(path: &Path) -> Result<()> {
    if !path.is_file() {
        println!("Content must be a file.");
        return Ok(());
    }

    let key = get_or_generate_secret_key()?;

    println!("Starting iroh node with mdns address lookup...");
    // create a new node
    let endpoint = Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Default)
        .secret_key(key)
        .address_lookup(MdnsAddressLookup::builder())
        .relay_mode(RelayMode::Disabled)
        .bind()
        .await?;
    let builder = Router::builder(endpoint.clone());
    let store = MemStore::new();
    let blobs = BlobsProtocol::new(&store, None);
    let builder = builder.accept(iroh_blobs::ALPN, blobs.clone());
    let node = builder.spawn();

    if !path.is_file() {
        println!("Content must be a file.");
        node.shutdown().await?;
        return Ok(());
    }
    let absolute = path.canonicalize()?;
    println!("Adding {} as {}...", path.display(), absolute.display());
    let tag = store.add_path(absolute).await?;
    println!(
        "To fetch the blob:\n\tcargo run --example mdns-address_lookup --features examples -- connect {} {} -o [FILE_PATH]",
        node.endpoint().id(),
        tag.hash
    );
    tokio::signal::ctrl_c().await?;
    node.shutdown().await?;
    Ok(())
}

async fn connect(node_id: PublicKey, hash: Hash, out: Option<PathBuf>) -> Result<()> {
    let key = SecretKey::generate();
    // todo: disable address_lookup publishing once https://github.com/n0-computer/iroh/issues/3401 is implemented
    let address_lookup = MdnsAddressLookup::builder();

    println!("Starting iroh node with mdns address_lookup...");
    // create a new node
    let endpoint = Endpoint::builder(presets::Minimal)
        .secret_key(key)
        .address_lookup(address_lookup)
        .bind()
        .await?;
    let store = MemStore::new();

    println!("NodeID: {}", endpoint.id());
    let conn = endpoint.connect(node_id, iroh_blobs::ALPN).await?;
    let stats = store.remote().fetch(conn, hash).await?;
    println!(
        "Fetched {} bytes for hash {}",
        stats.payload_bytes_read, hash
    );
    if let Some(path) = out {
        let absolute = std::env::current_dir()?.join(&path);
        ensure!(!absolute.is_dir(), "output must not be a directory");
        println!(
            "exporting {hash} to {} -> {}",
            path.display(),
            absolute.display()
        );
        let size = store.export(hash, absolute).await?;
        println!("Exported {size} bytes");
    }

    endpoint.close().await;
    // Shutdown the store. This is not needed for the mem store, but would be
    // necessary for a persistent store to allow it to write any pending data to disk.
    store.shutdown().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Accept { path } => {
            accept(path).await?;
        }
        Commands::Connect {
            endpoint_id,
            hash,
            out,
        } => {
            connect(*endpoint_id, *hash, out.clone()).await?;
        }
    }
    Ok(())
}
