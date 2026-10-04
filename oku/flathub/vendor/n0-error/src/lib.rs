//! A library for ergonomic errors with call-site location data
//!
//! This crate provides a [`StackError`] trait and [`stack_error`] proc macro to
//! ergonomically work with enum or struct errors.
//!
//! * All errors that use the macro will implement the [`StackError`] trait,
//!   which exposes call-site metadata indicating where the error occurred. Its
//!   [`source`](StackError::source) method returns an [`ErrorRef`], which is an
//!   enum over either a reference to a [`std::error::Error`] or another
//!   [`StackError`]. This allows retrieving error locations for the full error
//!   chain, as long as all errors are stack errors.
//!
//! * The proc macro can add a `meta` field to structs or enum variants. This
//!   field is the source for the call-site location accessed through the
//!   `StackError` trait. There is a simple declarative macro [`e!`](e) that
//!   provides an ergonomic way to construct errors with a `meta` field without
//!   having to spell it out everywhere.
//!
//! * This crate also provides an [`AnyError`] type, which is similar to
//!   [`anyhow`](https://docs.rs/anyhow/latest/anyhow/). If constructed from an
//!   error that implements `StackError`, the call-site location is preserved.
//!   The `AnyError` type is generally recommended for applications or tests,
//!   whereas libraries should use concrete errors with the macro.
//!
//! * There are result extensions to convert `StackError`s or `std::error::Error`s
//!   to `AnyError` while providing additional context.
//!
//! * While all errors using the derive macro from this crate have a `From`
//!   implementation for `AnyError`, regular std errors do not. Unfortunately, a
//!   blanket `From` implementation for all std errors would prevent a
//!   specialized implementation for stack errors, causing loss of location
//!   metadata upon conversion to `AnyError`. Therefore, you need to use the
//!   [`std_context`](StdResultExt::std_context) or
//!   [`anyerr`](StdResultExt::anyerr) methods to convert results with std errors
//!   to [`AnyError`]. You should not use these methods on stack errors; instead,
//!   use [`context`](StackResultExt::context) or simply forward with `?`.
//!
//! The call-site metadata in the `meta` field is collected only if the
//! environment variable `RUST_BACKTRACE=1` or `RUST_ERROR_LOCATION=1` is set.
//! Otherwise, it is not collected, as doing so has a small performance overhead.
//!
//! Both [`AnyError`] and all errors that use the
//! [derive macro](derive@StackError) feature the following outputs:
//!
//! * Display impl (`{error}`) prints only the message of the outermost error
//!   `failed to process input`
//! * Alternate display impl (`{error:#}`) prints the message for each error in the chain, in a single line
//!   `failed to process input: invalid input: wanted 23 but got 13`
//! * Debug impl (`{error:?}`) prints the message and each source, on separate lines.
//!   ```text
//!   failed to process input
//!   Caused by:
//!       invalid input
//!       wanted 23 but got 13
//!   ```
//!
//!   If `RUST_BACKTRACE` or `RUST_ERROR_LOCATION` is set, this will also print the call site of each error.
//!   ```text
//!   failed to process input (examples/basic.rs:61:17)
//!   Caused by:
//!        invalid input (examples/basic.rs:36:5)
//!        wanted 23 but got 13 (examples/basic.rs:48:13)
//!   ```
//! * Alternate debug impl  `{error:#?}`: An output similar to how the `#[derive(Debug)]` output looks.
//!
//! ### Feature flags
//!
//! * `anyhow` (off by default): Enables `From<anyhow::Error> for AnyError` and `impl StackError for anyhow::Error`
//!
//! ## Example
//!
//! ```rust
#![doc = include_str!("../examples/basic.rs")]
//! ```
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

pub use n0_error_macros::{StackError, stack_error};

mod any;
mod error;
mod ext;
mod macros;
mod meta;
#[cfg(test)]
mod tests;

pub use self::{any::*, error::*, ext::*, macros::*, meta::*};

/// `Result` type alias where the error type defaults to [`AnyError`].
pub type Result<T = (), E = AnyError> = std::result::Result<T, E>;

/// Returns a result with the error type set to [`AnyError`].
///
/// Equivalent to `Ok::<_, AnyError>(value)`.
#[allow(non_snake_case)]
pub fn Ok<T>(value: T) -> Result<T, AnyError> {
    std::result::Result::Ok(value)
}

/// Ensures we can use the macros within this crate as well.
extern crate self as n0_error;

/// Ensure the code in the README compiles
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_doctest {}
