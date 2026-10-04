# async-rt

## Overview

async-rt is a small library designed to utilize async executors (i.e Tokio) through a common API while extending functionality
for abortable task, tasks that receive messages and being able to switch between multiple async runtimes at compile time if specific
conditions are met (i.e tokio or compio if the feature is enabled and its a non-wasm32 arch, while wasm32-bindgen-futures is used if the arch is wasm32).

## Runtime attributes

The `macros` feature provides `main` and `test` attributes. It is enabled by default.

```rust
#[async_rt::main]
async fn main() {
    let task = async_rt::task::spawn(async { 42 });
    assert_eq!(task.await.unwrap(), 42);
}
```

A runtime can also be selected explicitly:

```rust,no_run
#[async_rt::main(executor = "compio")]
async fn main() {
    let task = async_rt::task::spawn(async { 42 });
    assert_eq!(task.await.unwrap(), 42);
}
```

The `lite` feature provides a built-in lightweight executor and is enabled by default:

```rust,no_run
#[async_rt::main(executor = "lite")]
async fn main() {
    let task = async_rt::task::spawn(async { 42 });
    assert_eq!(task.await.unwrap(), 42);
}
```

A custom executor can drive the annotated future through the `driver` option:

```rust,no_run
#[async_rt::main(driver = application::executor())]
async fn main() {
    run_application().await;
}
```

The driver expression must produce an executor that implements `ExecutorBlockOn`. A custom
driver does not change the built-in executor used by `async_rt::task`.

Supported selections are `"global"`, `"tokio"`, `"smol"`, `"compio"`, `"threadpool"`, and `"lite"`.
Explicit selection controls the runtime driving the annotated function as it does not
change the compile-time `DefaultExecutor`. This distinction matters when more than one
executor feature is enabled. Note that these attributes do not currently support 
wasm32 targets.

## MSRV

The minimum supported rust version is 1.95, which can be changed in the future. There is no guarantee that this library will work on older versions of rust.

## License

This crate is licensed under either Apache 2.0 or MIT.
