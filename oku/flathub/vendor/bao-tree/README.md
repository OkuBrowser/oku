# bao-tree

[![Actions Status](https://github.com/n0-computer/bao-tree/workflows/tests/badge.svg)](https://github.com/n0-computer/bao-tree/actions) [![docs.rs](https://docs.rs/bao-tree/badge.svg)](https://docs.rs/bao-tree) [![crates.io](https://img.shields.io/crates/v/bao-tree.svg)](https://crates.io/crates/bao-tree)

The merkle tree used for BLAKE3 verified streaming.

This is a slightly different take on blake3 verified streaming than the 
[bao](https://github.com/oconnor663/bao) crate.

The network wire format for encoded data and slices is compatible with the bao
crate, except that this crate has builtin support for *runtime* configurable chunk
groups.

It also allows encoding not just single ranges but sets of non-overlapping ranges.
E.g. you can ask for bytes `[0..1000,5000..6000]` in a single query.

The intention is also to support both sync and async en/decoding out of the box
with maximum code sharing.

It allows to define both pre- and post order outboard formats as well as custom
outboard formats. Post order outboard formats have advantages for synchronizing
append only files.

For more detailed info, see the [docs](https://docs.rs/bao-tree/latest/bao_tree/)

## License

Copyright 2025 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
