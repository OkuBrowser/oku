#!/usr/bin/bash

# fail fast
#
set -e

# print each command before it's executed
#
set -x

export RUSTFLAGS="-D warnings"

git clone --depth 1 https://github.com/najamelan/ws_stream_tungstenite
cd ws_stream_tungstenite

# Need to override the wasm target set in .cargo/config.toml
export TARGET=$(rustc -vV | sed -n 's|host: ||p');
echo $TARGET;

cargo build --example echo --release --target $TARGET
cargo build --example echo_tt --release --target $TARGET

cargo run --example echo --release --target $TARGET &
cargo run --example echo_tt --release --target $TARGET -- "127.0.0.1:3312"  &
