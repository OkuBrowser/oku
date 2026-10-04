use std::{env, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use iroh::{address_lookup::MemoryLookup, endpoint::presets, SecretKey};
use iroh_blobs::{
    api::downloader::Shuffled,
    provider::events::{AbortReason, EventMask, EventSender, ProviderMessage},
    store::fs::FsStore,
    test::{add_hash_sequences, create_random_blobs},
    HashAndFormat,
};
use iroh_tickets::endpoint::EndpointTicket;
use irpc::RpcMessage;
use n0_future::StreamExt;
use rand::{rngs::StdRng, RngExt, SeedableRng};
use tokio::signal::ctrl_c;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Commands to run
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Debug)]
pub struct CommonArgs {
    /// Random seed for reproducible results
    #[arg(long)]
    pub seed: Option<u64>,

    /// Path for store, none for in-memory store
    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Provide content to the network
    Provide(ProvideArgs),
    /// Request content from the network
    Request(RequestArgs),
}

#[derive(Parser, Debug)]
pub struct ProvideArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    /// Number of blobs to generate
    #[arg(long, default_value_t = 100)]
    pub num_blobs: usize,

    /// Size of each blob in bytes
    #[arg(long, default_value_t = 100000)]
    pub blob_size: usize,

    /// Number of hash sequences
    #[arg(long, default_value_t = 1)]
    pub hash_seqs: usize,

    /// Size of each hash sequence
    #[arg(long, default_value_t = 100)]
    pub hash_seq_size: usize,

    /// Size of each hash sequence
    #[arg(long, default_value_t = false)]
    pub allow_push: bool,
}

#[derive(Parser, Debug)]
pub struct RequestArgs {
    #[command(flatten)]
    pub common: CommonArgs,

    /// Hash of the blob to request
    #[arg(long)]
    pub content: Vec<HashAndFormat>,

    /// Nodes to request from
    pub nodes: Vec<EndpointTicket>,

    /// Split large requests
    #[arg(long, default_value_t = false)]
    pub split: bool,
}

pub fn get_or_generate_secret_key() -> Result<SecretKey> {
    if let Ok(secret) = env::var("IROH_SECRET") {
        // Parse the secret key from string
        SecretKey::from_str(&secret).context("Invalid secret key format")
    } else {
        // Generate a new random key
        let secret_key = SecretKey::generate();
        let secret_key_str = hex::encode(secret_key.to_bytes());
        println!("Generated new random secret key");
        println!("To reuse this key, set the IROH_SECRET={secret_key_str}");
        Ok(secret_key)
    }
}

pub fn dump_provider_events(allow_push: bool) -> (tokio::task::JoinHandle<()>, EventSender) {
    let (tx, mut rx) = EventSender::channel(100, EventMask::ALL_READONLY);
    fn dump_updates<T: RpcMessage>(mut rx: irpc::channel::mpsc::Receiver<T>) {
        tokio::spawn(async move {
            while let Ok(Some(update)) = rx.recv().await {
                println!("{update:?}");
            }
        });
    }
    let dump_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                ProviderMessage::ClientConnected(msg) => {
                    println!("{:?}", msg.inner);
                    msg.tx.send(Ok(())).await.ok();
                }
                ProviderMessage::ClientConnectedNotify(msg) => {
                    println!("{:?}", msg.inner);
                }
                ProviderMessage::ConnectionClosed(msg) => {
                    println!("{:?}", msg.inner);
                }
                ProviderMessage::GetRequestReceived(msg) => {
                    println!("{:?}", msg.inner);
                    msg.tx.send(Ok(())).await.ok();
                    dump_updates(msg.rx);
                }
                ProviderMessage::GetRequestReceivedNotify(msg) => {
                    println!("{:?}", msg.inner);
                    dump_updates(msg.rx);
                }
                ProviderMessage::GetManyRequestReceived(msg) => {
                    println!("{:?}", msg.inner);
                    msg.tx.send(Ok(())).await.ok();
                    dump_updates(msg.rx);
                }
                ProviderMessage::GetManyRequestReceivedNotify(msg) => {
                    println!("{:?}", msg.inner);
                    dump_updates(msg.rx);
                }
                ProviderMessage::PushRequestReceived(msg) => {
                    println!("{:?}", msg.inner);
                    let res = if allow_push {
                        Ok(())
                    } else {
                        Err(AbortReason::Permission)
                    };
                    msg.tx.send(res).await.ok();
                    dump_updates(msg.rx);
                }
                ProviderMessage::PushRequestReceivedNotify(msg) => {
                    println!("{:?}", msg.inner);
                    dump_updates(msg.rx);
                }
                ProviderMessage::ObserveRequestReceived(msg) => {
                    println!("{:?}", msg.inner);
                    let res = if allow_push {
                        Ok(())
                    } else {
                        Err(AbortReason::Permission)
                    };
                    msg.tx.send(res).await.ok();
                    dump_updates(msg.rx);
                }
                ProviderMessage::ObserveRequestReceivedNotify(msg) => {
                    println!("{:?}", msg.inner);
                    dump_updates(msg.rx);
                }
                ProviderMessage::Throttle(msg) => {
                    println!("{:?}", msg.inner);
                    msg.tx.send(Ok(())).await.ok();
                }
            }
        }
    });
    (dump_task, tx)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    match args.command {
        Commands::Provide(args) => provide(args).await,
        Commands::Request(args) => request(args).await,
    }
}

async fn provide(args: ProvideArgs) -> anyhow::Result<()> {
    println!("{args:?}");
    let tempdir = if args.common.path.is_none() {
        Some(tempfile::tempdir_in(".").context("Failed to create temporary directory")?)
    } else {
        None
    };
    let path = args
        .common
        .path
        .unwrap_or_else(|| tempdir.as_ref().unwrap().path().to_path_buf());
    let store = FsStore::load(&path).await?;
    println!("Using store at: {}", path.display());
    let mut rng = match args.common.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_rng(&mut rand::rng()),
    };
    let blobs = create_random_blobs(
        &store,
        args.num_blobs,
        |_, rand| rand.random_range(1..=args.blob_size),
        &mut rng,
    )
    .await?;
    let hs = add_hash_sequences(
        &store,
        &blobs,
        args.hash_seqs,
        |_, rand| rand.random_range(1..=args.hash_seq_size),
        &mut rng,
    )
    .await?;
    println!(
        "Created {} blobs and {} hash sequences",
        blobs.len(),
        hs.len()
    );
    for (i, info) in blobs.iter().enumerate() {
        println!("blob {i} {}", info.hash_and_format());
    }
    for (i, info) in hs.iter().enumerate() {
        println!("hash_seq {i} {}", info.hash_and_format());
    }
    let secret_key = get_or_generate_secret_key()?;
    let endpoint = iroh::Endpoint::builder(presets::N0)
        .secret_key(secret_key)
        .bind()
        .await?;
    let (dump_task, events_tx) = dump_provider_events(args.allow_push);
    let blobs = iroh_blobs::BlobsProtocol::new(&store, Some(events_tx));
    let router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_blobs::ALPN, blobs)
        .spawn();
    let addr = router.endpoint().addr();
    let ticket = EndpointTicket::from(addr.clone());
    println!("Node address: {addr:?}");
    println!("ticket:\n{ticket}");
    ctrl_c().await?;
    router.shutdown().await?;
    dump_task.abort();
    Ok(())
}

async fn request(args: RequestArgs) -> anyhow::Result<()> {
    println!("{args:?}");
    let tempdir = if args.common.path.is_none() {
        Some(tempfile::tempdir_in(".").context("Failed to create temporary directory")?)
    } else {
        None
    };
    let path = args
        .common
        .path
        .unwrap_or_else(|| tempdir.as_ref().unwrap().path().to_path_buf());
    let store = FsStore::load(&path).await?;
    println!("Using store at: {}", path.display());
    let sp = MemoryLookup::new();
    let endpoint = iroh::Endpoint::builder(presets::N0)
        .address_lookup(sp.clone())
        .bind()
        .await?;
    let downloader = store.downloader(&endpoint);
    for ticket in &args.nodes {
        sp.add_endpoint_info(ticket.endpoint_addr().clone());
    }
    let nodes = args
        .nodes
        .iter()
        .map(|ticket| ticket.endpoint_addr().id)
        .collect::<Vec<_>>();
    for content in args.content {
        let mut progress = downloader
            .download(content, Shuffled::new(nodes.clone()))
            .stream()
            .await?;
        while let Some(event) = progress.next().await {
            info!("Progress: {:?}", event);
        }
    }
    let hashes = store.list().hashes().await?;
    for hash in hashes {
        println!("Got {hash}");
    }
    store.dump().await?;
    endpoint.close().await;
    Ok(())
}
