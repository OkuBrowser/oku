# proc-macro-error

[![travis ci](https://travis-ci.org/CreepySkeleton/proc-macro-error.svg?branch=master)](https://travis-ci.org/CreepySkeleton/proc-macro-error)
[![docs.rs](https://docs.rs/proc-macro-error/badge.svg)](https://docs.rs/proc-macro-error)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

This crate aims to make error reporting in proc-macros simple and easy to use.
Migrate from `panic!`-based errors for as little effort as possible!

Also, there's ability to [append a dummy token stream][crate::dummy] to your errors.

```toml
[dependencies]
proc-macro-error = "0.4"
```

*Supports rustc 1.31 and up*

[Documentation and guide][guide]

## What emitted errors look like

```
error: multiple error part: multi2

  = note: help message test
  = help: Option help test
  = note: I see what you did here...

 --> $DIR/multi-error.rs:4:18
  |
4 | make_fn!(multi1, multi2, _, multi3);
  |                  ^^^^^^
```

## Examples

### Panic-like usage

```rust
use proc_macro_error::*;
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
use quote::quote;

// This is your main entry point
#[proc_macro]
// this attribute *MUST* be placed on top of the #[proc_macro] function
#[proc_macro_error]
pub fn make_answer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    if let Err(err) = some_logic(&input) {
        // we've got a span to blame, let's use it
        // This immediately aborts the proc-macro and shows the error
        //
        // You can use `proc_macro::Span`, `proc_macro2::Span`, and
        // anything that implements `quote::ToTokens` (almost every type from
        // `syn` and `proc_macro2`)
        abort!(err, "You made an error, go fix it: {}", err.msg);
    }

    // `Result` has some handy shortcuts if your error type implements
    // `Into<MacroError>`. `Option` has one unconditionally.
    more_logic(&input).expect_or_abort("What a careless user, behave!");

    if !more_logic_for_logic_god(&input) {
        // We don't have an exact location this time,
        // so just highlight the proc-macro invocation itself
        abort_call_site!(
            "Bad, bad user! Now go stand in the corner and think about what you did!");
    }

    // Now all the processing is done, return `proc_macro::TokenStream`
    quote!(/* stuff */).into()
}
```

### `proc_macro::Diagnostic`-like usage

```rust
use proc_macro_error::*;
use proc_macro::TokenStream;
use syn::{spanned::Spanned, DeriveInput, ItemStruct, Fields, Attribute , parse_macro_input};
use quote::quote;

fn process_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter_map(|attr| match process_attr(attr) {
            Ok(res) => Some(res),
            Err(msg) => {
                emit_error!(attr, "Invalid attribute: {}", msg);
                None
            }
        })
        .collect()
}

fn process_fields(_attrs: &Fields) -> Vec<TokenStream> {
    // processing fields in pretty much the same way as attributes
    unimplemented!()
}

#[proc_macro]
#[proc_macro_error]
pub fn make_answer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let attrs = process_attrs(&input.attrs);

    // abort right now if some errors were encountered
    // at the attributes processing stage
    abort_if_dirty();

    let fields = process_fields(&input.fields);

    // no need to think about emitted errors
    // #[proc_macro_error] will handle them for you
    //
    // just return a TokenStream as you normally would
    quote!(/* stuff */).into()
}
```

## Limitations

- Warnings are emitted only on nightly, they are ignored on stable.
- "help" suggestions can't have their own span info on stable,
  (essentially inheriting the parent span).
- If a panic occurs somewhere in your macro no errors will be displayed. This is not a
  technical limitation but rather intentional design. `panic` is not for error reporting.
- Temporary incompatible with `proc_macro_hack`, unfortunately. No worries, some highly
  trained people are working on it!

## MSRV policy

`proc_macro_error` will always be compatible with proc-macro Holy Trinity:
`proc_macro2`, `syn`, `quote` crates. In other words, if the Trinity is available
to you - `proc_macro_error` is available too.

> **Important!**
>
> If you want to use `#[proc_macro_error]` with `synstructure`, you're going
> to have to put the attribute inside the `decl_derive!` invocation. Unfortunately,
> due to some bug in pre-1.34 rustc, putting proc-macro attributes inside macro
> invocations doesn't work, so your MSRV is effectively 1.34 in cases like that.

## Motivation

Error handling in proc-macros sucks. There's not much of a choice today:
you either "bubble up" the error up to the top-level of the macro and convert it to
a [`compile_error!`][compl_err] invocation or just use a good old panic. Both these ways suck:

- Former sucks because it's quite redundant to unroll a proper error handling
    just for critical errors that will crash the macro anyway; so people mostly
    choose not to bother with it at all and use panic. Simple `.expect` is too tempting.

    Also, if you do decide to implement this `Result`-based architecture in your macro
    you're going to have to rewrite it entirely once [`proc_macro::Diagnostic`][] is finally
    stable. Not cool.

- Later sucks because there's no way to carry out the span info via `panic!`.
    `rustc` will highlight the invocation itself but not some specific token inside it.

    Furthermore, panics aren't for error-reporting at all; panics are for bug-detecting
    (like unwrapping on `None` or out-of-range indexing) or for early development stages
    when you need a prototype ASAP so error handling can wait. Mixing these usages only
    messes things up.

- There is [`proc_macro::Diagnostic`][] which is awesome but it has been experimental
    for more than a year and is unlikely to be stabilized any time soon.

    This crate's API is intentionally designed to be compatible with `proc_macro::Diagnostic`
    and delegates to it whenever possible. Once `Diagnostics` is stable this crate
    will **always** delegate to it, no code changes will be required on user side.

That said, we need a solution, but this solution must meet these conditions:

- It must be better than `panic!`. The main point: it must offer a way to carry span information
    over to user.
- It must take as little effort as possible to migrate from `panic!`. Ideally, a new
    macro with the same semantics plus ability to carry out span info.
- It must maintain compatibility with [`proc_macro::Diagnostic`][] .
- **It must be usable on stable**.

This crate aims to provide such a mechanism. All you have to do is annotate your top-level
`#[proc_macro]` function with `#[proc_macro_errors]` attribute and change panics to
[`abort!`]/[`abort_call_site!`] where appropriate, see [the Guide][guide].

## Disclaimer
Please note that **this crate is not intended to be used in any way other
than proc-macro error reporting**, use `Result` and `?` for anything else.

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>


[compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
[`proc_macro::Diagnostic`]: https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html

[crate::dummy]: https://docs.rs/proc-macro-error/0.4/proc_macro_error/dummy/index.html
[crate::multi]: https://docs.rs/proc-macro-error/0.4/proc_macro_error/multi/index.html

[`abort_call_site!`]: https://docs.rs/proc-macro-error/0.4/proc_macro_error/macro.abort_call_site.html
[`abort!`]: https://docs.rs/proc-macro-error/0.4/proc_macro_error/macro.abort.html
[guide]: https://docs.rs/proc-macro-error
