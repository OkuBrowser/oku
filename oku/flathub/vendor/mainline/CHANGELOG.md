# Changelog

All notable changes to mainline dht will be documented in this file.

## [8.0.0](https://github.com/pubky/mainline/compare/v7.0.0...v8.0.0) - 2026-08-04

### Fixed

- Reject malformed mutable PUT requests that omit the required sequence number
  or signature, and reject immutable PUT requests that include mutable-only
  fields.
- Validate that a mutable item's target matches the SHA-1 hash derived from its
  public key and salt.

### Changed

- **BREAKING**: Add `MutableError::InvalidMutableTarget`; `MutableItem` no
  longer implements `From<PutMutableRequestArguments>`.

## [7.0.0](https://github.com/pubky/mainline/compare/v6.2.0...v7.0.0) - 2026-06-08

### Added

- Add `PutOutcome` to report the target and number of DHT nodes that acknowledged a successful PUT.
- Add `AsyncDht::get_mutable_detailed()` with `GetMutableOutcome` diagnostics for queried nodes, valid values, empty responses, invalid responses, KRPC errors, and timeouts.

### Changed

- **BREAKING**: `AsyncDht::put()` now returns `PutOutcome` instead of a raw `Id`.
- **BREAKING**: `AsyncDht::put_mutable()` now returns `PutOutcome` instead of a raw `Id`.
- Mutable GET queries no longer echo locally pending mutable PUT values; GET results now only come from DHT node responses.

### Deprecated

- Deprecate blocking `Dht` operation methods and `GetIterator`; use the async API via `Dht::as_async()` instead.

## [6.2.0](https://github.com/pubky/mainline/compare/v6.1.2...v6.2.0) - 2026-05-22

### Added

- feat: Expose bootstrap configuration by @andrei-21 in https://github.com/pubky/mainline/pull/99

### Fixed

- fix: Select mutable item by highest sequence by @andrei-21 in https://github.com/pubky/mainline/pull/100
- fix: Report mutable CAS majority rejection by @andrei-21 in https://github.com/pubky/mainline/pull/101

### Changed

- build: Disable default features for dependencies by @andrei-21 in https://github.com/pubky/mainline/pull/98

## [6.1.2](https://github.com/pubky/mainline/compare/v6.1.1...v6.1.2) - 2026-04-28

- fix: Typo in log message for invalid ID by @dezren39 in https://github.com/pubky/mainline/pull/96
- chore: Update digest dependency version to 0.11 by @emmyoh in https://github.com/pubky/mainline/pull/97
- chore: Update digest pin by @eminence in https://github.com/pubky/mainline/pull/93
- chore: Release flow + guide by @SeverinAlexB in https://github.com/pubky/mainline/pull/95

## [6.1.1](https://github.com/pubky/mainline/compare/v6.1.0...v6.1.1) - 2026-02-05

### Changed

- Update dependencies (#85).

## [6.1.0](https://github.com/pubky/mainline/compare/v6.0.1...v6.1.0) - 2026-02-05

### Added

- Add configurable bind address via `DhtBuilder::bind_addr()` (#82).

### Fixed

- Pin digest to 0.11.0-rc.9 to fix curve25519-dalek compilation (#81).
- Reduce peer discovery target_size from 20 to 10 to prevent MTU overflow (#79).
- Avoid eager DNS resolution of default bootstrap nodes (#78).

## [6.0.1](https://github.com/pubky/mainline/compare/v6.0.0...v6.0.1) - 2025-11-03

### Fixed

- Fix table refill flag - handle empty table after ping response processed (#75).

## [6.0.0](https://github.com/pubky/mainline/compare/v5.4.0...v6.0.0) - 2025-10-03

### Changed

- **BREAKING**: Update ed25519-dalek to 3.0.0-pre.1 (#71).
- Refactor: use regular blocking socket with `set_read_timeout` instead of non-blocking socket (#68, #72).
- Refactor: switch `Vec<InflightRequest>` for `BTreeMap` (#66).
- Expand transaction ID space (#65).

### Added

- Add cargo-deny for dependency auditing (#63).

### Fixed

- Fix feature gate of `Testnet` (#62).

## [5.4.0](https://github.com/pubky/mainline/compare/v5.3.1...v5.4.0) - 2025-04-02

### Added

- Add `Testnet::new_async()`.
- Populate the bootstrapping node's routing table with requesting nodes.
- Update client version.

## [5.3.1](https://github.com/pubky/mainline/compare/v5.3.0...v5.3.1) - 2025-02-21

### Fixed

- Delay DNS lookups for default bootstrap to `Rpc::new()`.

## [5.3.0](https://github.com/pubky/mainline/compare/v5.2.1...v5.3.0) - 2025-02-20

### Changed

- Replace `rand` with `getrandom` dependency.

## [5.2.1](https://github.com/pubky/mainline/compare/v5.2.0...v5.2.1) - 2025-02-17

### Fixed

- Fix DHT size estimate from repetitive GET queries.

## [5.2.0](https://github.com/pubky/mainline/compare/v5.1.0...v5.2.0) - 2025-02-12

### Changed

- Update dev-dependencies.

## [5.1.0](https://github.com/pubky/mainline/compare/v5.0.0...v5.1.0) - 2025-02-11

### Changed

- Use Rust 2021 edition.

## [5.0.0](https://github.com/pubky/mainline/compare/v4.2.1...v5.0.0) - 2025-02-11

### Added

- Add `Id::from_ipv4()`.
- Add `Id::is_valid_for_ipv4`.
- Add `RoutingTable::nodes()` iterator.
- Add `DhtBuilder::server_mode` to force server mode.
- Add `DhtBuilder::public_ip` for manually setting the node's public ip to generate secure node `Id` from.
- Add [adaptive mode](https://github.com/pubky/mainline?tab=readme-ov-file#adaptive-mode).
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

## [4.2.0](https://github.com/pubky/mainline/compare/v4.1.0...v4.2.0) - 2024-12-13

### Added

- Make MutableItem de/serializable (mikedilger)

## [4.1.0](https://github.com/pubky/mainline/compare/v3.0.0...v4.1.0) - 2024-11-29

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
