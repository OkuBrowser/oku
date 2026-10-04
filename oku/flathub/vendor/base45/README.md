# Base45

[![Test and publish Base45](https://github.com/opendevtools/base45/workflows/Release/badge.svg?branch=main)](https://github.com/opendevtools/base45/actions?query=workflow%3A%22Release%22)
[![Crate](https://img.shields.io/crates/v/base45.svg)](https://crates.io/crates/base45)
[![API](https://docs.rs/base45/badge.svg)](https://docs.rs/base45)

A encoder/decoder for base45 that is fully compatible with
[rfc9285](https://datatracker.ietf.org/doc/html/rfc9285). When encoding QR or Aztec codes, a scheme other than the standard base64, base32, and base16 is needed.

## Installation

```toml
[dependencies]
base45 = "1.0.1"
```

## Benchmarks

Benchmarks were created using [Criterion](https://github.com/bheisler/criterion.rs). The benchmarks test encoding and decoding of the string "The quick brown fox jumps over the lazy dog". Test computer is a MacBook Pro 14" (2021) M1 Pro with 32 GB RAM.

```
encode long string              time:   [107.51 ns 107.69 ns 107.91 ns]
encode long string from buffer  time:   [126.66 ns 126.84 ns 127.07 ns]
decode long string              time:   [115.58 ns 115.81 ns 116.08 ns]

encode long str 100 times       time:   [12.805 µs 12.821 µs 12.839 µs]
decode long str 100 times       time:   [12.407 µs 12.424 µs 12.445 µs]

encode 0x10 random bytes        time:   [57.717 ns 57.827 ns 57.946 ns]
encode 0x100 random bytes       time:   [292.00 ns 292.64 ns 293.50 ns]
encode 0x1000 random bytes      time:   [3.6926 µs 3.6962 µs 3.7001 µs]
encode 0x10000 random bytes     time:   [65.210 µs 65.476 µs 65.755 µs]
```
