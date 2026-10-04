# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`system-deps` is a Rust crate for declaring system library dependencies declaratively in `Cargo.toml` metadata (under `[package.metadata.system-deps]`) instead of programmatically in `build.rs`. It wraps `pkg-config` to probe for system libraries. MSRV is 1.78.0, edition 2018.

## Build & Development Commands

```sh
cargo check          # Type-check
cargo build          # Build
cargo test           # Run all tests
cargo test <name>    # Run a single test by name
cargo fmt            # Format code
cargo clippy         # Lint (CI uses -D warnings)
```

## Architecture

The crate has three source files:

- **`src/lib.rs`** — Public API and core logic. Key types:
  - `Config` — Builder that reads Cargo.toml metadata and probes pkg-config. Entry point: `Config::new().probe()`.
  - `Dependencies` — Collection of resolved libraries (uses `BTreeMap` for deterministic ordering).
  - `Library` — A single resolved dependency with link paths, libs, frameworks, include paths, defines, etc.
  - `Error` — Comprehensive error enum with proper source chains.

- **`src/metadata.rs`** — Parses `[package.metadata.system-deps]` TOML tables. Handles simple version strings, complex table specs (name overrides, feature-gating, fallback names, `cfg()` expressions). Uses `toml` crate's low-level API with spanned values for positional error reporting.

- **`src/test.rs`** — Integration tests using fixed TOML manifests in `src/tests/toml-*/` directories. Each test directory contains a `Cargo.toml` and mock `.pc` files. Tests use `lazy_static` mutex + mock `EnvVariables` to isolate environment state.

## Testing Conventions

- Test manifests live in `src/tests/toml-*/` — each is a minimal Cargo.toml with specific metadata scenarios.
- The `EnvVariables` enum abstracts environment access; tests use the mock variant to avoid race conditions.
- Helper functions: `create_config()`, `toml()`, `assert_flags()`.

## Environment Variable Override System

Users can override pkg-config results via `SYSTEM_DEPS_$NAME_*` env vars (where `$NAME` is SHOUTY_SNAKE_CASE):
- `_SEARCH_NATIVE`, `_SEARCH_FRAMEWORK` — link search paths
- `_LIB` — libraries to link
- `_INCLUDE` — include paths
- `_LINK=static` — force static linking
- `_NO_PKG_CONFIG=1` — skip pkg-config probing
