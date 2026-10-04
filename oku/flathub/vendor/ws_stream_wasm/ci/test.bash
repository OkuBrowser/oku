#!/usr/bin/bash

# fail fast
#
set -e

# print each command before it's executed
#
set -x

wasm-pack test  --firefox --headless -- --features "tokio_io" --no-default-features
wasm-pack test  --chrome  --headless -- --features "tokio_io" --no-default-features
wasm-pack test  --firefox --headless -- --features "tokio_io" --no-default-features --release
wasm-pack test  --chrome  --headless -- --features "tokio_io" --no-default-features --release
