# iroh-docs

[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/iroh-docs/)
[![Crates.io](https://img.shields.io/crates/v/iroh-docs.svg?style=flat-square)](https://crates.io/crates/iroh-docs)
[![downloads](https://img.shields.io/crates/d/iroh-docs.svg?style=flat-square)](https://crates.io/crates/iroh-docs)
[![Chat](https://img.shields.io/discord/1161119546170687619?logo=discord&style=flat-square)](https://discord.com/invite/DpmJgtU7cW)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square)](LICENSE-APACHE)
[![CI](https://img.shields.io/github/actions/workflow/status/n0-computer/iroh-docs/ci.yaml?branch=main&style=flat-square&label=CI)](https://github.com/n0-computer/iroh-docs/actions/workflows/ci.yaml)

Multi-dimensional key-value documents with an efficient synchronization protocol.

The crate operates on *Replicas*. A replica contains an unlimited number of
*Entries*. Each entry is identified by a key, its author, and the replica's
namespace. Its value is the 32-byte BLAKE3 hash of the entry's content data,
the size of this content data, and a timestamp.
The content data itself is not stored or transferred through a replica.

All entries in a replica are signed with two keypairs:

* The *Namespace* key, as a token of write capability. The public key is the *NamespaceId*, which
  also serves as the unique identifier for a replica.
* The *Author* key, as a proof of authorship. Any number of authors may be created, and
  their semantic meaning is application-specific. The public key of an author is the [AuthorId].

Replicas can be synchronized between peers by exchanging messages. The synchronization algorithm
is based on a technique called *range-based set reconciliation*, based on [this paper][paper] by
Aljoscha Meyer:

> Range-based set reconciliation is a simple approach to efficiently compute the union of two
sets over a network, based on recursively partitioning the sets and comparing fingerprints of
the partitions to probabilistically detect whether a partition requires further work.

The crate exposes a generic storage interface with in-memory and persistent, file-based
implementations. The latter makes use of [`redb`], an embedded key-value store, and persists
the whole store with all replicas to a single file.

[paper]: https://arxiv.org/abs/2212.13567

# Getting Started

The entry into the `iroh-docs` protocol is the `Docs` struct, which uses an [`Engine`](https://docs.rs/iroh-docs/latest/iroh_docs/engine/struct.Engine.html) to power the protocol.

`Docs` was designed to be used in conjunction with `iroh`. [Iroh](https://docs.rs/iroh) is a networking library for making direct connections, these connections are peers send sync messages and transfer data.

Iroh provides a [`Router`](https://docs.rs/iroh/latest/iroh/protocol/struct.Router.html) that takes an [`Endpoint`](https://docs.rs/iroh/latest/iroh/endpoint/struct.Endpoint.html) and any protocols needed for the application. Similar to a router in webserver library, it runs a loop accepting incoming connections and routes them to the specific protocol handler, based on `ALPN`.

`Docs` is a "meta protocol" that relies on the [`iroh-blobs`](https://docs.rs/iroh-blobs) and [`iroh-gossip`](https://docs.rs/iroh-gossip) protocols. Setting up `Docs` will require setting up `Blobs` and `Gossip` as well.

Here is a basic example of how to set up `iroh-docs` with `iroh`:

```rust
use iroh::{endpoint::presets, protocol::Router, Endpoint};
use iroh_blobs::{BlobsProtocol, store::mem::MemStore, ALPN as BLOBS_ALPN};
use iroh_docs::{protocol::Docs, ALPN as DOCS_ALPN};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create an iroh endpoint that includes the standard discovery mechanisms
    // we've built at number0
    let endpoint = Endpoint::bind(presets::N0).await?;

    // build the blobs protocol
    let blobs = MemStore::default();

    // build the gossip protocol
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // build the docs protocol
    let docs = Docs::memory()
        .spawn(endpoint.clone(), (*blobs).clone(), gossip.clone())
        .await?;

    // create a router builder, we will add the
    // protocols to this builder and then spawn
    // the router
    let builder = Router::builder(endpoint.clone());

    // setup router
    let _router = builder
        .accept(BLOBS_ALPN, BlobsProtocol::new(&blobs, None))
        .accept(GOSSIP_ALPN, gossip)
        .accept(DOCS_ALPN, docs)
        .spawn();

    // do fun stuff with docs!
    Ok(())
}
```

# License

Copyright 2026 N0, INC.

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
