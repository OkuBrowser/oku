# Changelog

All notable changes to iroh-blobs will be documented in this file.

## [0.103.0](https://github.com/n0-computer/iroh-blobs/compare/v0.102.0..0.103.0) - 2026-06-15

### 🐛 Bug Fixes

- Report the empty blob as `Complete` from `status` ([#238](https://github.com/n0-computer/iroh-blobs/issues/238)) - ([e7343de](https://github.com/n0-computer/iroh-blobs/commit/e7343def2a6e11d9d3a5b462c631181db4a48fe1))

### 📚 Documentation

- Add temp_tag pattern for protecting long-running downloads ([#236](https://github.com/n0-computer/iroh-blobs/issues/236)) - ([789f904](https://github.com/n0-computer/iroh-blobs/commit/789f904556c5bdef2ca4f7a682c4afd46e085fc1))

### Deps

- [**breaking**] Update to iroh 1.0 ([#239](https://github.com/n0-computer/iroh-blobs/issues/239)) - ([5a3cfad](https://github.com/n0-computer/iroh-blobs/commit/5a3cfadcde17cecb79e5db1b9d7c398c08c3ae16))

## [0.102.0](https://github.com/n0-computer/iroh-blobs/compare/v0.101.0..0.102.0) - 2026-05-27

### ⛰️  Features

- [**breaking**] Update to iroh@1.0.0-rc.1 ([#234](https://github.com/n0-computer/iroh-blobs/issues/234)) - ([8933aff](https://github.com/n0-computer/iroh-blobs/commit/8933aff25b6a12461ea2595f1eb913c653f100e5))

## [0.101.0](https://github.com/n0-computer/iroh-blobs/compare/v0.100.0..0.101.0) - 2026-05-08

### 🐛 Bug Fixes

- *(api/downloader)* Reap completed download tasks ([#225](https://github.com/n0-computer/iroh-blobs/issues/225)) - ([7cc92c7](https://github.com/n0-computer/iroh-blobs/commit/7cc92c7cf84d4f4dab32704e328551212977d2a2))

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump the github-actions group across 1 directory with 2 updates ([#223](https://github.com/n0-computer/iroh-blobs/issues/223)) - ([181473a](https://github.com/n0-computer/iroh-blobs/commit/181473a92061c387dddb569e42f3bf17190e8e5a))
- Update n0 dependencies ([#228](https://github.com/n0-computer/iroh-blobs/issues/228)) - ([dccf825](https://github.com/n0-computer/iroh-blobs/commit/dccf82520665f80a63505c7c661c9c17fe266fb0))
- Ensure changelog generation - ([192262e](https://github.com/n0-computer/iroh-blobs/commit/192262e5652cc8f6305807e1182620fd0697751b))

### Deps

- Use connection pool from iroh-util ([#229](https://github.com/n0-computer/iroh-blobs/issues/229)) - ([d0ba8d8](https://github.com/n0-computer/iroh-blobs/commit/d0ba8d8f4efa05c80427e688f4b63bca2f6d73c1))
- Use Redb 4 and remove futures-lite dep ([#230](https://github.com/n0-computer/iroh-blobs/issues/230)) - ([2faaf7d](https://github.com/n0-computer/iroh-blobs/commit/2faaf7dbd4180ad0d70592bd5bfdaf6a47e12b0c))

## [0.35.0](https://github.com/n0-computer/iroh-blobs/compare/v0.34.1..0.35.0) - 2025-05-12

### ⛰️  Features

- [**breaking**] Allow configuring the downloader when creating a blobs protocol handler ([#76](https://github.com/n0-computer/iroh-blobs/issues/76)) - ([60be4ff](https://github.com/n0-computer/iroh-blobs/commit/60be4ffda6123f558dcbb5b72b84c653fe65d2a9))

### 🐛 Bug Fixes

- Use latest bao-tree ([#82](https://github.com/n0-computer/iroh-blobs/issues/82)) - ([fbc6f47](https://github.com/n0-computer/iroh-blobs/commit/fbc6f47889b74bf2295b0f88820c393cc8ed4f17))

### 🚜 Refactor

- [**breaking**] Update to latest iroh-metrics version, use non-global metrics collection ([#85](https://github.com/n0-computer/iroh-blobs/issues/85)) - ([0308a77](https://github.com/n0-computer/iroh-blobs/commit/0308a77f9bc949a6f75750b514cdb99cfc1143f7))

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump mozilla-actions/sccache-action from 0.0.8 to 0.0.9 in the github-actions group ([#79](https://github.com/n0-computer/iroh-blobs/issues/79)) - ([d2ff3b1](https://github.com/n0-computer/iroh-blobs/commit/d2ff3b121c89e1ccade90b425b44c937d66882ab))
- Post correct link to discord about flaky failures. ([#83](https://github.com/n0-computer/iroh-blobs/issues/83)) - ([ce939a2](https://github.com/n0-computer/iroh-blobs/commit/ce939a2134fbda7bf80fe711dcedae0a73951cc4))
- Fix redb version to the latest version that uses edition 2021 ([#88](https://github.com/n0-computer/iroh-blobs/issues/88)) - ([25af32b](https://github.com/n0-computer/iroh-blobs/commit/25af32b65ff3494bdf8389ab1a5daea4a4bf1014))
- Update to `iroh` v0.35 ([#91](https://github.com/n0-computer/iroh-blobs/issues/91)) - ([8a975ec](https://github.com/n0-computer/iroh-blobs/commit/8a975ecda8bca5e8988db911857012e6817b456e))

## [0.34.1](https://github.com/n0-computer/iroh-blobs/compare/v0.34.0..0.34.1) - 2025-04-07

### ⛰️  Features

- Boxed client ([#80](https://github.com/n0-computer/iroh-blobs/issues/80)) - ([d8521b4](https://github.com/n0-computer/iroh-blobs/commit/d8521b44cf148c1f6d700067726c1b4e40a0ac27))

### ⚙️ Miscellaneous Tasks

- Update bao-tree dependency and get rid of iroh-blake3 dep ([#81](https://github.com/n0-computer/iroh-blobs/issues/81)) - ([2e823f6](https://github.com/n0-computer/iroh-blobs/commit/2e823f697a251df8d59fcda7bb3b25aa755eff6c))

## [0.34.0](https://github.com/n0-computer/iroh-blobs/compare/v0.33.1..0.34.0) - 2025-03-18

### ⛰️  Features

- Richer tags api ([#69](https://github.com/n0-computer/iroh-blobs/issues/69)) - ([387c68c](https://github.com/n0-computer/iroh-blobs/commit/387c68cc4d084b7067bfedae341abb277eaac8c0))
- Modify Downloader config through Blobs builder ([#75](https://github.com/n0-computer/iroh-blobs/issues/75)) - ([6e9f06b](https://github.com/n0-computer/iroh-blobs/commit/6e9f06b48a97957550e2343694966ac2fee07f39))
- Enable RPC by default ([#73](https://github.com/n0-computer/iroh-blobs/issues/73)) - ([b1029e2](https://github.com/n0-computer/iroh-blobs/commit/b1029e2f5542b56525d53365b040d874549d9fe7))

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump mozilla-actions/sccache-action from 0.0.7 to 0.0.8 in the github-actions group ([#66](https://github.com/n0-computer/iroh-blobs/issues/66)) - ([3e9662c](https://github.com/n0-computer/iroh-blobs/commit/3e9662c9cdb4948f9f8c59e7c74ce6eca7942cf9))
- Update to latest iroh ([#77](https://github.com/n0-computer/iroh-blobs/issues/77)) - ([253a8c6](https://github.com/n0-computer/iroh-blobs/commit/253a8c6bf05db30bf39485822f0e2114481e26ce))
- Update lockfile - ([65a84bb](https://github.com/n0-computer/iroh-blobs/commit/65a84bb011e543e3b752b5d7eda1c5f3c1eba481))

## [0.33.1](https://github.com/n0-computer/iroh-blobs/compare/v0.33.0..0.33.1) - 2025-03-11

### 🐛 Bug Fixes

- Do not panic when parsing invalid hashes ([#68](https://github.com/n0-computer/iroh-blobs/issues/68)) - ([cfdfca0](https://github.com/n0-computer/iroh-blobs/commit/cfdfca04760369a9457ea09b4085ab63588398c1))

### ⚙️ Miscellaneous Tasks

- Patch to use main branch of iroh dependencies ([#64](https://github.com/n0-computer/iroh-blobs/issues/64)) - ([d739d52](https://github.com/n0-computer/iroh-blobs/commit/d739d5225029d40749150ad4f2d5e1c1c6f1c0c4))
- Release ([#70](https://github.com/n0-computer/iroh-blobs/issues/70)) - ([4c282fe](https://github.com/n0-computer/iroh-blobs/commit/4c282fea5536f142fe6aab78de1c58d2871c912f))
- Update change log ([#71](https://github.com/n0-computer/iroh-blobs/issues/71)) - ([f4feff7](https://github.com/n0-computer/iroh-blobs/commit/f4feff72c79559ff09ddc8091e15996cf2df0c27))
- Release iroh-blobs version 0.33.1 - ([e4aa724](https://github.com/n0-computer/iroh-blobs/commit/e4aa7245a3ec31a652a5573b70928d0dffd7fbc7))

## [0.33.1](https://github.com/n0-computer/iroh-blobs/compare/v0.33.0..0.33.1) - 2025-03-11

### 🐛 Bug Fixes

- Do not panic when parsing invalid hashes ([#68](https://github.com/n0-computer/iroh-blobs/issues/68)) - ([cfdfca0](https://github.com/n0-computer/iroh-blobs/commit/cfdfca04760369a9457ea09b4085ab63588398c1))

### ⚙️ Miscellaneous Tasks

- Patch to use main branch of iroh dependencies ([#64](https://github.com/n0-computer/iroh-blobs/issues/64)) - ([d739d52](https://github.com/n0-computer/iroh-blobs/commit/d739d5225029d40749150ad4f2d5e1c1c6f1c0c4))
- Release ([#70](https://github.com/n0-computer/iroh-blobs/issues/70)) - ([4c282fe](https://github.com/n0-computer/iroh-blobs/commit/4c282fea5536f142fe6aab78de1c58d2871c912f))
- Update change log ([#71](https://github.com/n0-computer/iroh-blobs/issues/71)) - ([f4feff7](https://github.com/n0-computer/iroh-blobs/commit/f4feff72c79559ff09ddc8091e15996cf2df0c27))

## [0.33.1](https://github.com/n0-computer/iroh-blobs/compare/v0.33.0..0.33.1) - 2025-03-11

### 🐛 Bug Fixes

- Do not panic when parsing invalid hashes ([#68](https://github.com/n0-computer/iroh-blobs/issues/68)) - ([cfdfca0](https://github.com/n0-computer/iroh-blobs/commit/cfdfca04760369a9457ea09b4085ab63588398c1))

### ⚙️ Miscellaneous Tasks

- Patch to use main branch of iroh dependencies ([#64](https://github.com/n0-computer/iroh-blobs/issues/64)) - ([d739d52](https://github.com/n0-computer/iroh-blobs/commit/d739d5225029d40749150ad4f2d5e1c1c6f1c0c4))
- Release ([#70](https://github.com/n0-computer/iroh-blobs/issues/70)) - ([4c282fe](https://github.com/n0-computer/iroh-blobs/commit/4c282fea5536f142fe6aab78de1c58d2871c912f))

## [0.33.0](https://github.com/n0-computer/iroh-blobs/compare/v0.32.0..0.33.0) - 2025-02-25

### 📚 Documentation

- Update readme ([#55](https://github.com/n0-computer/iroh-blobs/issues/55)) - ([d8d2b48](https://github.com/n0-computer/iroh-blobs/commit/d8d2b48fbaaaf4d604e8583e87c874cdc9c5b3c6))

### ⚙️ Miscellaneous Tasks

- Patch to use main branch of iroh dependencies ([#58](https://github.com/n0-computer/iroh-blobs/issues/58)) - ([57cb626](https://github.com/n0-computer/iroh-blobs/commit/57cb62696bbad313d497c4a33821657fb6bf53ee))
- Upgrade to latest `iroh` and `quic-rpc` ([#63](https://github.com/n0-computer/iroh-blobs/issues/63)) - ([a198ccc](https://github.com/n0-computer/iroh-blobs/commit/a198cccbde55071973e2b637e7e3ea56908f5d7d))

### Example

- Simplify transfer example ([#53](https://github.com/n0-computer/iroh-blobs/issues/53)) - ([bbbb636](https://github.com/n0-computer/iroh-blobs/commit/bbbb63679794345ed9e6155e67d0423667bfbf26))

## [0.32.0](https://github.com/n0-computer/iroh-blobs/compare/v0.31.0..0.32.0) - 2025-02-04

### ⛰️  Features

- Update quic-rpc to 0.18 ([#46](https://github.com/n0-computer/iroh-blobs/issues/46)) - ([030420e](https://github.com/n0-computer/iroh-blobs/commit/030420e7fa03c80b44491f8da16b993f4015007f))
- [**breaking**] Simplify LocalPool handling ([#47](https://github.com/n0-computer/iroh-blobs/issues/47)) - ([b29991d](https://github.com/n0-computer/iroh-blobs/commit/b29991dc913459e034b758271d9b79f8ae6c498e))

### ⚙️ Miscellaneous Tasks

- Fix URL to beta workflow ([#50](https://github.com/n0-computer/iroh-blobs/issues/50)) - ([5cacccb](https://github.com/n0-computer/iroh-blobs/commit/5cacccb33818b11eab487b89da0bb4a69325f52b))
- Remove individual repo project tracking ([#48](https://github.com/n0-computer/iroh-blobs/issues/48)) - ([64b6ae6](https://github.com/n0-computer/iroh-blobs/commit/64b6ae6a6b1dfcdf639ad55923391957b0b4186e))
- Upgrade to `iroh@v0.32.0` and `quic-rpc@v0.18.1` ([#52](https://github.com/n0-computer/iroh-blobs/issues/52)) - ([7dccac9](https://github.com/n0-computer/iroh-blobs/commit/7dccac9610482f9acbde4c46a134d99e979e6001))

## [0.31.0](https://github.com/n0-computer/iroh-blobs/compare/v0.30.0..0.31.0) - 2025-01-14

### ⚙️ Miscellaneous Tasks

- *(deps)* Bump mozilla-actions/sccache-action ([#40](https://github.com/n0-computer/iroh-blobs/issues/40)) - ([57112e6](https://github.com/n0-computer/iroh-blobs/commit/57112e62618e07a833a261b0dbfd2f64cc22eb82))
- Add project tracking ([#43](https://github.com/n0-computer/iroh-blobs/issues/43)) - ([a279ad1](https://github.com/n0-computer/iroh-blobs/commit/a279ad1bc0472fb4e47df466ab73ed9e0fa0a50a))
- Pin nextest version ([#44](https://github.com/n0-computer/iroh-blobs/issues/44)) - ([b1de3b3](https://github.com/n0-computer/iroh-blobs/commit/b1de3b306135984e113d09531beff9ed6463a778))
- Upgrade to `iroh@v0.31.0` ([#45](https://github.com/n0-computer/iroh-blobs/issues/45)) - ([2b800c9](https://github.com/n0-computer/iroh-blobs/commit/2b800c9264b21dfb73bfecbe9881bc6c07c7e0d1))

## [0.30.0](https://github.com/n0-computer/iroh-blobs/compare/v0.29.0..0.30.0) - 2024-12-17

### ⛰️  Features

- Update to new protocolhandler ([#29](https://github.com/n0-computer/iroh-blobs/issues/29)) - ([dba7850](https://github.com/n0-computer/iroh-blobs/commit/dba7850ae874939bd9a83f97c36dc6eceee7f9bd))
- Import iroh_base::ticket::BlobTicket and iroh_base::hash - ([f9d3ae1](https://github.com/n0-computer/iroh-blobs/commit/f9d3ae1e6a0cbdbdece56b0b3d948f0a3d62118c))
- [**breaking**] Update to iroh@0.30.0 ([#41](https://github.com/n0-computer/iroh-blobs/issues/41)) - ([74f1ee3](https://github.com/n0-computer/iroh-blobs/commit/74f1ee32cca396cd8e4d1cb8815b71e27c98df74))

### 🐛 Bug Fixes

- [**breaking**] Make `net_protocol` feature work without `rpc` ([#27](https://github.com/n0-computer/iroh-blobs/issues/27)) - ([4c1446f](https://github.com/n0-computer/iroh-blobs/commit/4c1446f7578778dadee6db0ad07ff025ef753779))
- Fix the task leak with the lazy in-mem rpc client while still keeping it lazy ([#31](https://github.com/n0-computer/iroh-blobs/issues/31)) - ([9ae2e52](https://github.com/n0-computer/iroh-blobs/commit/9ae2e52431ca6948ba60bca0169bba7d7cde1d06))
- Fix silent failure to add data of more than ~16MB via add_bytes or add_bytes_named ([#36](https://github.com/n0-computer/iroh-blobs/issues/36)) - ([dec9643](https://github.com/n0-computer/iroh-blobs/commit/dec96436772007178a2c9190d87598893a38b57d))

### 🚜 Refactor

- [**breaking**] Make Dialer trait private and inline iroh::dialer::Dialer ([#34](https://github.com/n0-computer/iroh-blobs/issues/34)) - ([d91b2ce](https://github.com/n0-computer/iroh-blobs/commit/d91b2ce7784cfa78fd2e6f9e0fb74f9b950a878f))
- [**breaking**] Remove the migration from redb v1 ([#33](https://github.com/n0-computer/iroh-blobs/issues/33)) - ([ae91f16](https://github.com/n0-computer/iroh-blobs/commit/ae91f16bd466f00e003d064593ea53c4c2276999))
- Simplify quinn rpc test ([#39](https://github.com/n0-computer/iroh-blobs/issues/39)) - ([60dfdbb](https://github.com/n0-computer/iroh-blobs/commit/60dfdbbacdb002621b85378449c66397d4d377f1))

### 📚 Documentation

- Add "getting started" instructions to the readme ([#32](https://github.com/n0-computer/iroh-blobs/issues/32)) - ([dd6673e](https://github.com/n0-computer/iroh-blobs/commit/dd6673e777e8f8ceab462501348941ac2387fab1))

### ⚙️ Miscellaneous Tasks

- Update rcgen to 0.13 ([#35](https://github.com/n0-computer/iroh-blobs/issues/35)) - ([57340cc](https://github.com/n0-computer/iroh-blobs/commit/57340cc931c7e0a3e8e3d14bef00e926ab7cfe47))

## [0.29.0](https://github.com/n0-computer/iroh-blobs/compare/v0.28.1..0.29.0) - 2024-12-04

### ⛰️  Features

- Update to renamed iroh-net ([#18](https://github.com/n0-computer/iroh-blobs/issues/18)) - ([b9dbf2c](https://github.com/n0-computer/iroh-blobs/commit/b9dbf2cc1e6d8a6b60f3bf9f52b832fbd23c394e))
- Update to iroh@0.29.0  - ([e9b136a](https://github.com/n0-computer/iroh-blobs/commit/e9b136ad59d8feddd16df50503bf67206cccedd9))

### 🚜 Refactor

- Avoid `get_protocol`, just keep `Arc<Blobs>` around - ([7b404f1](https://github.com/n0-computer/iroh-blobs/commit/7b404f12bca87b32af85ef0b099cc8174940219f))

### 📚 Documentation

- Add simple file transfer example - ([7ba883d](https://github.com/n0-computer/iroh-blobs/commit/7ba883d238bb2b0be8c64672369c27074519962c))

### 🧪 Testing

- Update iroh-test to 0.29 - ([cda9756](https://github.com/n0-computer/iroh-blobs/commit/cda9756d84b34583f469ffa6f9083b9b3a5fd2a5))

### ⚙️ Miscellaneous Tasks

- Prune some deps ([#12](https://github.com/n0-computer/iroh-blobs/issues/12)) - ([4c7b8e7](https://github.com/n0-computer/iroh-blobs/commit/4c7b8e79f495376245852a14688ce23a12adda85))
- Init changelog - ([acafd9e](https://github.com/n0-computer/iroh-blobs/commit/acafd9ef8fe4851854ac2a48016ebdba215f5b6b))
- Update release config - ([b621f3c](https://github.com/n0-computer/iroh-blobs/commit/b621f3c97416b61d2b7970a5c2b4ab9f5a7d9752))

### Examples

- Import examples from main repo - ([1579555](https://github.com/n0-computer/iroh-blobs/commit/1579555ba3f67102d3e4aafcf7889b558f744460))

## [0.28.1](https://github.com/n0-computer/iroh-blobs/compare/v0.28.0..v0.28.1) - 2024-11-04

### 🐛 Bug Fixes

- Use correctly patched iroh-quinn and iroh-net - ([b3c5f76](https://github.com/n0-computer/iroh-blobs/commit/b3c5f7624716896c085add70215336404188442a))

### ⚙️ Miscellaneous Tasks

- Release iroh-blobs version 0.28.1 - ([191cd2a](https://github.com/n0-computer/iroh-blobs/commit/191cd2a1c25885f8ef0d58d83df150017bc4c8bb))


