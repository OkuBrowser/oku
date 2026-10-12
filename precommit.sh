#!/bin/sh
cargo clippy -p oku-core --fix --allow-dirty --features="cli,fuse,persistent"
cargo clippy -p oku --fix --allow-dirty --features="browser"
# __CARGO_FIX_YOLO=1 cargo clippy -p oku-core --fix --broken-code --allow-dirty --features="cli,fuse,persistent"
# __CARGO_FIX_YOLO=1 cargo clippy -p oku --fix --broken-code --allow-dirty --features="browser"
cargo fmt --all
cargo check -p oku-core
cargo check -p oku-core --features="cli"
cargo check -p oku-core --features="fuse"
cargo check -p oku-core --features="persistent"
cargo check -p oku-core --features="cli,fuse,persistent"
cargo test -p oku-core
cargo test -p oku-core --features="cli"
cargo test -p oku-core --features="fuse"
cargo test -p oku-core --features="persistent"
cargo test -p oku-core --features="cli,fuse,persistent"
cargo check -p oku
cargo check -p oku --features="browser"