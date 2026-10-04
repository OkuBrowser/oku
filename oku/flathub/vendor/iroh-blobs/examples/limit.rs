/// Example how to limit blob requests by hash and endpoint id, and to add
/// throttling or limiting the maximum number of connections.
///
/// Limiting is done via a fn that returns an EventSender and internally
/// makes liberal use of spawn to spawn background tasks.
///
/// This is fine, since the tasks will terminate as soon as the [BlobsProtocol]
/// instance holding the [EventSender] will be dropped. But for production
/// grade code you might nevertheless put the tasks into a [tokio::task::JoinSet] or
/// [n0_future::FuturesUnordered].
mod common;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::Result;
use clap::Parser;
use common::setup_logging;
use iroh::{endpoint::presets, protocol::Router, EndpointAddr, EndpointId, SecretKey};
use iroh_blobs::{
    provider::events::{
        AbortReason, ConnectMode, EventMask, EventSender, ProviderMessage, RequestMode,
        ThrottleMode,
    },
    store::mem::MemStore,
    ticket::BlobTicket,
    BlobFormat, BlobsProtocol, Hash,
};

use crate::common::get_or_generate_secret_key;

#[derive(Debug, Parser)]
#[command(version, about)]
pub enum Args {
    /// Limit requests by endpoint id
    ByEndpointId {
        /// Path for files to add.
        paths: Vec<PathBuf>,
        #[clap(long("allow"))]
        /// Endpoints that are allowed to download content.
        allowed_endpoints: Vec<EndpointId>,
        /// Number of secrets to generate for allowed endpoint ids.
        #[clap(long, default_value_t = 1)]
        secrets: usize,
    },
    /// Limit requests by hash, only first hash is allowed
    ByHash {
        /// Path for files to add.
        paths: Vec<PathBuf>,
    },
    /// Throttle requests
    Throttle {
        /// Path for files to add.
        paths: Vec<PathBuf>,
        /// Delay in milliseconds after sending a chunk group of 16 KiB.
        #[clap(long, default_value = "100")]
        delay_ms: u64,
    },
    /// Limit maximum number of connections.
    MaxConnections {
        /// Path for files to add.
        paths: Vec<PathBuf>,
        /// Maximum number of concurrent get requests.
        #[clap(long, default_value = "1")]
        max_connections: usize,
    },
    /// Get a blob. Just for completeness sake.
    Get {
        /// Ticket for the blob to download
        ticket: BlobTicket,
    },
}

fn limit_by_node_id(allowed_nodes: HashSet<EndpointId>) -> EventSender {
    let mask = EventMask {
        // We want a request for each incoming connection so we can accept
        // or reject them. We don't need any other events.
        connected: ConnectMode::Intercept,
        ..EventMask::DEFAULT
    };
    let (tx, mut rx) = EventSender::channel(32, mask);
    n0_future::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let ProviderMessage::ClientConnected(msg) = msg {
                let res: std::result::Result<(), AbortReason> = match msg.endpoint_id {
                    Some(endpoint_id) if allowed_nodes.contains(&endpoint_id) => {
                        println!("Client connected: {endpoint_id}");
                        Ok(())
                    }
                    Some(endpoint_id) => {
                        println!("Client rejected: {endpoint_id}");
                        Err(AbortReason::Permission)
                    }
                    None => {
                        println!("Client rejected: no endpoint id");
                        Err(AbortReason::Permission)
                    }
                };
                msg.tx.send(res).await.ok();
            }
        }
    });
    tx
}

fn limit_by_hash(allowed_hashes: HashSet<Hash>) -> EventSender {
    let mask = EventMask {
        // We want to get a request for each get request that we can answer
        // with OK or not OK depending on the hash. We do not want detailed
        // events once it has been decided to handle a request.
        get: RequestMode::Intercept,
        ..EventMask::DEFAULT
    };
    let (tx, mut rx) = EventSender::channel(32, mask);
    n0_future::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let ProviderMessage::GetRequestReceived(msg) = msg {
                let res = if !msg.request.ranges.is_blob() {
                    println!("HashSeq request not allowed");
                    Err(AbortReason::Permission)
                } else if !allowed_hashes.contains(&msg.request.hash) {
                    println!("Request for hash {} not allowed", msg.request.hash);
                    Err(AbortReason::Permission)
                } else {
                    println!("Request for hash {} allowed", msg.request.hash);
                    Ok(())
                };
                msg.tx.send(res).await.ok();
            }
        }
    });
    tx
}

fn throttle(delay_ms: u64) -> EventSender {
    let mask = EventMask {
        // We want to get requests for each sent user data blob, so we can add a delay.
        // Other than that, we don't need any events.
        throttle: ThrottleMode::Intercept,
        ..EventMask::DEFAULT
    };
    let (tx, mut rx) = EventSender::channel(32, mask);
    n0_future::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let ProviderMessage::Throttle(msg) = msg {
                n0_future::task::spawn(async move {
                    println!(
                        "Throttling {} {}, {}ms",
                        msg.connection_id, msg.request_id, delay_ms
                    );
                    // we could compute the delay from the size of the data to have a fixed rate.
                    // but the size is almost always 16 KiB (16 chunks).
                    n0_future::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    msg.tx.send(Ok(())).await.ok();
                });
            }
        }
    });
    tx
}

fn limit_max_connections(max_connections: usize) -> EventSender {
    #[derive(Default, Debug, Clone)]
    struct ConnectionCounter(Arc<(AtomicUsize, usize)>);

    impl ConnectionCounter {
        fn new(max: usize) -> Self {
            Self(Arc::new((Default::default(), max)))
        }

        fn inc(&self) -> Result<usize, usize> {
            let (c, max) = &*self.0;
            c.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                if n >= *max {
                    None
                } else {
                    Some(n + 1)
                }
            })
        }

        fn dec(&self) {
            let (c, _) = &*self.0;
            c.fetch_sub(1, Ordering::SeqCst);
        }
    }

    let mask = EventMask {
        // For each get request, we want to get a request so we can decide
        // based on the current connection count if we want to accept or reject.
        // We also want detailed logging of events for the get request, so we can
        // detect when the request is finished one way or another.
        connected: ConnectMode::Intercept,
        ..EventMask::DEFAULT
    };
    let (tx, mut rx) = EventSender::channel(32, mask);
    n0_future::task::spawn(async move {
        let requests = ConnectionCounter::new(max_connections);
        while let Some(msg) = rx.recv().await {
            match msg {
                ProviderMessage::ClientConnected(msg) => {
                    let connection_id = msg.connection_id;
                    let node_id = msg.endpoint_id;
                    let res = if let Ok(n) = requests.inc() {
                        println!(
                            "Accepting connection {n}, node_id {node_id:?}, connection_id {connection_id}"
                        );
                        Ok(())
                    } else {
                        Err(AbortReason::RateLimited)
                    };
                    msg.tx.send(res).await.ok();
                }
                ProviderMessage::ConnectionClosed(msg) => {
                    requests.dec();
                    println!("Connection closed, connection_id {}", msg.connection_id,);
                }
                _ => {}
            }
        }
    });
    tx
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();
    let args = Args::parse();
    let secret = get_or_generate_secret_key()?;
    let endpoint = iroh::Endpoint::builder(presets::N0)
        .secret_key(secret)
        .bind()
        .await?;
    match args {
        Args::Get { ticket } => {
            let connection = endpoint
                .connect(ticket.addr().clone(), iroh_blobs::ALPN)
                .await?;
            let (data, stats) = iroh_blobs::get::request::get_blob(connection, ticket.hash())
                .bytes_and_stats()
                .await?;
            println!("Downloaded {} bytes", data.len());
            println!("Stats: {stats:?}");
        }
        Args::ByEndpointId {
            paths,
            allowed_endpoints,
            secrets,
        } => {
            let mut allowed_endpoints = allowed_endpoints.into_iter().collect::<HashSet<_>>();
            if secrets > 0 {
                println!("Generating {secrets} new secret keys for allowed endpoints:");
                for _ in 0..secrets {
                    let secret = SecretKey::generate();
                    let public = secret.public();
                    allowed_endpoints.insert(public);
                    println!("IROH_SECRET={}", hex::encode(secret.to_bytes()));
                }
            }

            let store = MemStore::new();
            let hashes = add_paths(&store, paths).await?;
            let events = limit_by_node_id(allowed_endpoints.clone());
            let (router, addr) = setup(store, events).await?;

            for (path, hash) in hashes {
                let ticket = BlobTicket::new(addr.clone(), hash, BlobFormat::Raw);
                println!("{}: {ticket}", path.display());
            }
            println!();
            println!("Endpoint id: {}\n", router.endpoint().id());
            for id in &allowed_endpoints {
                println!("Allowed endpoint: {id}");
            }

            tokio::signal::ctrl_c().await?;
            router.shutdown().await?;
        }
        Args::ByHash { paths } => {
            let store = MemStore::new();

            let mut hashes = HashMap::new();
            let mut allowed_hashes = HashSet::new();
            for (i, path) in paths.into_iter().enumerate() {
                let tag = store.add_path(&path).await?;
                hashes.insert(path, tag.hash);
                if i == 0 {
                    allowed_hashes.insert(tag.hash);
                }
            }

            let events = limit_by_hash(allowed_hashes.clone());
            let (router, addr) = setup(store, events).await?;

            for (path, hash) in hashes.iter() {
                let ticket = BlobTicket::new(addr.clone(), *hash, BlobFormat::Raw);
                let permitted = if allowed_hashes.contains(hash) {
                    "allowed"
                } else {
                    "forbidden"
                };
                println!("{}: {ticket} ({permitted})", path.display());
            }
            tokio::signal::ctrl_c().await?;
            router.shutdown().await?;
        }
        Args::Throttle { paths, delay_ms } => {
            let store = MemStore::new();
            let hashes = add_paths(&store, paths).await?;
            let events = throttle(delay_ms);
            let (router, addr) = setup(store, events).await?;
            for (path, hash) in hashes {
                let ticket = BlobTicket::new(addr.clone(), hash, BlobFormat::Raw);
                println!("{}: {ticket}", path.display());
            }
            tokio::signal::ctrl_c().await?;
            router.shutdown().await?;
        }
        Args::MaxConnections {
            paths,
            max_connections,
        } => {
            let store = MemStore::new();
            let hashes = add_paths(&store, paths).await?;
            let events = limit_max_connections(max_connections);
            let (router, addr) = setup(store, events).await?;
            for (path, hash) in hashes {
                let ticket = BlobTicket::new(addr.clone(), hash, BlobFormat::Raw);
                println!("{}: {ticket}", path.display());
            }
            tokio::signal::ctrl_c().await?;
            router.shutdown().await?;
        }
    }
    endpoint.close().await;
    Ok(())
}

async fn add_paths(store: &MemStore, paths: Vec<PathBuf>) -> Result<HashMap<PathBuf, Hash>> {
    let mut hashes = HashMap::new();
    for path in paths {
        let tag = store.add_path(&path).await?;
        hashes.insert(path, tag.hash);
    }
    Ok(hashes)
}

async fn setup(store: MemStore, events: EventSender) -> Result<(Router, EndpointAddr)> {
    let secret = get_or_generate_secret_key()?;
    let endpoint = iroh::Endpoint::builder(presets::N0)
        .secret_key(secret)
        .bind()
        .await?;
    endpoint.online().await;
    let addr = endpoint.addr();
    let blobs = BlobsProtocol::new(&store, Some(events));
    let router = Router::builder(endpoint)
        .accept(iroh_blobs::ALPN, blobs)
        .spawn();
    Ok((router, addr))
}
