# other-error

Lightweight no-std compatible wrapper types for sharing (via `Arc` or `Rc`) and combining errors.

## Usage
```rust,ignore
use other_error::{ArcError, EitherError};
use either::Either;

// Make a non-Clone error cheaply cloneable and shareable.
let err = ArcError::new(std::io::Error::other("boom"));
let also_err = err.clone(); // shares the same allocation

// Represent a value that may fail in one of two ways.
let either: EitherError<std::io::Error, std::fmt::Error> =
    Either::Left(std::io::Error::other("boom")).into();
println!("{either}");
```

## MSRV

The minimum supported rust version is 1.81, which can be changed in the future. There is no guarantee that this library will work on older versions of rust.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
