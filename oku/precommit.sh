#!/bin/sh
cargo clippy --fix --allow-dirty --features="browser"
# __CARGO_FIX_YOLO=1 cargo clippy --fix --broken-code --allow-dirty --features="browser"
cargo fmt
cargo check
cargo check --features="browser"