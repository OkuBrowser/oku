#!/usr/bin/env bash
set -euo pipefail
set -x

run_pair() {
    echo
    echo "=== cargo build $* ==="
    cargo build "$@"
    echo "=== cargo test  $* ==="
    cargo test  "$@"
}

# 1) default (std)
run_pair

# 2) no_std
run_pair --no-default-features

# 3) alloc only
run_pair --no-default-features --features alloc

# 4) serde only
run_pair --no-default-features --features serde

# 5) alloc + serde
run_pair --no-default-features --features alloc,serde

# 6) all features
run_pair --all-features
