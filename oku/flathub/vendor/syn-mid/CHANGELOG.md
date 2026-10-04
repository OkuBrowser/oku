# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [Unreleased]

## [0.5.4] - 2023-06-29

- Increase the minimum supported Rust version from Rust 1.31 to Rust 1.56.

- Update minimal version of `proc-macro2` to 1.0.60.

## [0.5.3] - 2021-01-05

- Exclude unneeded files from crates.io.

## [0.5.2] - 2020-12-29

- Documentation improvements.

## [0.5.1] - 2020-10-14

- Implement `Parse` for `Signature`.

- Update parser implementations based on `syn` 1.0.44. This includes some bugfixes.

- Disable default features of `proc-macro2` and `quote`.

## [0.5.0] - 2019-12-09

- [Added `Signature` type.](https://github.com/taiki-e/syn-mid/pull/13)

## [0.4.0] - 2019-08-15

- Updated all data structures based on `syn` 1.0.

- Updated `proc-macro2`, `syn`, and `quote` to 1.0.

- Bumped the minimum required version from Rust 1.30 to Rust 1.31.

## [0.3.0] - 2019-02-18

- Removed support for unneeded syntax.

- Removed unneeded types and fields.

- Implemented `Parse` for `Block`.

- Changed `clone-impls` feature to "disabled by default".

- Removed `extra-traits` feature.

- Bumped the minimum required version from Rust 1.15 to Rust 1.30.

## [0.2.0] - 2019-02-15

- Reduced features.

- Fixed bugs.

## [0.1.0] - 2019-02-14

**Note:** This release has been yanked.

Initial release

[Unreleased]: https://github.com/taiki-e/syn-mid/compare/v0.5.4...HEAD
[0.5.4]: https://github.com/taiki-e/syn-mid/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/taiki-e/syn-mid/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/taiki-e/syn-mid/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/taiki-e/syn-mid/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/taiki-e/syn-mid/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/taiki-e/syn-mid/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/taiki-e/syn-mid/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/taiki-e/syn-mid/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/taiki-e/syn-mid/releases/tag/v0.1.0
