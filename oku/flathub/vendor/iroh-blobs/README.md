# iroh-blobs

**NOTE: this version of iroh-blobs is not yet considered production quality. For now, if you need production quality, use iroh-blobs 0.35**

This crate provides blob and blob sequence transfer support for iroh. It implements a simple request-response protocol based on BLAKE3 verified streaming.

A request describes data in terms of BLAKE3 hashes and byte ranges. It is possible to request blobs or ranges of blobs, as well as entire sequences of blobs in one request.

The requester opens a QUIC stream to the provider and sends the request. The provider answers with the requested data, encoded as [BLAKE3](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf) verified streams, on the same QUIC stream.

This crate is used together with [iroh](https://crates.io/crates/iroh). Connection establishment is left up to the user or higher level APIs.

## Concepts

- **Blob:** a sequence of bytes of arbitrary size, without any metadata.

- **Link:** a 32 byte BLAKE3 hash of a blob.

- **HashSeq:** a blob that contains a sequence of links. Its size is a multiple of 32.

- **Provider:** The side that provides data and answers requests. Providers wait for incoming requests from Requests.

- **Requester:** The side that asks for data. It is initiating requests to one or many providers.

A node can be a provider and a requester at the same time.

## Getting started

The `iroh-blobs` protocol was designed to be used in conjunction with `iroh`. [Iroh](https://docs.rs/iroh) is a networking library for making direct connections, these connections are what power the data transfers in `iroh-blobs`.

Iroh provides a [`Router`](https://docs.rs/iroh/latest/iroh/protocol/struct.Router.html) that takes an [`Endpoint`](https://docs.rs/iroh/latest/iroh/endpoint/struct.Endpoint.html) and any protocols needed for the application. Similar to a router in webserver library, it runs a loop accepting incoming connections and routes them to the specific protocol handler, based on `ALPN`.

Here is a basic example of how to set up `iroh-blobs` with `iroh`:

```rust,no_run
use iroh::{protocol::Router, Endpoint, endpoint::presets};
use iroh_blobs::{store::mem::MemStore, BlobsProtocol, ticket::BlobTicket};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create an iroh endpoint that includes the standard address lookup mechanisms
    // we've built at number0
    let endpoint = Endpoint::bind(presets::N0).await?;

    // create a protocol handler using an in-memory blob store.
    let store = MemStore::new();
    let tag = store.add_slice(b"Hello world").await?;

    let _ = endpoint.online().await;
    let addr = endpoint.addr();
    let ticket = BlobTicket::new(addr, tag.hash, tag.format);

    // build the router
    let blobs = BlobsProtocol::new(&store, None);
    let router = Router::builder(endpoint)
        .accept(iroh_blobs::ALPN, blobs)
        .spawn();

    println!("We are now serving {}", ticket);

    // wait for control-c
    tokio::signal::ctrl_c().await;

    // clean shutdown of router and store
    router.shutdown().await?;
    Ok(())
}
```

## Examples

Examples that use `iroh-blobs` can be found in [this repo](https://github.com/n0-computer/iroh-blobs/tree/main/examples).

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
