# Changelog

## [1.0.0](https://github.com/n0-computer/n0-watcher/compare/v1.0.0-rc.0..1.0.0) - 2026-06-15

### ⛰️  Features

- [**breaking**] Update to n0-error@1.0 ([#59](https://github.com/n0-computer/n0-watcher/issues/59)) - ([7471990](https://github.com/n0-computer/n0-watcher/commit/7471990a8eab8daaf437139146363d0d1a05b9d8))

## [1.0.0-rc.0](https://github.com/n0-computer/n0-watcher/compare/0.6.1..v1.0.0-rc.0) - 2026-05-06

### ⚙️ Miscellaneous Tasks

- Update deps and docs ([#57](https://github.com/n0-computer/n0-watcher/issues/57)) - ([c981b92](https://github.com/n0-computer/n0-watcher/commit/c981b9288879c347f676cb08319c376a9eb8a45b))

## [0.6.1](https://github.com/n0-computer/n0-watcher/compare/v0.6.0..0.6.1) - 2026-02-04

### 🐛 Bug Fixes

- Free the allocation the Weak pointer keeps alive once we notice its strong count is 0 ([#38](https://github.com/n0-computer/n0-watcher/issues/38)) - ([3e3c8e7](https://github.com/n0-computer/n0-watcher/commit/3e3c8e7883a869fe262981d4b9c9249618484bcc))
- Avoid adding wakers that wake the same task ([#46](https://github.com/n0-computer/n0-watcher/issues/46)) - ([c2e2ce8](https://github.com/n0-computer/n0-watcher/commit/c2e2ce8359db7b17aabf8f80307b1a274483f72e))

## [0.6.0](https://github.com/n0-computer/n0-watcher/compare/v0.5.0..v0.6.0) - 2025-11-12

### ⛰️  Features

- Split up `Watcher::get` into `Watcher::update` and `Watcher::peek` for more control over perf ([#32](https://github.com/n0-computer/n0-watcher/issues/32)) - ([956c17b](https://github.com/n0-computer/n0-watcher/commit/956c17b108d79e390556db750a93205839c733fd))

## [0.5.0](https://github.com/n0-computer/n0-watcher/compare/v0.4.0..v0.5.0) - 2025-11-03

### ⛰️  Features

- Add three tuple watcher implementation - ([7f2da2b](https://github.com/n0-computer/n0-watcher/commit/7f2da2bb7ee3c65482e52fd24a1ae82a2d682f1f))

### 🚜 Refactor

- Migrate from snafu to n0-error ([#28](https://github.com/n0-computer/n0-watcher/issues/28)) - ([7b794c6](https://github.com/n0-computer/n0-watcher/commit/7b794c6274c703ade983f6e67cfad174731bce02))

## [0.3.1](https://github.com/n0-computer/n0-watcher/compare/v0.3.0..v0.3.1) - 2025-10-21

### ⛰️  Features

- Don't return `Result` in `Watcher::map` ([#9](https://github.com/n0-computer/n0-watcher/issues/9)) - ([40f79f0](https://github.com/n0-computer/n0-watcher/commit/40f79f034d0fed7752a734894551d940707d0afd))

### ⚙️ Miscellaneous Tasks

- Update tracing-subscriber for security fix ([#13](https://github.com/n0-computer/n0-watcher/issues/13)) - ([a9375bd](https://github.com/n0-computer/n0-watcher/commit/a9375bd5f869d7e9ffaf09708dcb7042b0a8bab7))
- Add dependabot for crates.io ([#17](https://github.com/n0-computer/n0-watcher/issues/17)) - ([6e115ec](https://github.com/n0-computer/n0-watcher/commit/6e115ec30c47f3db4cc337399899c5f051de4d5b))
- Update n0-future to 0.3.0 - ([b421d87](https://github.com/n0-computer/n0-watcher/commit/b421d87e6adfc298c8616ff401564495ad5a37c5))

### Deps

- Bump n0-future and rand - ([4769260](https://github.com/n0-computer/n0-watcher/commit/47692605ff308e3413ebf7bdf442d4b76d1d8410))

## [0.3.0](https://github.com/n0-computer/n0-watcher/compare/v0.2.0..v0.3.0) - 2025-07-25

### ⛰️  Features

- [**breaking**] Make `Watcher::get` take `&mut self` and return a value instead of `Result` ([#6](https://github.com/n0-computer/n0-watcher/issues/6)) - ([eaf849e](https://github.com/n0-computer/n0-watcher/commit/eaf849eb590e7a6affe3a8a2ca32bb0a48488436))
- Implement `Watchable::has_watchers` and fix a couple of edge cases ([#7](https://github.com/n0-computer/n0-watcher/issues/7)) - ([437cf43](https://github.com/n0-computer/n0-watcher/commit/437cf43942cb8b9941fd14a0a65e26fce1fd7bb1))

### ⚙️ Miscellaneous Tasks

- Fix CI link to workflow - ([04bb820](https://github.com/n0-computer/n0-watcher/commit/04bb820a7627952d7d93c60a0e60c98c2c92f893))
- Fix warnings ([#5](https://github.com/n0-computer/n0-watcher/issues/5)) - ([f29bd96](https://github.com/n0-computer/n0-watcher/commit/f29bd961eb714ebfd862b3861565c997f235de31))
- Bump version ([#8](https://github.com/n0-computer/n0-watcher/issues/8)) - ([d304163](https://github.com/n0-computer/n0-watcher/commit/d30416359622bddeed1a641c714f00963ed42a43))

## [0.2.0](https://github.com/n0-computer/n0-watcher/compare/v0.1.0..v0.2.0) - 2025-06-03

### ⛰️  Features

- Ci ([#2](https://github.com/n0-computer/n0-watcher/issues/2)) - ([90980d0](https://github.com/n0-computer/n0-watcher/commit/90980d05a581256c7aa2c9d00ee423b9c3ab6a91))
- Switch to snafu from thiserror - ([7765c73](https://github.com/n0-computer/n0-watcher/commit/7765c738a269395a1b5706d7ece858359dea9e81))

## [0.1.0] - 2025-05-19

### ⛰️  Features

- Add join combinator - ([e0b9ccb](https://github.com/n0-computer/n0-watcher/commit/e0b9ccb2af8c2f749e615fc0f9580c59ba27be14))

### ⚙️ Miscellaneous Tasks

- Prep release - ([15407c0](https://github.com/n0-computer/n0-watcher/commit/15407c0e6be13cc479bb09bc949fa4c1394452e3))


