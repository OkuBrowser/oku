# iroh-metrics

[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/iroh-metrics/)
[![Crates.io](https://img.shields.io/crates/v/iroh-metrics.svg?style=flat-square)](https://crates.io/crates/iroh-metrics)
[![downloads](https://img.shields.io/crates/d/iroh-metrics.svg?style=flat-square)](https://crates.io/crates/iroh-metrics)
[![Chat](https://img.shields.io/discord/1161119546170687619?logo=discord&style=flat-square)](https://discord.com/invite/DpmJgtU7cW)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square)](LICENSE-APACHE)
[![CI](https://img.shields.io/github/actions/workflow/status/n0-computer/iroh-metrics/ci.yaml?branch=main&style=flat-square&label=CI)](https://github.com/n0-computer/iroh-metrics/actions/workflows/ci.yaml)

Metrics collection for iroh. It keeps the "glue" necessary to expose metrics defined metrics for collection: an HTTP server that exposes tracked metrics that a service like prometheus can poll for recording.

Nearly all metrics in iroh are recorded as simple counters.

# License

Copyright 2026 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
