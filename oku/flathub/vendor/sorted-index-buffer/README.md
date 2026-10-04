# SortedIndexBuffer

This crate provides a data structure with identical behaviour to a `BTreeMap<u64, T>`,
but optimized for the case where keys are mostly consecutive.

It has no dependencies and should work in all environments.

Tests are comparing against `BTreeMap<u64, T>` using [proptest].

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

[proptest]: https://docs.rs/proptest/latest/proptest/