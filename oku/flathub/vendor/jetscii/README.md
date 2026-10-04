# Jetscii

A tiny library to efficiently search strings for sets of ASCII
characters or byte slices for sets of bytes.

[![Current Version](https://img.shields.io/crates/v/jetscii.svg)](https://crates.io/crates/jetscii) [![Documentation](https://docs.rs/jetscii/badge.svg)][docs]

## Examples

### Searching for a set of ASCII characters

```rust
#[macro_use]
extern crate jetscii;

fn main() {
    let part_number = "86-J52:rev1";
    let first = ascii_chars!('-', ':').find(part_number);
    assert_eq!(first, Some(2));
}
```

### Searching for a set of bytes

```rust
#[macro_use]
extern crate jetscii;

fn main() {
    let raw_data = [0x00, 0x01, 0x10, 0xFF, 0x42];
    let first = bytes!(0x01, 0x10).find(&raw_data);
    assert_eq!(first, Some(1));
}
```

Check out [the documentation][docs] for information about feature flags and benchmarks.

[docs]: https://docs.rs/jetscii/

## Contributing

1. Fork it (https://github.com/shepmaster/jetscii/fork)
2. Create your feature branch (`git checkout -b my-new-feature`)
3. Add a failing test.
4. Add code to pass the test.
5. Commit your changes (`git commit -am 'Add some feature'`)
6. Ensure tests pass.
7. Push to the branch (`git push origin my-new-feature`)
8. Create a new Pull Request
