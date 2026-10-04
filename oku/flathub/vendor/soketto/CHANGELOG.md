# Changelog

The format is based on [Keep a Changelog].

[Keep a Changelog]: http://keepachangelog.com/en/1.0.0/

## 0.8.1

- [fixed] ignore I/O error after successful close handshake [#115](https://github.com/paritytech/soketto/pull/115)

## 0.8.0

- [changed] move to rust 2021 [#56](https://github.com/paritytech/soketto/pull/56)
- [changed] Replace sha-1 v0.9 with sha1 v0.10 [#62](https://github.com/paritytech/soketto/pull/62)
- [changed] Update hyper requirement from v0.14 to v1.0 [#99](https://github.com/paritytech/soketto/pull/99)
- [changed] Update base64 requirement from 0.13 to 0.22 [#97](https://github.com/paritytech/soketto/pull/97)
- [changed] Bump MSRV to 1.71.1.
- [fixed] doc typo on Client resource field [#79](https://github.com/paritytech/soketto/pull/97)

## 0.7.1

- [fixed] Advance reader when a too big message is received [#54](https://github.com/paritytech/soketto/pull/54)

## 0.7.0

- [added] Added the `handshake::http` module and example usage at `examples/hyper_server.rs` to make using Soketto in conjunction with libraries that use the `http` types (like Hyper) simpler [#45](https://github.com/paritytech/soketto/pull/45) [#48](https://github.com/paritytech/soketto/pull/48)
- [added] Allow setting custom headers on the client to be sent to WebSocket servers when the opening handshake is performed [#47](https://github.com/paritytech/soketto/pull/47)

## 0.6.0

- [changed] Expose the `Origin` headers from the client handshake on `ClientRequest` [#35](https://github.com/paritytech/soketto/pull/35)
- [changed] Update handshake error to expose a couple of new variants (`IncompleteHttpRequest` and `SecWebSocketKeyInvalidLength`) [#35](https://github.com/paritytech/soketto/pull/35)
- [added] Add `send_text_owned` method to `Sender` as an optimisation when you can pass an owned `String` in [#36](https://github.com/paritytech/soketto/pull/36)
- [updated] Run rustfmt over the repository, and minor tidy up [#41](https://github.com/paritytech/soketto/pull/41)

## 0.5.0

- Update examples to Tokio 1 [#27](https://github.com/paritytech/soketto/pull/27)
- Update deps and remove unnecessary transients [#30](https://github.com/paritytech/soketto/pull/30)
- Add CLOSE reason handling [#31](https://github.com/paritytech/soketto/pull/31)
- Fix handshake with case-sensible servers [#32](https://github.com/paritytech/soketto/pull/32)

## 0.4.2

- Added connection ID to log output (#21).
- Added `ClientRequest::path` to access the path requested by the client
  (See #23 by @mward for details).
- Updated `sha-1` dependency to 0.9 (#24).

## 0.4.1

- Update some `dev-dependencies`.

## 0.4.0

- Remove all `unsafe` code blocks.
- Remove internal use of `futures::io::BufWriter`.
- `Extension::decode` now takes a `&mut Vec<u8>` instead of a `BytesMut`.
- `Incoming::Pong` contains the PONG payload data slice inline.
- `Data` not longer contains application data, but reports only the number
  of bytes. The actual data is written directly into the `&mut Vec<u8>`
  parameter of `Receiver::receive` or `Receiver::receive_data`.
- `Receiver::into_stream` has been removed.

## 0.3.2

- Bugfix release. `Codec::encode_header` contained a hidden assumption that
  a `usize` would be 8 bytes long, which is obviously only true on 64-bit
  architectures. See #18 for details.

## 0.3.1

- A method `into_inner` to get back the socket has been added to
  `handshake::{Client, Server}`.

## 0.3.0

Update to use and work with async/await:

- `Connection` has been split into a `Sender` and `Receiver` pair with
  async methods to send and receive data or control frames such as Pings
  or Pongs.
- `connection::into_stream` has been added to get a `futures::stream::Stream`
  from a `Receiver`.
- A `connection::Builder` has been added to setup connection parameters.
  `handshake::Client` and `handshake::Server` no longer have an
  `into_connection` method, but an `into_builder` one which returns the
  `Builder` and allows further configuration.
- `base::Data` has been moved to `data`. In addition `data::Incoming`
  supports control frame data.
- `base::Codec` no longer implements `Encoder`/`Decoder` traits but has
  inherent methods for encoding and decoding websocket frame headers.
- `base::Frame` has been removed. The `base::Codec` only deals with
  headers.
- The `handshake` module contains separate sub-modules for `client` and
  `server` handshakes. Some handshake related types have been refactored
  slightly.
- `Extension`s `decode` methods work on `&mut BytesMut` parameters
  instead of `Data`. For `encode` a new type `Storage` has been added
  which unifies different types of data, i.e. shared, unique and owned data.

## 0.2.3

- Maintenance release.

## 0.2.2

- Improved handshake header matching which is now more robust and can cope with
  repeated header names and comma separated values.

## 0.2.1

- The DEFLATE extension now allows custom maximum window bits for client and server.
- Fix handling of reserved bits in base codec.

## 0.2.0

- Change `Extension` trait and add an optional DEFLATE extension (RFC 7692).
  For now the possibility to use reserved opcodes in extensions is not enabled.
  The DEFLATE extension does not support setting of window bits other than 15
  currently.
- Limit the max. buffer size in `Connection` (see `Connection::set_max_buffer_size`).

## 0.1.0

Initial release.
