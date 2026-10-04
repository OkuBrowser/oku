# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-06-25

### ⚠️ Breaking Changes

- **Prelude reorganization**: The single `easy_fuser::prelude` has been replaced by mode-specific preludes:
  - `easy_fuser::fuse_serial::prelude::*`
  - `easy_fuser::fuse_parallel::prelude::*`
  - `easy_fuser::fuse_async::prelude::*`
- **Module rename**: Template/preset implementations moved from `easy_fuser::templates` to `easy_fuser::fuse_presets`.
- **Preset design change**: Presets (`DefaultFuseHandler`, `MirrorFs`) no longer implement `FuseHandler` directly. Users now implement `FuseHandler` on their own struct and use the `delegate_fs!` macro to delegate operations.

### Migration Guide

```rust
// Before (v0.4.x)
use easy_fuser::prelude::*;
use easy_fuser::templates::DefaultFuseHandler;

struct MyFs;
impl FuseHandler for MyFs { /* ... */ }

// After (v0.5.0)
use easy_fuser::fuse_parallel::prelude::*; // or fuse_serial / fuse_async
use easy_fuser::fuse_presets::DefaultFuseHandler;
use easy_fuser_macro::delegate_fs;

struct MyFs {
    default_fs: DefaultFuseHandler<PathBuf>,
}

impl FuseHandler for MyFs {
    type TId = PathBuf;
    delegate_fs! { default_fs, [ access, getattr, read, /* ... */ ] }
}
```

### Added

- **Async concurrency model** (`fuse_async` feature): Full `async`/`await` support via `tokio` with `#[async_trait]`.
- **`delegate_fs!` macro**: Synchronously delegates `FuseHandler` methods to a named field (serial/parallel modes).
- **`delegate_fs_async!` macro**: Delegates to an async target field (async mode).
- **`delegate_fs_sync_to_async!` macro**: Wraps synchronous method calls in a pinned async block for use in async mode.
- **`easy_fuser_macro` proc-macro crate**: Houses all delegation macros and the `fuse_handler_fnsig!` DSL used internally.
- **`FuseSession` and `FusePruner`**: Public API for managing a background-mounted filesystem session and pruning unreferenced inodes.
- **macOS and BSD support** (#92, #93): Platform-specific filesystem implementations and CI workflows.
- **Templated code generation**: `fuse_driver`, `fuse_handler`, `mounting`, and `fuse_lib` modules are now generated at build time from Askama (Jinja2) templates per concurrency mode, eliminating duplicate code.
- **`readdirplus` default implementation**: Default impl combining `readdir` + `lookup` provided on `FuseHandler`.
- **Default `forget` implementation**: No-op default provided; users can override for manual inode management.
- **macOS CI** (`.github/workflows/macos.yml`): Automated testing on macOS runners.
- **Async integration test** (`tests/async_test.rs`): End-to-end test for the async concurrency mode.

### Changed

- `MirrorFs` and `DefaultFuseHandler` are now composable building blocks meant to be used via `delegate_fs!`, not subclassed.
- `inode_mapper` and `inode_multi_mapper` stabilized backing ID implementation (#90).
- Integration test suite significantly expanded with read-only and read-write scenarios.
- `spawn_mount` and `mount` documentation consolidated into the generated `mounting.rs` module.

### Removed

- `easy_fuser::prelude` (replaced by mode-specific preludes).
- `easy_fuser::templates` module (replaced by `easy_fuser::fuse_presets`).
- `src/core/fuse_driver.rs`, `src/core/fuse_driver_types.rs`, `src/core/macros.rs`, `src/core/thread_mode.rs` (replaced by Askama-generated equivalents).
- `docs/mount.md`, `docs/spawn_mount.md` (documentation now lives in generated `mounting.rs`).
- `tests/mount_mirror_fs.rs` (superseded by the expanded `integration_test.rs`).

---

## [0.4.5]

### Fixed
- `getattr` IO error handling (#83, issue #55).

### Changed
- Internal `if-let`-chains stabilized using `if let Some(...) && ...` pattern (#85).

---

## [0.4.4] and earlier

Please refer to the [GitHub releases page](https://github.com/Alogani/easy_fuser/releases) and git history for older changelogs.

---

[Unreleased]: https://github.com/Alogani/easy_fuser/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/Alogani/easy_fuser/compare/v0.4.5...v0.5.0
[0.4.5]: https://github.com/Alogani/easy_fuser/compare/v0.4.4...v0.4.5
