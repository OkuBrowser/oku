#!/bin/sh
cargo clippy --fix --allow-dirty --features="cli,fuse"
# __CARGO_FIX_YOLO=1 cargo clippy --fix --broken-code --allow-dirty --features="cli,fuse"
cargo fmt
cargo check
cargo check --features="cli"
cargo check --features="fuse"
cargo check --features="cli,fuse"