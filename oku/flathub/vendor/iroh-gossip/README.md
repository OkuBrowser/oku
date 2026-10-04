# iroh-gossip

This crate implements the `iroh-gossip` protocol.
It is based on *epidemic broadcast trees* to disseminate messages among a swarm of peers interested in a *topic*.
The implementation is based on the papers [HyParView](https://asc.di.fct.unl.pt/~jleitao/pdf/dsn07-leitao.pdf) and [PlumTree](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf).

The crate is made up from two modules:
The `proto` module is the protocol implementation, as a state machine without any IO.
The `net` module implements networking logic for running `iroh-gossip` on `iroh` connections.

The `net` module is optional behind the `net` feature flag (enabled by default).

# Getting Started

The `iroh-gossip` protocol was designed to be used in conjunction with `iroh`. [Iroh](https://docs.rs/iroh) is a networking library for making direct connections, these connections are how gossip messages are sent.

Iroh provides a [`Router`](https://docs.rs/iroh/latest/iroh/protocol/struct.Router.html) that takes an [`Endpoint`](https://docs.rs/iroh/latest/iroh/endpoint/struct.Endpoint.html) and any protocols needed for the application. Similar to a router in webserver library, it runs a loop accepting incoming connections and routes them to the specific protocol handler, based on `ALPN`.

Here is a basic example of how to set up `iroh-gossip` with `iroh`:
```rust,no_run
use iroh::{protocol::Router, Endpoint, EndpointId, endpoint::presets};
use iroh_gossip::{api::Event, Gossip, TopicId};
use n0_error::{Result, StdResultExt};
use n0_future::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // create an iroh endpoint that includes the standard discovery mechanisms
    // we've built at number0
    let endpoint = Endpoint::bind(presets::N0).await?;

    // build gossip protocol
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // setup router
    let router = Router::builder(endpoint)
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn();

    // gossip swarms are centered around a shared "topic id", which is a 32 byte identifier
    let topic_id = TopicId::from_bytes([23u8; 32]);
    // and you need some bootstrap peers to join the swarm
    let bootstrap_peers = bootstrap_peers();

    // then, you can subscribe to the topic and join your initial peers
    let (sender, mut receiver) = gossip
        .subscribe(topic_id, bootstrap_peers)
        .await?
        .split();

    // you might want to wait until you joined at least one other peer:
    receiver.joined().await?;

    // then, you can broadcast messages to all other peers!
    sender.broadcast(b"hello world this is a gossip message".to_vec().into()).await?;

    // and read messages from others!
    while let Some(event) = receiver.next().await {
        match event? {
            Event::Received(message) => {
                println!("received a message: {:?}", std::str::from_utf8(&message.content));
            }
            _ => {}
        }
    }

    // clean shutdown makes sure that other peers are notified that you went offline
    router.shutdown().await.std_context("shutdown router")?;
    Ok(())
}

fn bootstrap_peers() -> Vec<EndpointId> {
    // insert your bootstrap peers here, or get them from your environment
    vec![]
}
```

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
