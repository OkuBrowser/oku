# Serde Bencode

[![Checking](https://github.com/toby/serde-bencode/actions/workflows/checking.yml/badge.svg)](https://github.com/toby/serde-bencode/actions/workflows/checking.yml) [![Formatting](https://github.com/toby/serde-bencode/actions/workflows/formatting.yml/badge.svg)](https://github.com/toby/serde-bencode/actions/workflows/formatting.yml) [![Testing](https://github.com/toby/serde-bencode/actions/workflows/testing.yaml/badge.svg)](https://github.com/toby/serde-bencode/actions/workflows/testing.yaml) [![Benchmarking](https://github.com/toby/serde-bencode/actions/workflows/benchmarking.yml/badge.svg)](https://github.com/toby/serde-bencode/actions/workflows/benchmarking.yml) [![Coverage](https://github.com/toby/serde-bencode/actions/workflows/coverage.yaml/badge.svg)](https://github.com/toby/serde-bencode/actions/workflows/coverage.yaml) [![Crates.io](https://img.shields.io/crates/v/serde_bencode)](https://crates.io/crates/serde_bencode) [![docs.rs](https://img.shields.io/docsrs/serde_bencode)](https://docs.rs/serde_bencode)

A [Serde](https://github.com/serde-rs/serde) backed [Bencode](https://en.wikipedia.org/wiki/Bencode)
encoding/decoding library for Rust.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
serde_bencode = "^0.2.4"
serde = "^1.0.0"
serde_derive = "^1.0.0"
```

## Usage

This is an abbreviated `.torrent` parsing example from [examples/parse_torrent.rs](examples/parse_torrent.rs). If you compile this crate as a binary, it will print metadata for any Torrent sent to stdin.

## Benchmarking

```console
cargo bench
```
