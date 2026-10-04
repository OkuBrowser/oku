# n0-error

[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/n0-error/)
[![Crates.io](https://img.shields.io/crates/v/n0-error.svg?style=flat-square)](https://crates.io/crates/n0-error)
[![downloads](https://img.shields.io/crates/d/n0-error.svg?style=flat-square)](https://crates.io/crates/n0-error)
[![Chat](https://img.shields.io/discord/1161119546170687619?logo=discord&style=flat-square)](https://discord.com/invite/DpmJgtU7cW)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square)](LICENSE-APACHE)
[![CI](https://img.shields.io/github/actions/workflow/status/n0-computer/n0-error/ci.yaml?branch=main&style=flat-square&label=CI)](https://github.com/n0-computer/n0-error/actions/workflows/ci.yaml)

An Rust error library that supports tracking the call-site location of errors.

This crate provides a `StackError` trait and proc macro to ergonomically work with nested
enum and struct errors, while allowing to track the call-site location for the
full error chain.

It also has a `AnyError` type that works similar to anyhow errors while keeping
the location metadata of `StackError` errors accessible through the full error chain.

## Example

```rust
use n0_error::{e, ensure, Result, StackResultExt, stack_error};

/// The `stack_error` macro controls how to turn our enum into a `StackError`.
///
/// * `add_meta` adds a field to all variants to track the call-site error location
/// * `derive` adds `#[derive(StackError)]`
/// * `from_sources` creates `From` impls for the error sources
#[stack_error(derive, add_meta, from_sources)]
enum MyError {
    /// We can define the error message with the `error` attribute
    #[error("invalid input")]
    InvalidInput { source: InvalidInput },
    /// Or we can define a variant as `transparent`, which forwards the Display impl to the error source.
    #[error(transparent)]
    Io {
        /// For sources that do not implement `StackError`, we have to mark the source as `std_err`.
        #[error(std_err)]
        source: std::io::Error,
    },
}

/// We can use the [`stack_error`] macro on structs as well.
#[stack_error(derive, add_meta)]
#[error("wanted {expected} but got {actual}")]
struct InvalidInput {
    expected: u32,
    actual: u32,
}

fn validate_input(input: u32) -> Result<(), InvalidInput> {
    if input != 23 {
        // With the `e` macro we can construct an error directly without spelling out the `meta` field:
        return Err(e!(InvalidInput { expected: 12, actual: input }))
    }
    /// There's also `bail` and `ensure` macros that expand to include the `meta` field:
    n0_error::ensure!(input == 23, InvalidInput { expected: 23, actual: input });
    Ok(())
}

/// The `Result` type defaults to `AnyError` for the error variant.
///
/// Errors types using the derive macro convert to `AnyError`, and we can add additional context
/// with the result extensions.
fn process(input: u32) -> Result<()> {
    validate_input(input).context("failed to process input")?;
    Ok(())
}
```

The error returned from `process` would look like this in the alternate display format:
```text
failed to process input: invalid input: wanted 23 but got 13
```
and like this in the debug format with `RUST_BACKTRACE=1` or `RUST_ERROR_LOCATION=1`:
```text
failed to process input (examples/basic.rs:61:17)
Caused by:
    invalid input (examples/basic.rs:36:5)
    wanted 23 but got 13 (examples/basic.rs:48:13)
```

### Details

- All errors using the macro implement the `StackError` trait, which exposes call-site metadata for
  where the error occurred. Its `source` method returns references which may be other stack errors,
  allowing access to location data for the entire error chain.
- The proc macro can add a `meta` field to structs or enum variants. This field stores call-site
  metadata accessed through the `StackError` trait.
    * Call-site metadata in the `meta` field is collected only if `RUST_BACKTRACE=1` or
     `RUST_ERROR_LOCATION=1` env variable is set. Otherwise, it is disabled to avoid runtime overhead.
    * The declarative macro `e!` provides an ergonomic way to construct such errors without
      explicitly setting the field. The crate's `ensure` and `bail` macro also do this.
- The crate includes an `AnyError` type, similar to `anyhow::Error`. When created from a
  `StackError`, the call-site metadata is preserved. `AnyError` is recommended for applications and
  tests, while libraries should use concrete derived errors.
    * All stack errors convert to `AnyError`, so they can be propagated to such results with `?`.
    * For std errors, use `std_context` or `anyerr` to convert to `AnyError`. For stack errors, use
      `context` or simply propagate with `?`.
    * Result extension traits provide conversions between results with `StackError`s,
      `std::error::Error`s to `AnyError`, with support for attaching context.
- Both `AnyError` and all errors using the `StackError` derive feature consistent, structured output
  that includes location metadata when available.

### Feature flags

* `anyhow` (off by default): Enables `From<anyhow::Error> for AnyError` and `impl StackError for anyhow::Error`

## License

Copyright 2026 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
