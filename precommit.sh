#!/bin/sh
cargo clippy --fix --allow-dirty
# __CARGO_FIX_YOLO=1 cargo clippy --fix --broken-code
cargo fmt
cargo check