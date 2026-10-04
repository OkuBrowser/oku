# Changelog

All notable changes to mainline dht will be documented in this file.

## Unpublished 

## Added

- `AsyncDht::as_sync()` to get a reference to the inner synchronous `Dht`.

## [6.3.0](https://github.com/Nuhvi/mainline/compare/v6.0.0...v6.1.0) - 2026-02-26

### Added

- `DhtBuilder::build_async()`, `Dht::new_async()`, and `AsyncDht::new()` to avoid blocking on initial dns queries in async runtimes.

## [6.2.0](https://github.com/Nuhvi/mainline/compare/v6.0.0...v6.1.0) - 2025-01-25

### Fixed

- Unexpected shutdown in actor thread.
- Remove panics and unwraps.
- Fix peers number in signed_peers.

### Refactor

- Large refactor for better maintainability and sans-io Core.

## [6.1.0](https://github.com/Nuhvi/mainline/compare/v6.0.0...v6.1.0) - 2025-10-28

### Fixed

- Respond to `FIND_NODE` with nodes supporting `signing_peers` first.
- In `server_mode`, always add nodes from incoming requests to `signing_peers_routing_table` if they support it.
- When querying `FIND_NODE` find closest nodes from all tables (include `signing_peers_routing_table`).

### Added

- `Info::routing_table_size()` and `Info::signed_peers_routing_table_size()`

## [6.0.0](https://github.com/Nuhvi/mainline/compare/v5.5.1...v6.0.0) - 2025-10-27

### Added

- Internal `request_timeout` estimation based on observed RTTs.
- `announce_signed_peer` and `get_signed_peers` as described at [BEP_????](https://github.com/Nuhvi/mainline/blob/main/beps/bep_signed_peers.rst).

### Removed

- `DhtBuilder::request_timeout()` (Breaking change).
- `node` Feature flag, now the `Dht` is always available
- `RoutingTable::closest_secure()` made private.

### Changed

- Upgrade `getrandom` to `v0.3`
- Upgrade `ed25519-dalek` to `v3.0.0-pre.1` (Breaking change)
- `MutableItem::new()` takes a reference to the signer.
- `DhtBuilder::bootstrap()` and `DhtBuilder::extra_bootstrap()` takes a `ToString`.

### Fixed

- Adaptive request timeout 
- Adaptive interval of polling udp socket for new messages, reducing latency and cpu usage.
- Accept both u16 or u32 transaction ids
- `DhtBuilder::extra_bootstrap()` was removing existing defaults

## [5.4.0](https://github.com/nuhvi/mainline/compare/v5.3.1...v5.4.0 ) - 2025-04-02

### Added

- Add `Testnet::new_async()`
- Populate the bootstrapping node's routing table with requesting nodes
- Update client version from RS04 to RS05

## [5.3.1](https://github.com/nuhvi/mainline/compare/v5.3.0...v5.3.1 ) - 2025-02-21

### Fixed

- Delay DNS lookups for default bootstrap to `Rpc::new()`

## [5.3.0] - 2025-02-20

### Changed

- Replace `rand` with `getrandom`

## [5.2.1] - 2025-02-17

### Fixed

- DHT size estimate from repetitive GET queries (PR #50)

## [5.2.0] - 2025-02-11

### Changed

- Update dev-dependencies

## [5.1.0] - 2025-02-11

### Changed

- Use Rust 2021 edition

## [5.0.0] - 2025-02-11

### Added

- Add `Id::from_ipv4()`.
- Add `Id::is_valid_for_ipv4`.
- Add `RoutingTable::nodes()` iterator.
- Add `DhtBuilder::server_mode` to force server mode.
- Add `DhtBuilder::public_ip` for manually setting the node's public ip to generate secure node `Id` from.
- Add [adaptive mode](https://github.com/nuhvi/mainline?tab=readme-ov-file#adaptive-mode).
- Add `DhtBuilder::extra_bootstrap()` to add more bootstrapping nodes from previous sessions.
- Add `Dht::bootstrapped()` and `AsyncDht::bootstrapped()` to wait for the routing table to be bootstrapped.
- Add `RoutingTable::to_bootstrap()`, `Dht::to_bootstrap()`, and `AsyncDht::to_bootstrap()` to export the addresses nodes in the routing table.
- Add `Info::public_address()` which returns the best estimate for this node's public address.
- Add `Info::firewalled()` which returns whether or not this node is firewalled, or publicly accessible.
- Add `Info::server_mode()` which returns whether or not this node is running in server mode.
- Add `DhtBuilder::info()` to export a thread safe and lightweight summary of the node's information and statistics.
- Add `cache_bootstrap.rs` example to show how you can store your routing table to disk and use it for subsequent bootstrapping.
- Add `Dht::get_mutable_most_recent()` and `AsyncDht::get_mutable_most_recent()` to get the most recent mutable item from the network.
- Add `PutQueryError::Timeout` in case put query is terminated unsuccessfully, but no error responses.
- Add `PutMutableError::Concurrrency(ConcurrrencyError)` for all cases where a `Lost Update Problem` may occur (read `Dht::put_mutable` documentation for more details).
- Add `Dht::get_closest_nodes()` and `AsyncDht::get_closest_nodes()` to return the closest nodes (that support BEP_0044) with valid tokens.
- Add `Dht::put()` and `AsyncDht::put()` to put a request to the closest nodes, and optionally to extra arbitrary nodes with valid tokens.
- Add `Testnet::leak()` to keep the dht network running as a `'static`
- Add `MutableError `.
- Add `DecodeIdError`
- Export `Dhtbuilder`.
- Export `RoutingTable`.
- Support `BEP_0042 DHT Security extension` when running in server mode. 

### Removed

- Remove `bytes` dependency.
- Remove `ipv6` optionality and commit to `ipv4`.
- Remove `Id::to_vec()`.
- Exported `ClosestNodes`, you have to use it from `mainline::rpc`.
- Removed `Node::unique()`, `Node::with_id()`, `Node::with_address()`, and `Node::with_token()`.
- Removed `RoutingTable::default()`.
- Removed exporting `rpc` module, and `Rpc` struct.
- Removed `Dht::shutdown()` and `AsyncDht::shutdown()`.
- Removed `DhtWasShutdown`
- Removed `DefaultServer` export.

### Changed

- Rename `Settings` to `ClientBuilder`.
- `Dht`, and `AsyncDht` is now behind a feature flag `node`, so you can include the `Rpc` only and build your own node.
- All methods that were returning `Result<T, DhtWasShutdown>` now return `T`.
- Enable calling `Dht::announce_peer()` and `Dht::put_immutable()` multiple times concurrently. 
- Return `PutMutableError::Concurrrency(ConcurrrencyError)` from `Dht::put_mutable()`.
- `Info::local_addr()` is infallible.
- `MutableItem::seq()` returns `i64` instead of a reference.
- `Dht::put_immutable()` and `AsyncDh::put_immutable()` take `&[u8]` instead of `bytes::Bytes`.
- `Dht::get_immutable()` and `AsyncDh::get_immutable()` return boxed slice `Box<[u8]>` instead of `bytes::Bytes`.
- `Dht::put_immutable()` and `AsyncDh::put_immutable()` return `PutImmutableError`.
- `Dht::announce_peer()` and `AsyncDh::announce_peer()` return `AnnouncePeerError`.
- `Dht::put_mutable()` and `AsyncDh::put_mutable()` return `PutMutableError`.
- All tracing logs are either `TRACE` (for krpcsocket), `DEBUG`, or `INFO` only for rare and singular events, 
  like starting the node, updating the node Id, or switching to server mode (from adaptive mode).
- Change `PutError` to contain transparent elements for generic `PutQueryError`, and more specialized `ConcurrrencyError`.
- Remove `MutableItem::cas` field, and add optional `CAS` parameter to `Dht::put_mutable` and `AsyncDht::put_mutable`.
- `Dht::find_node()` and `AsyncDht::find_node()` return `Box<[Node]>` instead of `Vec<Node>`.
- `Node` is `Send` and `Sync`, and cheap to clone using an internal `Arc`.
- `Node::new()` take `Id` and `SocketAddrV4`.
- `RoutingTable::new()` takes an `Id`.
- Return `GetIterator<T>` and `GetStream<T>` from `get_` methods from `Dht` and `AsyncDht` instead of exposing `flume`.
- Remove `Server` trait and replace it with `RequestFilter` trait.
- `DhtBuilder` is not consuming, thanks to `Config` being `Clone`.

## [4.2.0](https://github.com/nuhvi/mainline/compare/v4.1.0...v4.2.0) - 2024-12-13

### Added

- Make MutableItem de/serializable (mikedilger)

## [4.1.0](https://github.com/nuhvi/mainline/compare/v3.0.0...v4.1.0) - 2024-11-29

### Added

- Export `errors` module containing `PutError` as a part of the response of `Rpc::put`.
- `Dht::find_node()` and `AsyncDht::find_node()` to find the closest nodes to a certain target.
- `Dht::info()` and `AsyncDht::info()` some internal information about the node from one method.
- `Info::dht_size_estimate` to get the ongoing dht size estimate resulting from watching results of all queries.
- `Info::id` to get the Id of the node.
- `measure_dht` example to estimate the DHT size.

### Changed

- Removed all internal panic `#![deny(clippy::unwrap_used)]`.
- `Testnet::new(size)` returns a `Result<Testnet>`.
- `Dht::local_addr()` and `AsyncDht::local_addr()` replaced with `::info()`.
- `Dht::shutdown()` and `AsyncDht::shutdown()` are now idempotent, and returns `()`.
- `Rpc::drop` uses `tracing::debug!()` to log dropping the Rpc.
- `Id::as_bytes()` instead of exposing internal `bytes` property.
- Replace crate `Error` with more granular errors.
- Replace Flume's `RecvError` with `expect()` message, since the sender should never be dropped to soon.
- `DhtWasShutdown` error is a standalone error.
- `InvalidIdSize` error is a standalone error.
- Rename `DhtSettings` to `Settings`
- Rename `DhtServer` to `DefaultServer`
- `Dht::get_immutable()` and `AsyncDht::get_immutable()` return `Result<Option<bytes::Bytes>, DhtWasShutdown>`
- `Node` fields are now all private, with `id()` and `address()` getters.
- Changed `Settings` to be a the Builder, and make fields private.
- Replaced `Rpc::new()` with `Settings::build_rpc()`.
- Update the client version from `RS01` to `RS04`

### Removed

- Removed `mainline::error::Error` and `mainline::error::Result`.
