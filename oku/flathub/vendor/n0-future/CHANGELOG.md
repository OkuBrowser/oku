# Changelog

## [0.3.2](https://github.com/n0-computer/n0-future/compare/v0.3.1..0.3.2) - 2026-01-07

### ⛰️  Features

- Implement `JoinSet::poll_join_next*` and `JoinError::try_into_panic` ([#23](https://github.com/n0-computer/n0-future/issues/23)) - ([4ecea8f](https://github.com/n0-computer/n0-future/commit/4ecea8f83994d993d34aec9bc3173055fa63999c))

### ⚙️ Miscellaneous Tasks

- Bump derive_more ([#22](https://github.com/n0-computer/n0-future/issues/22)) - ([0464e04](https://github.com/n0-computer/n0-future/commit/0464e04cd86c68cd3643f6637a3e23e53b9c5418))

## [0.3.1](https://github.com/n0-computer/n0-future/compare/v0.3.0..v0.3.1) - 2025-11-11

### ⛰️  Features

- Add JoinSet::spawn_local ([#16](https://github.com/n0-computer/n0-future/issues/16)) - ([0704b54](https://github.com/n0-computer/n0-future/commit/0704b5474f5205042dc4fa23c1ca355dddd8434b))
- Add serde feature for web_time::SystemTime ([#17](https://github.com/n0-computer/n0-future/issues/17)) - ([f5ea6ed](https://github.com/n0-computer/n0-future/commit/f5ea6ede1f468658fda2cbd7764b6679c8d2898c))

### ⚙️ Miscellaneous Tasks

- Prepare 0.3.1 release ([#18](https://github.com/n0-computer/n0-future/issues/18)) - ([94455f2](https://github.com/n0-computer/n0-future/commit/94455f2250136c0fb71a4d37083b1c30aa904448))

## [0.3.0](https://github.com/n0-computer/n0-future/compare/v0.2.0..v0.3.0) - 2025-10-20

### ⛰️  Features

- *(wasm)* Add `AbortHandle` and task ids ([#7](https://github.com/n0-computer/n0-future/issues/7)) - ([6a7062d](https://github.com/n0-computer/n0-future/commit/6a7062d4f08eb4f3ea37f894d7e9ee7ab3943532))
- Future::now_or_never - ([9bd31cb](https://github.com/n0-computer/n0-future/commit/9bd31cb684d727fb9ee320f6f1a264439ab463b0))

### ⚙️ Miscellaneous Tasks

- Fix ci tasks - ([fcc42c7](https://github.com/n0-computer/n0-future/commit/fcc42c77fefb739214c0368c6322c6bfa1658e62))

## [0.2.0](https://github.com/n0-computer/n0-future/compare/v0.1.3..v0.2.0) - 2025-07-28

### ⛰️  Features

- Add `MaybeFuture` ([#5](https://github.com/n0-computer/n0-future/issues/5)) - ([7e1226c](https://github.com/n0-computer/n0-future/commit/7e1226c170f894475716eabd055b5c9a6611b6c5))

### ⚙️ Miscellaneous Tasks

- Initial setup ([#6](https://github.com/n0-computer/n0-future/issues/6)) - ([eb25bd7](https://github.com/n0-computer/n0-future/commit/eb25bd7c7b62e2a682fcf9b5281f416997e0bd28))

### Deps

- Bump deps ([#4](https://github.com/n0-computer/n0-future/issues/4)) - ([e8dbee5](https://github.com/n0-computer/n0-future/commit/e8dbee5e739188ba27c7f646864a5ba81bad5bab))

## [0.1.3](https://github.com/n0-computer/n0-future/compare/v0.1.2..v0.1.3) - 2025-04-30

### ⚙️ Miscellaneous Tasks

- Fix minimal crate versions - ([179703c](https://github.com/n0-computer/n0-future/commit/179703c51ce34bb6ab4fc261c3dc812fb2df7f52))

## [0.1.2](https://github.com/n0-computer/n0-future/compare/v0.1.1..v0.1.2) - 2025-01-29

### ⛰️  Features

- Implement `std::error::Error` for error types - ([14eb141](https://github.com/n0-computer/n0-future/commit/14eb14166b67e405dc98c6cda2501e186d9b24b6))

## [0.1.1](https://github.com/n0-computer/n0-future/compare/v0.1.0..v0.1.1) - 2025-01-28

### ⛰️  Features

- Also expose `TryFutureExt` and `TryStreamExt` - ([fdbce1c](https://github.com/n0-computer/n0-future/commit/fdbce1c6c11947a2673b74f3d5ed83fb0cdf5fac))

## [0.1.0](https://github.com/n0-computer/n0-future/compare/v0.0.1..v0.1.0) - 2025-01-28

### ⛰️  Features

- Also re-export `ready!` and `pin!` macros - ([385cabf](https://github.com/n0-computer/n0-future/commit/385cabf47a55f9481cfb9e995a8fc338358e860a))

### 🚜 Refactor

- Remove `futures-sink` dependency in favor of `futures-util` import - ([0b10dda](https://github.com/n0-computer/n0-future/commit/0b10dda075eba3ffeaa670f4adb3f34a63b131fc))

### ⚙️ Miscellaneous Tasks

- Bump version from 0.0.1 to 0.1.0 - ([8525f26](https://github.com/n0-computer/n0-future/commit/8525f265073c67a1614678525b4ac11449277da1))

## [0.0.1] - 2025-01-27

### ⛰️  Features

- Add `futures_{lite,buffered,sink,util}` reexports - ([a566728](https://github.com/n0-computer/n0-future/commit/a566728beafdfc89ae1aa3e1039da48f31c08843))
- `boxed` module with `Send`/`!Send` variants and re-export `stream` - ([528e019](https://github.com/n0-computer/n0-future/commit/528e019311b95428c02e5ad1596784f89932c776))

### 🐛 Bug Fixes

- A maximum of 5 keywords is allowed - ([6d9c5fb](https://github.com/n0-computer/n0-future/commit/6d9c5fbe650c0fef51062f8a4424215e45d28433))

### 📚 Documentation

- Add LICENSE, README and cargo metadata - ([c4450a5](https://github.com/n0-computer/n0-future/commit/c4450a5c9f8303c858fa11c5ac55f82a18e53df1))


