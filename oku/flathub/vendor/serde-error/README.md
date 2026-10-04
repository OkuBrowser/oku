# serde-error

`serde-error` provides a (de)serializable `Error` type implementing
`std::error::Error`, that can easily be used to transmit errors over
the wire.

## Should I use this?

This crate is production-grade. However, you probably do not want to
use it: usually, it makes much more sense to just sum up the error as
some type, instead of serializing the whole causality chain.

The use case for which this crate was designed is running Rust
WebAssembly blobs inside a Rust wasmtime-running host. In such a case
the causality chain is clearly kept across the serialization boundary,
and it thus makes sense to keep it all.

In some other cases it may make sense to serialize the whole causality
chain, but most often it makes most sense to just not serialize
errors.

As such, please use `serde-error` with parsimony.

## How should I use this?

```rust
use anyhow::Context;
use std::error::Error;

fn foo() -> anyhow::Result<()> {
    // ...
    Err(anyhow::anyhow!("Failed smurfing the smurfs"))
}

fn bar() -> anyhow::Result<()> {
    // ...
    foo().context("Running foo")
}

fn main() {
    if let Err(returned_err) = bar() {
        let s = bincode::serialize(&serde_error::Error::new(&*returned_err))
            .expect("Serializing error");
        let d: serde_error::Error = bincode::deserialize(&s)
            .expect("Deserializing error");
        let e = anyhow::Error::from(d);
        assert_eq!(e.to_string(), "Running foo");
        assert_eq!(e.source().unwrap().to_string(), "Failed smurfing the smurfs");
    } else {
        panic!("bar did not return an error");
    }
}
```
