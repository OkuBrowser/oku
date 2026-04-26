#!/bin/sh
cargo clippy --fix --allow-dirty --features="cli,fuse"
# __CARGO_FIX_YOLO=1 cargo clippy --fix --broken-code --allow-dirty --features="cli,fuse"
cargo fmt
cargo check
cargo check --features="cli"
cargo check --features="fuse"
cargo check --features="persistent"
cargo check --features="cli,fuse,persistent"
cargo test
cargo test --features="cli"
cargo test --features="fuse"
cargo test --features="persistent"
cargo test --features="cli,fuse,persistent"