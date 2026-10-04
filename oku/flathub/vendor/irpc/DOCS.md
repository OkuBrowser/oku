Building docs for this crate is a bit complex. There are lots of feature flags,
so we want feature flag markers in the docs, especially for the transports.

There is an experimental cargo doc feature that adds feature flag markers. To
get those, run docs with this command line:

```rust
RUSTDOCFLAGS="--cfg quicrpc_docsrs" cargo +nightly doc --all-features --no-deps --open
```

This sets the flag `quicrpc_docsrs` when creating docs, which triggers statements
like below that add feature flag markers. Note that you *need* nightly for this feature
as of now.

```
#[cfg_attr(quicrpc_docsrs, doc(cfg(feature = "flume-transport")))]
```

The feature is *enabled* using this statement in lib.rs:

```
#![cfg_attr(quicrpc_docsrs, feature(doc_cfg))]
```

We tell [docs.rs] to use the `quicrpc_docsrs` config using these statements
in Cargo.toml:

```
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "quicrpc_docsrs"]
```