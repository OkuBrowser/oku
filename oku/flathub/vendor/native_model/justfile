set shell := ["nu", "-c"]

default:
    @just --list --unsorted;

build_no_default:
    cargo build --no-default-features

build_default:
    cargo build

build_serde:
    cargo build --no-default-features --features serde

build_bincode_1_3:
    cargo build --features bincode_1_3

build_no_default_bincode_1_3:
    cargo build --no-default-features --features serde --features bincode_1_3

build_bincode_2_rc:
    cargo build --features bincode_2_rc

build_no_default_bincode_2_rc:
    cargo build --no-default-features --features serde --features bincode_2_rc

build_postcard_1_0:
    cargo build --features postcard_1_0

build_no_default_postcard_1_0:
    cargo build --no-default-features --features serde --features postcard_1_0

build_all: build_no_default build_default build_serde build_bincode_1_3 build_no_default_bincode_1_3 build_bincode_2_rc build_no_default_bincode_2_rc build_postcard_1_0 build_no_default_postcard_1_0

_tests_crate args='':
    cd tests_crate; \
    cargo test {{args}}

test_no_default:
    @just _tests_crate '--no-default-features'

test_default:
    @just _tests_crate args=''

test_bincode_1_3:
    @just _tests_crate '--features bincode_1_3'

test_bincode_2_rc:
    @just _tests_crate '--features bincode_2_rc'

test_postcard_1_0:
    @just _tests_crate '--features postcard_1_0'

test_docs:
    cargo test --doc --features bincode_2_rc

test_all: test_docs test_no_default test_default test_bincode_1_3 test_bincode_2_rc test_postcard_1_0

bench_overhead:
    cargo bench --bench overhead

bench_all: bench_overhead