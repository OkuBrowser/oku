use iroh::{endpoint::presets, protocol::Router, Endpoint};
use iroh_gossip::{net::Gossip, ALPN};
use n0_error::{Result, StdResultExt};

#[tokio::main]
async fn main() -> Result<()> {
    // create an iroh endpoint that includes the standard address lookup mechanisms
    // we've built at number0
    let endpoint = Endpoint::bind(presets::N0).await?;

    // build gossip protocol
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // setup router
    let router = Router::builder(endpoint.clone())
        .accept(ALPN, gossip.clone())
        .spawn();
    // do fun stuff with the gossip protocol
    router.shutdown().await.std_context("shutdown router")?;
    Ok(())
}
