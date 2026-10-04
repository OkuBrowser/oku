# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.10.1] - 2026-09-06

### Changed
- Update `base64` to v0.23.1 (#196).
- Update `quick-xml` to v0.42.0 (#196).
- Update `indexmap` to v2.14.0 (#196).
- Update to Rust 2024 edition (#198).
- Run clippy in CI (#200).

### Fixed
- Don't panic when binary plist dates are out of range (#199).

## [1.10.0] - 2026-07-04

### Changed
- Add `plist!` (#184) and `plist_dict!` (#189) macros.
- Update `quick-xml` to v0.41.0 to fix RUSTSEC-2026-0194 (#190).

### Fixed
- Read binary plists with 24-bit references (#188).

## [1.9.0] - 2026-04-26

### Changed
- Bump MSRV to v1.88 as required by `time` v0.3.47.
- Update `quick-xml` to v0.39.2 (#176).
- Update `time` to v0.3.47 to fix RUSTSEC-2026-0009 (#177).

## [1.8.0] - 2025-09-15

### Changed
- Bump MSRV to v1.81 as required by `time-core` v0.1.6.
- Don't publish test data to crates.io (#164).
- Publish using crates.io Trusted Publishing.

### Fixed
- Read binary plists with 24-bit integer offset tables (#165).

## [1.7.4] - 2025-07-07

### Changed
- Update `quick-xml` to v0.38.
- Run `cargo-semver-checks` on pull requests.

## [1.7.3] - 2025-07-05

### Fixed
- Fail deserialisation if input is not completely consumed (#149).

## [1.7.2] - 2025-06-11

### Changed
- Update `quick-xml` to v0.37.

[unreleased]: https://github.com/ebarnard/rust-plist/compare/v1.10.1...HEAD
[1.10.0]: https://github.com/ebarnard/rust-plist/compare/v1.10.0...v1.10.1
[1.10.0]: https://github.com/ebarnard/rust-plist/compare/v1.9.0...v1.10.0
[1.9.0]: https://github.com/ebarnard/rust-plist/compare/v1.8.0...v1.9.0
[1.8.0]: https://github.com/ebarnard/rust-plist/compare/v1.7.4...v1.8.0
[1.7.4]: https://github.com/ebarnard/rust-plist/compare/v1.7.3...v1.7.4
[1.7.3]: https://github.com/ebarnard/rust-plist/compare/v1.7.2...v1.7.3
[1.7.2]: https://github.com/ebarnard/rust-plist/compare/v1.7.1...v1.7.2
