# iroh-mainline-address-lookup

Pkarr-based address lookup for [iroh](https://github.com/n0-computer/iroh),
backed by the [BitTorrent Mainline DHT](https://en.wikipedia.org/wiki/Mainline_DHT).

This crate publishes and resolves iroh endpoint addresses via [pkarr]
records stored on the Mainline DHT. Pkarr ([Public-Key Addressable
Resource Records][pkarr]) lets a node publish DNS Resource Records
under a name derived from its public key, signed by the corresponding
secret key.

[pkarr]: https://pkarr.org

## License

Copyright 2025 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
