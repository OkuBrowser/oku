#!/usr/bin/bash

# fail fast
#
set -e

# print each command before it's executed
#
set -x

export RUSTFLAGS="-D warnings --cfg getrandom_backend=\"wasm_js\""

cargo clean
cargo +nightly clippy --tests --examples --benches --all-features --target wasm32-unknown-unknown -- -D warnings

