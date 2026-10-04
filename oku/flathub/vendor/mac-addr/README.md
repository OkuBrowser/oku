[crates-badge]: https://img.shields.io/crates/v/mac-addr.svg
[crates-url]: https://crates.io/crates/mac-addr
[license-badge]: https://img.shields.io/crates/l/mac-addr.svg
[license-url]: https://github.com/shellrow/mac-addr/blob/main/LICENSE
[doc-url]: https://docs.rs/mac-addr/latest/mac-addr

# mac-addr [![Crates.io][crates-badge]][crates-url] [![License][license-badge]][license-url]
Lightweight, dependency-free, `no_std` compatible **MAC address** library  
supporting both **EUI-48** (`MacAddr`) and **EUI-64** (`MacAddr8`).

## Overview
It’s designed for portability and low-level use:
- **no_std** compatible  
- Optional `alloc` for string formatting  
- Optional `serde` support for serialization and deserialization  
- Small, efficient, and FFI-safe

## Usage
Add `mac-addr` to your dependencies  
```toml:Cargo.toml
[dependencies]
mac-addr = "0.3"
```

## Example

```rust
use mac_addr::{MacAddr, MacAddr8};

let mac6: MacAddr = "00:25:96:12:34:56".parse().unwrap();
assert_eq!(mac6.to_string(), "00:25:96:12:34:56");

// Convert EUI-48 -> EUI-64
let mac8 = MacAddr8::from_eui48(mac6);
assert_eq!(mac8.to_string(), "00:25:96:ff:fe:12:34:56");

// And back
assert_eq!(mac8.to_eui48().unwrap(), mac6);
```

For more details, see [doc][doc-url].  

## Features
| Feature | Default | Description |
|----------|----------|-------------|
| `std` | ✅ | Use Rust standard library |
| `alloc` | ⬜ | Enable heap allocation for `MacAddr::address()` |
| `serde` | ⬜ | Add `serde::Serialize` / `Deserialize` implementations |
