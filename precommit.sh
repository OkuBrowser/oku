#!/bin/sh
cargo clippy --fix --bin "oku" --allow-dirty
cargo fmt