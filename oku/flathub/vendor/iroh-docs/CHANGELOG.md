# Changelog

All notable changes to iroh-docs will be documented in this file.

## [0.101.0](https://github.com/n0-computer/iroh-docs/compare/v0.100.0..0.101.0) - 2026-06-15

### 🐛 Bug Fixes

- *(store)* Migrate redb 2.x tuple tables on open ([#105](https://github.com/n0-computer/iroh-docs/issues/105)) - ([fc89461](https://github.com/n0-computer/iroh-docs/commit/fc894617c449a99580bd2a87a5f55d2fecd21d0f))

### Deps

- Update iroh to 1.0, iroh-blobs to 0.103, iroh-gossip to 0.101 ([#107](https://github.com/n0-computer/iroh-docs/issues/107)) - ([abb3594](https://github.com/n0-computer/iroh-docs/commit/abb3594a23c5e0622a89af067e8bcbe5e384a164))

## [0.100.0](https://github.com/n0-computer/iroh-docs/compare/v0.99.1..0.100.0) - 2026-05-27

### ⛰️  Features

- [**breaking**] Update to iroh@1.0.0-rc.1 ([#103](https://github.com/n0-computer/iroh-docs/issues/103)) - ([aa81fba](https://github.com/n0-computer/iroh-docs/commit/aa81fbae0f6722d28379a53c933e0c356a58f4b0))

### 🚜 Refactor

- [**breaking**] Complete ed25519_dalek to iroh-base public API migration ([#102](https://github.com/n0-computer/iroh-docs/issues/102)) - ([14ab6a6](https://github.com/n0-computer/iroh-docs/commit/14ab6a687a950e7ad29bffa17891cbb7c6347d4d))

## [0.99.1](https://github.com/n0-computer/iroh-docs/compare/v0.99.0..0.99.1) - 2026-05-26

### 🐛 Bug Fixes

- *(sync)* Wrap EntrySignature in iroh::Signature and lock wire format ([#101](https://github.com/n0-computer/iroh-docs/issues/101)) - ([466e541](https://github.com/n0-computer/iroh-docs/commit/466e541202ae2580dec9871c1a8c7b9edd8848ce))

## [0.99.0](https://github.com/n0-computer/iroh-docs/compare/v0.98.0..0.99.0) - 2026-05-08

### ⛰️  Features

- *(sync)* Short-hex Debug for peer from ([#98](https://github.com/n0-computer/iroh-docs/issues/98)) - ([1f88013](https://github.com/n0-computer/iroh-docs/commit/1f880135c27e4c95f705f8338d41650af14646ef))
- [**breaking**] Update to latest 1.0.0-rc.0 deps and redb@4 ([#100](https://github.com/n0-computer/iroh-docs/issues/100)) - ([d4092bf](https://github.com/n0-computer/iroh-docs/commit/d4092bf82a1535ae34e013d89e2670bf4de6ecc3))

### 🐛 Bug Fixes

- *(actor)* Drain Actor::tasks JoinSet in run_async ([#96](https://github.com/n0-computer/iroh-docs/issues/96)) - ([b4f985d](https://github.com/n0-computer/iroh-docs/commit/b4f985d20567be85fead51101520a0320c68b79e))
- *(sync)* Use microseconds for MAX_TIMESTAMP_FUTURE_SHIFT ([#99](https://github.com/n0-computer/iroh-docs/issues/99)) - ([8e3f2ba](https://github.com/n0-computer/iroh-docs/commit/8e3f2ba4064e0d5aac0d4de3065f889db1211240))

## [0.98.0](https://github.com/n0-computer/iroh-docs/compare/v0.97.0..0.98.0) - 2026-04-20

### ⛰️  Features

- [**breaking**] Update to iroh@0.98n, iroh-gossip@0.98, iroh-blobs@0.100 ([#95](https://github.com/n0-computer/iroh-docs/issues/95)) - ([e03aa6f](https://github.com/n0-computer/iroh-docs/commit/e03aa6fb4aef36d294a94bfa7e80821b245a0184))

## [0.97.0](https://github.com/n0-computer/iroh-docs/compare/v0.96.0..0.97.0) - 2026-03-17

### ⛰️  Features

- Build on wasm ([#75](https://github.com/n0-computer/iroh-docs/issues/75)) - ([bf1fc32](https://github.com/n0-computer/iroh-docs/commit/bf1fc3285c9f8c4b3678d8b5e39ee7b7d1ea78c0))
- Update to iroh@main ([#85](https://github.com/n0-computer/iroh-docs/issues/85)) - ([b81c4f1](https://github.com/n0-computer/iroh-docs/commit/b81c4f14a2da39ed30ddcdd0548ccbbd5a1be6ed))

### 🐛 Bug Fixes

- Configure git identity in cleanup workflow ([#83](https://github.com/n0-computer/iroh-docs/issues/83)) - ([ec49201](https://github.com/n0-computer/iroh-docs/commit/ec492017c2e46025508cde85914199cfca843170))

## [0.96.0](https://github.com/n0-computer/iroh-docs/compare/v0.95.0..0.96.0) - 2026-01-29

### ⚙️ Miscellaneous Tasks

- Upgrade to `iroh` 0.96 - ([5133ca4](https://github.com/n0-computer/iroh-docs/commit/5133ca4360f3fd9e4891103d73cd6878200fde27))

## [0.95.0](https://github.com/n0-computer/iroh-docs/compare/v0.94.0..0.95.0) - 2025-11-06

### ⚙️ Miscellaneous Tasks

- Release prep ([#76](https://github.com/n0-computer/iroh-docs/issues/76)) - ([a226a2b](https://github.com/n0-computer/iroh-docs/commit/a226a2bf5244b4b1cf4bb5998e585afd84802a1a))

## [0.94.0](https://github.com/n0-computer/iroh-docs/compare/v0.93.1..0.94.0) - 2025-10-22

### ⛰️  Features

- Upgrade redb to v3 compatible format ([#69](https://github.com/n0-computer/iroh-docs/issues/69)) - ([b257606](https://github.com/n0-computer/iroh-docs/commit/b257606c6b40abc0fa4d0d78542e267367d45bdb))

### ⚙️ Miscellaneous Tasks

- Release prep ([#73](https://github.com/n0-computer/iroh-docs/issues/73)) - ([f1b53df](https://github.com/n0-computer/iroh-docs/commit/f1b53df9e61fbe3f2513652b121f482eb85be484))

## [0.93.1](https://github.com/n0-computer/iroh-docs/compare/v0.93.0..0.93.1) - 2025-10-11

### ⚙️ Miscellaneous Tasks

- Upgrade nightly version in CI and doc workflows ([#72](https://github.com/n0-computer/iroh-docs/issues/72)) - ([415f62a](https://github.com/n0-computer/iroh-docs/commit/415f62ab79ddf8a4c7cf7af383f8d382c0e7c91a))

## [0.93.0](https://github.com/n0-computer/iroh-docs/compare/v0.92.0..0.93.0) - 2025-10-09

### ⚙️ Miscellaneous Tasks

- Release prep ([#71](https://github.com/n0-computer/iroh-docs/issues/71)) - ([3c598b5](https://github.com/n0-computer/iroh-docs/commit/3c598b59ba63593ff8ccdafdf9c8cf554fff5e45))

## [0.92.0](https://github.com/n0-computer/iroh-docs/compare/v0.91.0..0.92.0) - 2025-09-19

### 🐛 Bug Fixes

- Pub mod actor exposing SyncHandle ([#52](https://github.com/n0-computer/iroh-docs/issues/52)) - ([8a4d9e2](https://github.com/n0-computer/iroh-docs/commit/8a4d9e2e0a2476dde3c892160dad3452c0c3b173))

### ⚙️ Miscellaneous Tasks

- Clippy fixes ([#54](https://github.com/n0-computer/iroh-docs/issues/54)) - ([cea3ebe](https://github.com/n0-computer/iroh-docs/commit/cea3ebe97a15c9f59830f7f860db4ab8041e115f))
- Upgrade `iroh`, `irpc`, `iroh-blobs`,  ([#58](https://github.com/n0-computer/iroh-docs/issues/58)) - ([783c559](https://github.com/n0-computer/iroh-docs/commit/783c559034d33606a193a320490a81c101a00289))

## [0.91.0](https://github.com/n0-computer/iroh-docs/compare/v0.35.0..0.91.0) - 2025-08-01

### Deps/refactor

- Port to iroh 0.91 ([#47](https://github.com/n0-computer/iroh-docs/issues/47)) - ([f3214d0](https://github.com/n0-computer/iroh-docs/commit/f3214d016fbdf748cf89eed49a71bf9197dcb201))

## [0.35.0](https://github.com/n0-computer/iroh-docs/compare/v0.34.0..0.35.0) - 2025-05-12

### 🐛 Bug Fixes

- Flaky tests ([#40](https://github.com/n0-computer/iroh-docs/issues/40)) - ([faa929c](https://github.com/n0-computer/iroh-docs/commit/faa929ccfea052d2c3f14bea2b35aef7aa893f9a))

### 🚜 Refactor

- [**breaking**] Update to iroh 0.35 and non-global metrics tracking ([#41](https://github.com/n0-computer/iroh-docs/issues/41)) - ([8721c39](https://github.com/n0-computer/iroh-docs/commit/8721c39a2b33a8b9d3858778f1e983494919cc56))

### ⚙️ Miscellaneous Tasks

- Update sccache github action ([#42](https://github.com/n0-computer/iroh-docs/issues/42)) - ([e8b89a8](https://github.com/n0-computer/iroh-docs/commit/e8b89a82ab05c1174c2e20180d226f0fcfc18277))

## [0.34.0](https://github.com/n0-computer/iroh-docs/compare/v0.33.0..0.34.0) - 2025-03-18

### ⚙️ Miscellaneous Tasks

- Patch to use main branch of iroh dependencies ([#37](https://github.com/n0-computer/iroh-docs/issues/37)) - ([2f79a3f](https://github.com/n0-computer/iroh-docs/commit/2f79a3fa6d1394b5d3eeca72d477a79bb41dce07))
- Update to latest iroh ([#39](https://github.com/n0-computer/iroh-docs/issues/39)) - ([e6432d1](https://github.com/n0-computer/iroh-docs/commit/e6432d1d5b3fa818c08997e74fbfdfe347a82872))

## [0.33.0](https://github.com/n0-computer/iroh-docs/compare/v0.32.0..0.33.0) - 2025-02-25

### ⚙️ Miscellaneous Tasks

- Patch to use main branch of iroh dependencies ([#35](https://github.com/n0-computer/iroh-docs/issues/35)) - ([8c6951a](https://github.com/n0-computer/iroh-docs/commit/8c6951abef4f62d633e592bd25a18827243d4e85))
- Upgrade to latest `iroh`, `iroh-gossip` and `iroh-blobs` ([#36](https://github.com/n0-computer/iroh-docs/issues/36)) - ([911a8cc](https://github.com/n0-computer/iroh-docs/commit/911a8cc4553cb7805aa3f92db81b29759c73b25e))

## [0.32.0](https://github.com/n0-computer/iroh-docs/compare/v0.31.0..0.32.0) - 2025-02-05

### ⚙️ Miscellaneous Tasks

- Remove individual repo project tracking ([#29](https://github.com/n0-computer/iroh-docs/issues/29)) - ([7c4c871](https://github.com/n0-computer/iroh-docs/commit/7c4c8715cc2dd76649bd89d8f667d516a15a7a9f))
- Upgrade to `iroh`, `iroh-gossip`, and `iroh-blobs` v0.32.0 ([#33](https://github.com/n0-computer/iroh-docs/issues/33)) - ([85ea65d](https://github.com/n0-computer/iroh-docs/commit/85ea65d033ed604a9332f4c3a3ac5b003692c461))

## [0.31.0](https://github.com/n0-computer/iroh-docs/compare/v0.30.0..0.31.0) - 2025-01-14

### ⚙️ Miscellaneous Tasks

- Add project tracking ([#24](https://github.com/n0-computer/iroh-docs/issues/24)) - ([a5219d1](https://github.com/n0-computer/iroh-docs/commit/a5219d186eeeed0c05f9c8082adf02719fdf8e1f))
- Pin nextest version ([#25](https://github.com/n0-computer/iroh-docs/issues/25)) - ([9340e32](https://github.com/n0-computer/iroh-docs/commit/9340e32db71a634e06026c4861e8981774736c16))
- Fix URL to actions by using a variable ([#27](https://github.com/n0-computer/iroh-docs/issues/27)) - ([7667e46](https://github.com/n0-computer/iroh-docs/commit/7667e46a03d9ac34e23b4e2e25c8f9b9a0b414a3))
- Upgrade to `iroh@v0.31.0` ([#28](https://github.com/n0-computer/iroh-docs/issues/28)) - ([7911992](https://github.com/n0-computer/iroh-docs/commit/7911992383b8bfe633a6f56f749beaaeb5a96f64))

## [0.30.0](https://github.com/n0-computer/iroh-docs/compare/v0.29.0..0.30.0) - 2024-12-17

### ⛰️  Features

- [**breaking**] Remove rpc from the default feature flag set - ([5897668](https://github.com/n0-computer/iroh-docs/commit/5897668996cf509fcbcb0f9801925a3d3ad89148))
- [**breaking**] Add Docs<S> which wraps the Engine<S> ([#18](https://github.com/n0-computer/iroh-docs/issues/18)) - ([857f0dc](https://github.com/n0-computer/iroh-docs/commit/857f0dc7e9b14d27b2a21e0f12ff00609b077a52))
- [**breaking**] Update to iroh@0.30.0 & MSRV 1.81 - ([31b6698](https://github.com/n0-computer/iroh-docs/commit/31b66980a8216a7f52183a6f46bb9455c58726ae))

### 🚜 Refactor

- Adapt to new ProtocolHandler ([#15](https://github.com/n0-computer/iroh-docs/issues/15)) - ([b6c7616](https://github.com/n0-computer/iroh-docs/commit/b6c7616c401ac7c465beafebdc49f866263cf026))

### 📚 Documentation

- Add "Getting Started" section to the README and add the readme to the docs ([#19](https://github.com/n0-computer/iroh-docs/issues/19)) - ([98e2e17](https://github.com/n0-computer/iroh-docs/commit/98e2e170bcf4c1f23aa1479ae6c2b21b92c1146d))

## [0.29.0](https://github.com/n0-computer/iroh-docs/compare/v0.28.0..0.29.0) - 2024-12-04

### ⛰️  Features

- Update to latest iroh  - ([c146f7a](https://github.com/n0-computer/iroh-docs/commit/c146f7a969926eb2624fbe3ba4f93f157fc04864))
- Update to iroh@0.29.0 - ([a625e88](https://github.com/n0-computer/iroh-docs/commit/a625e88889853dd98837982b8344af20b54ac606))

### 🚜 Refactor

- Extract RPC definitions into here - ([176715a](https://github.com/n0-computer/iroh-docs/commit/176715a567c7f5c660a980dcc334b5d5892d2cb1))
- Remove all blobs gc related tests - ([c63bae3](https://github.com/n0-computer/iroh-docs/commit/c63bae32e5a6fc78db6b4b2549bc56f82fbf94db))
- Switch to hex - ([e8fa3da](https://github.com/n0-computer/iroh-docs/commit/e8fa3dae967540dc4d6ed7361357d23500e5932a))

### ⚙️ Miscellaneous Tasks

- Prune some deps - ([b6fc71d](https://github.com/n0-computer/iroh-docs/commit/b6fc71df5dbdb745afe1852315c1176109360079))
- Init changelog - ([dd2d6d2](https://github.com/n0-computer/iroh-docs/commit/dd2d6d286f91c78262c66133a0304d270123d402))
- Fix release config - ([0ac3fbe](https://github.com/n0-computer/iroh-docs/commit/0ac3fbe60403edd3a302a5c53500299360e31797))

## [0.28.0] - 2024-11-04

### ⛰️  Features

- *(iroh-gossip)* Configure the max message size ([#2340](https://github.com/n0-computer/iroh-docs/issues/2340)) - ([8815bed](https://github.com/n0-computer/iroh-docs/commit/8815bedd3009a7013ea6eae3447622696e0bee0f))
- *(iroh-net)* [**breaking**] Improve initial connection latency ([#2234](https://github.com/n0-computer/iroh-docs/issues/2234)) - ([62afe07](https://github.com/n0-computer/iroh-docs/commit/62afe077adc67bd13db38cf52d875ea5c96e8f07))
- *(iroh-net)* Own the public QUIC API ([#2279](https://github.com/n0-computer/iroh-docs/issues/2279)) - ([6f4c35e](https://github.com/n0-computer/iroh-docs/commit/6f4c35ef9c51c671c4594ea828a3dd29bd952ce5))
- *(iroh-net)* [**breaking**] Upgrade to Quinn 0.11 and Rustls 0.23 ([#2595](https://github.com/n0-computer/iroh-docs/issues/2595)) - ([add0bc3](https://github.com/n0-computer/iroh-docs/commit/add0bc3a9a002d685e755f8fce03241dc9fa500a))
- [**breaking**] New quic-rpc, simlified generics, bump MSRV to 1.76 ([#2268](https://github.com/n0-computer/iroh-docs/issues/2268)) - ([9c49883](https://github.com/n0-computer/iroh-docs/commit/9c498836f8de4c9c42f15d422f282fa3efdbae10))
- Set derive_more to 1.0.0 (no beta!) ([#2736](https://github.com/n0-computer/iroh-docs/issues/2736)) - ([9e517d0](https://github.com/n0-computer/iroh-docs/commit/9e517d09e361978f3a95983f6c906068d1cf6512))
- Ci ([#1](https://github.com/n0-computer/iroh-docs/issues/1)) - ([6b2c973](https://github.com/n0-computer/iroh-docs/commit/6b2c973f8ba9acefce1b36df93a8e6a5f8d7392a))

### 🐛 Bug Fixes

- *(docs)* Prevent deadlocks with streams returned from docs actor ([#2346](https://github.com/n0-computer/iroh-docs/issues/2346)) - ([6251369](https://github.com/n0-computer/iroh-docs/commit/625136937a2fa63479c760ce9a13d24f8c97771c))
- *(iroh-docs)* Ensure docs db write txn gets closed regularly under all circumstances ([#2474](https://github.com/n0-computer/iroh-docs/issues/2474)) - ([ea226dd](https://github.com/n0-computer/iroh-docs/commit/ea226dd48c0dcc0397f319aeb767563cbec298c6))
- *(iroh-docs)* [**breaking**] Add `flush_store` and use it to make sure the default author is persisted ([#2471](https://github.com/n0-computer/iroh-docs/issues/2471)) - ([f67a8b1](https://github.com/n0-computer/iroh-docs/commit/f67a8b143a36a18903c1338e8412cdd7f1729582))
- *(iroh-docs)* Do not dial invalid peers ([#2470](https://github.com/n0-computer/iroh-docs/issues/2470)) - ([dc7645e](https://github.com/n0-computer/iroh-docs/commit/dc7645e45ce875b9c19d32d65544abefd0417e80))
- *(iroh-net)* Prevent adding addressing info that points back to us ([#2333](https://github.com/n0-computer/iroh-docs/issues/2333)) - ([54d5991](https://github.com/n0-computer/iroh-docs/commit/54d5991f5499ed52749e420eb647d726a3674eb4))
- *(iroh-net)* Fix a compiler error with newer `derive_more` versions ([#2578](https://github.com/n0-computer/iroh-docs/issues/2578)) - ([3f93765](https://github.com/n0-computer/iroh-docs/commit/3f93765ea36f47554123bfb48c3ac06068b970b6))
- *(iroh-net)* [**breaking**] DiscoveredDirectAddrs need to update the timestamp ([#2808](https://github.com/n0-computer/iroh-docs/issues/2808)) - ([916aedd](https://github.com/n0-computer/iroh-docs/commit/916aedd4a7a0f458be3d19d1d3f3d46d0bb074ad))
- Properly wait for docs engine shutdown ([#2389](https://github.com/n0-computer/iroh-docs/issues/2389)) - ([5b48493](https://github.com/n0-computer/iroh-docs/commit/5b48493917a5a87b21955fefc2f341a2f9fa0a1e))
- Pin derive_more to avoid sudden breakages ([#2584](https://github.com/n0-computer/iroh-docs/issues/2584)) - ([cfda981](https://github.com/n0-computer/iroh-docs/commit/cfda981a0ad075111e0c64e3e9a145d4490d1cc5))

### 🚜 Refactor

- *(iroh)* [**breaking**] Remove tags from downloader ([#2348](https://github.com/n0-computer/iroh-docs/issues/2348)) - ([38f75de](https://github.com/n0-computer/iroh-docs/commit/38f75de7cbde0a39582afcac12c73a571705ea58))
- *(iroh-docs)* Replace flume with async_channel in docs ([#2540](https://github.com/n0-computer/iroh-docs/issues/2540)) - ([335b6b1](https://github.com/n0-computer/iroh-docs/commit/335b6b15bf07471c672320127c205a34788462bb))
- *(iroh-net)* [**breaking**] Rename MagicEndpoint -> Endpoint ([#2287](https://github.com/n0-computer/iroh-docs/issues/2287)) - ([86e7c07](https://github.com/n0-computer/iroh-docs/commit/86e7c07a41953f07994dd5d31b9ed3d529a519c3))
- Renames iroh-sync & iroh-bytes ([#2271](https://github.com/n0-computer/iroh-docs/issues/2271)) - ([f1c3cf9](https://github.com/n0-computer/iroh-docs/commit/f1c3cf9b96fab2340df3dc0f5355137234772d95))
- Move docs engine into iroh-docs ([#2343](https://github.com/n0-computer/iroh-docs/issues/2343)) - ([28cf40b](https://github.com/n0-computer/iroh-docs/commit/28cf40b295c3adca75f9328a23be3ff0e6e8a57f))
- [**breaking**] Metrics ([#2464](https://github.com/n0-computer/iroh-docs/issues/2464)) - ([4938055](https://github.com/n0-computer/iroh-docs/commit/4938055577fb6a975e547983ebdfb1c3b6b03df9))
- [**breaking**] Migrate to tokio's AbortOnDropHandle ([#2701](https://github.com/n0-computer/iroh-docs/issues/2701)) - ([cf05227](https://github.com/n0-computer/iroh-docs/commit/cf05227141cacace21fe914c0479786000989869))
- Extract `ProtocolHandler` impl into here - ([16bc7fe](https://github.com/n0-computer/iroh-docs/commit/16bc7fe4c7dee1b1b88f54390856c3fafa3d7656))

### 📚 Documentation

- *(*)* Document cargo features in docs ([#2761](https://github.com/n0-computer/iroh-docs/issues/2761)) - ([8346d50](https://github.com/n0-computer/iroh-docs/commit/8346d506fd2553bc813548a7d41f04adc984f7d2))
- Fix typos discovered by codespell ([#2534](https://github.com/n0-computer/iroh-docs/issues/2534)) - ([b5b7072](https://github.com/n0-computer/iroh-docs/commit/b5b70726f10cd33d354e07b8985e171cdd3cfc0f))

### ⚙️ Miscellaneous Tasks

- Release - ([b8dd924](https://github.com/n0-computer/iroh-docs/commit/b8dd9243361b708a41b15cad4809ad5b34d73c87))
- Release - ([045e3ce](https://github.com/n0-computer/iroh-docs/commit/045e3ce6dd56bf57b2127960fd1ebad4b9858026))
- Release - ([a9a8700](https://github.com/n0-computer/iroh-docs/commit/a9a87007b597f4f41c68dcbebe8b30bdb2cd3119))
- Release - ([08fe6f3](https://github.com/n0-computer/iroh-docs/commit/08fe6f3ad6fc4ded76991b059a8ffefaf06c1129))
- Introduce crate-ci/typos ([#2430](https://github.com/n0-computer/iroh-docs/issues/2430)) - ([b97e407](https://github.com/n0-computer/iroh-docs/commit/b97e4071508191894581e48882a53cf6b095f86f))
- Release - ([73c22f7](https://github.com/n0-computer/iroh-docs/commit/73c22f7f13ce0276677fc599e80c11f470127966))
- Release - ([9b53b1b](https://github.com/n0-computer/iroh-docs/commit/9b53b1bee105952d883a81bb2238b1e569f78332))
- Fix clippy warnings ([#2550](https://github.com/n0-computer/iroh-docs/issues/2550)) - ([956c51a](https://github.com/n0-computer/iroh-docs/commit/956c51a21b49e15e136434205dbc21040aa8b62b))
- Release - ([5fd07fa](https://github.com/n0-computer/iroh-docs/commit/5fd07fabf313828d5484b7ea315199f9bd7178b9))
- Release - ([92d8493](https://github.com/n0-computer/iroh-docs/commit/92d8493071ac9c5642358daea93b47b1c620e312))
- Release - ([f929d15](https://github.com/n0-computer/iroh-docs/commit/f929d15e76bfabb6d713402daff3b399f122e016))
- Release - ([7e3f028](https://github.com/n0-computer/iroh-docs/commit/7e3f028c65028c1b16c41a01fcc25ff6e2ad2e63))
- Release - ([5f98043](https://github.com/n0-computer/iroh-docs/commit/5f980439a5305bf781ea8c04f3d20976d26f88a8))
- Format imports using rustfmt ([#2812](https://github.com/n0-computer/iroh-docs/issues/2812)) - ([e6aadbd](https://github.com/n0-computer/iroh-docs/commit/e6aadbd337dda7697a34526a5a17fb38d15978c6))
- Increase version numbers and update ([#2821](https://github.com/n0-computer/iroh-docs/issues/2821)) - ([5ac936f](https://github.com/n0-computer/iroh-docs/commit/5ac936f0d5a7a8a6ffe9931638dbb009dc339494))
- Release - ([d52df5f](https://github.com/n0-computer/iroh-docs/commit/d52df5f46d1c9fea2b2012a32f8fe9a3b86a14e3))
- Copy missing files - ([61d8e6a](https://github.com/n0-computer/iroh-docs/commit/61d8e6ada870d736182333d3dc4f2c2387e1dbb4))
- Update Cargo.toml - ([6be9cb0](https://github.com/n0-computer/iroh-docs/commit/6be9cb01a3dc8ba0d9cbd74572231b1a18df36ae))
- Upgrade 0.28 iroh-net - ([7e48c6a](https://github.com/n0-computer/iroh-docs/commit/7e48c6a20e411c8e5928b5f09fe0ba46d5880ce5))
- Upgrade 0.28 iroh-router - ([933af7a](https://github.com/n0-computer/iroh-docs/commit/933af7af0e38281853042764aaa99ad95327d320))
- Release iroh-docs version 0.28.0 - ([c3017de](https://github.com/n0-computer/iroh-docs/commit/c3017de4573930fd03a05ec4aa6c7ab7a84f1890))


