# pollable-map

> Collections of futures and streams that can be polled as a single stream, along with utilities.

[![Crates.io](https://img.shields.io/crates/v/pollable-map.svg)](https://crates.io/crates/pollable-map)
[![Documentation](https://docs.rs/pollable-map/badge.svg)](https://docs.rs/pollable-map)
![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)

## Description

`pollable-map` provides keyed collections of futures and streams that can be
polled as a single stream, along with related asynchronous utilities.

## Overview

- `FutureMap` polls a collection of futures assigned to a specific key.
- `StreamMap` polls a collection of streams assigned to a specific key.
- `FutureSet` polls futures without assigning user-provided keys.
- `StreamSet` combines streams without exposing internal keys.
- `OrderedFutureSet` polls futures one at a time in FIFO order.
- `Optional<T>` is similar to `Option<T>` but is a reusable future or stream
  that can be empty and later receive another task.
- Timeout map, set, and optional wrappers are available with the `timeout` feature.

Maps and optional tasks remain reusable after becoming empty. Set
`FutureMap::set_terminate_on_empty` or `StreamMap::set_terminate_on_empty` to true to make it behave as a
terminated stream after all current entries complete.

## Examples

### FutureMap example

```rust
use futures::future::BoxFuture;
use futures::StreamExt;
use pollable_map::futures::FutureMap;

fn main() {
    futures::executor::block_on(async {
        let mut fut_map = FutureMap::<u8, BoxFuture<'static, ()>>::new();

        fut_map.insert(
            1,
            Box::pin(async {
                println!("message from 1");
            }),
        );
        fut_map.insert(
            2,
            Box::pin(async {
                println!("message from 2");
            }),
        );

        while let Some((key, _)) = fut_map.next().await {
            println!("> future \"{}\" is finished", key);
        }

        println!("Done processing map")
    });
}

```

### StreamMap example

```rust
use futures::stream::BoxStream;
use futures::StreamExt;
use pollable_map::stream::StreamMap;

fn main() {
    futures::executor::block_on(async {
        let mut st_map = StreamMap::<u8, BoxStream<'static, ()>>::new();

        st_map.insert(
            1,
            futures::stream::once(async {
                println!("message from 1");
            })
            .boxed(),
        );
        st_map.insert(
            2,
            futures::stream::once(async {
                println!("message from 2");
            })
            .boxed(),
        );

        while let Some((key, _)) = st_map.next().await {
            println!("> stream \"{}\" is finished", key);
        }

        println!("Done processing map")
    });
}
```

## License

Dual licensed under the
[MIT License](https://github.com/dariusc93/pollable-map/blob/main/LICENSE-MIT)
or the
[Apache License 2.0](https://github.com/dariusc93/pollable-map/blob/main/LICENSE-APACHE).
You may choose either license.
