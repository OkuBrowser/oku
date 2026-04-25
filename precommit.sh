#!/bin/sh
cargo clippy -p oku-core --fix --allow-dirty --features="cli,fuse"
cargo clippy -p oku --fix --allow-dirty
# __CARGO_FIX_YOLO=1 cargo clippy --fix --broken-code --allow-dirty --features="cli,fuse"
cargo fmt --all
cargo check -p oku-core
cargo check -p oku-core --features="cli"
cargo check -p oku-core --features="fuse"
cargo check -p oku-core --features="cli,fuse"
cargo check -p oku