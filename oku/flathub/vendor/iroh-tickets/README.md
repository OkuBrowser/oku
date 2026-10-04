# iroh-tickets

[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/iroh-tickets/)
[![Crates.io](https://img.shields.io/crates/v/iroh-tickets.svg?style=flat-square)](https://crates.io/crates/iroh-tickets)
[![downloads](https://img.shields.io/crates/d/iroh-tickets.svg?style=flat-square)](https://crates.io/crates/iroh-tickets)
[![Chat](https://img.shields.io/discord/1161119546170687619?logo=discord&style=flat-square)](https://discord.com/invite/DpmJgtU7cW)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square)](LICENSE-APACHE)
[![CI](https://img.shields.io/github/actions/workflow/status/n0-computer/iroh-tickets/ci.yaml?branch=main&style=flat-square&label=CI)](https://github.com/n0-computer/iroh-tickets/actions/workflows/ci.yaml)

> Simple ticket system used for signaling with [iroh](https://github.com/n0-computer/iroh).

A ticket bundles the information needed to reach an iroh endpoint into a single
serializable value. Tickets are expected to round-trip to and from their canonical string form
(lowercase kind prefix + base32) as well as to and from their byte form, via the
[`Ticket`](https://docs.rs/iroh-tickets/latest/iroh_tickets/trait.Ticket.html) trait.

## Example

```rust
use std::str::FromStr;

use iroh_base::{EndpointAddr, PublicKey, TransportAddr};
use iroh_tickets::{Ticket, endpoint::EndpointTicket};

let pk = PublicKey::from_str(
    "ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6",
)
.unwrap();
let addr = EndpointAddr::from_parts(
    pk,
    [TransportAddr::Ip("127.0.0.1:1234".parse().unwrap())],
);
let ticket = EndpointTicket::new(addr);

// Encode to the canonical string form (lowercase KIND prefix + base32).
let encoded = ticket.encode_string();
assert!(encoded.starts_with("endpoint"));

// Decode back via `FromStr` (which delegates to `Ticket::decode_string`).
let decoded: EndpointTicket = encoded.parse().unwrap();
assert_eq!(ticket, decoded);
```

## License

Copyright 2026 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
