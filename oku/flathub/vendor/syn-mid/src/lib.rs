/*!
<!-- tidy:crate-doc:start -->
Providing the features between "full" and "derive" of syn.

This crate provides the following two unique data structures.

- [`syn_mid::ItemFn`] -- A function whose body is not parsed.

  ```text
  fn process(n: usize) -> Result<()> { ... }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ ^     ^
  ```

- [`syn_mid::Block`] -- A block whose body is not parsed.

  ```text
  { ... }
  ^     ^
  ```

Other data structures are the same as data structures of [syn]. These are
defined in this crate because they cannot be used in [syn] without "full"
feature.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
syn-mid = "0.5"
```

*Compiler support: requires rustc 1.56+*

[**Examples**](https://github.com/taiki-e/syn-mid/tree/HEAD/examples)

## Optional features

- **`clone-impls`** — Clone impls for all syntax tree types.

[syn]: https://github.com/dtolnay/syn

<!-- tidy:crate-doc:end -->
*/

#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_variables)
    )
))]
#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(
    clippy::pedantic,
    // lints for public library
    clippy::alloc_instead_of_core,
    // clippy::exhaustive_enums, // TODO
    // clippy::exhaustive_structs, // TODO
    // clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
)]

// Many of the code contained in this crate are copies from https://github.com/dtolnay/syn.

#[cfg(doc)]
extern crate self as syn_mid;

#[macro_use]
mod macros;

mod func;
mod pat;
mod path;

pub use crate::{
    func::{Block, FnArg, ItemFn, Receiver, Signature},
    pat::{
        FieldPat, Pat, PatIdent, PatPath, PatReference, PatStruct, PatTuple, PatTupleStruct,
        PatType, PatWild,
    },
};
