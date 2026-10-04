IPLD DAG-PB codec
=================

[![Crates.io](https://img.shields.io/crates/v/ipld-dagpb.svg)](https://crates.io/crates/ipld-dagpb)
[![Documentation](https://docs.rs/ipld-dagpb/badge.svg)](https://docs.rs/ipld-dagpb)

This is an implementation of the [IPLD] [DAG-PB] codec. It can be use in conjunction with [ipld-core].

The code is based on [libipld-pb] and was imported with its full history.

DAG-PB is a special IPLD codec in the sense, that it does *not* implement the full [IPLD Data Model]. Therefore it cannot be used easily for structured data (as opposed to the [Serde] based codecs [serde_ipld_dagcbor] and [serde_ipld_dagjson]). It's only possible to encode and decode the data into/from an IPLD object of a [specific shape].


[IPLD]: https://ipld.io/
[DAG-PB]: https://ipld.io/specs/codecs/dag-pb/spec/
[ipld-core]: https://crates.io/crates/ipld-core
[libipld-pb]: https://crates.io/crates/libipld-pb
[IPLD Data Model]: https://ipld.io/docs/data-model/
[Serde]: https://github.com/serde-rs/serde
[serde_ipld_dagcbor]: https://crates.io/crates/serde_ipld_dagcbor
[serde_ipld_dagjson]: https://crates.io/crates/serde_ipld_dagjson
[specific shape]: https://ipld.io/specs/codecs/dag-pb/spec/#logical-format


License
-------

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
