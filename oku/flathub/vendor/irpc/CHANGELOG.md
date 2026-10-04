# Changelog

All notable changes to irpc will be documented in this file.

## [0.17.0](https://github.com/n0-computer/irpc/compare/v0.16.0..0.17.0) - 2026-06-15

### ⛰️  Features

- *(deps)* [**breaking**] Update to iroh 1.0 and noq 1.0 ([#103](https://github.com/n0-computer/irpc/issues/103)) - ([c485bff](https://github.com/n0-computer/irpc/commit/c485bff025b9c37c9da53c5c4229d8749a407362))

## [0.16.0](https://github.com/n0-computer/irpc/compare/v0.15.0..0.16.0) - 2026-05-27

### ⛰️  Features

- [**breaking**] Update to iroh@1.0.0-rc.1 ([#101](https://github.com/n0-computer/irpc/issues/101)) - ([eba4551](https://github.com/n0-computer/irpc/commit/eba4551f9078d3cd7290c9a80d1e0ad31364cb79))

## [0.15.0](https://github.com/n0-computer/irpc/compare/v0.14.0..0.15.0) - 2026-05-08

### 🚜 Refactor

- Remove specialized 0rtt rpc calls ([#92](https://github.com/n0-computer/irpc/issues/92)) - ([1dd579d](https://github.com/n0-computer/irpc/commit/1dd579d92dd772c4a8fb39b5d9f68d0078666221))

### 📚 Documentation

- *(irpc-iroh)* Expand span_propagation example to work with Jaeger ([#100](https://github.com/n0-computer/irpc/issues/100)) - ([910d56e](https://github.com/n0-computer/irpc/commit/910d56e68e8aa601c42591c2f4f75f4499af4fe5))
- Update license text to match other repos - ([d758de3](https://github.com/n0-computer/irpc/commit/d758de39b4f68873f8a131d3150ffd24f4f55a70))

### ⚙️ Miscellaneous Tasks

- Migrate to rust edition 2024, align formatting with iroh standards ([#90](https://github.com/n0-computer/irpc/issues/90)) - ([56dc5a3](https://github.com/n0-computer/irpc/commit/56dc5a33344d3a73e51fcc5149602dc632bee4c6))
- Migrate to rust edition 2024, align formatting with iroh standards ([#90](https://github.com/n0-computer/irpc/issues/90)) - ([b109a95](https://github.com/n0-computer/irpc/commit/b109a9510ef20fc7c6590136e090693799063fba))
- Update to noq 1.0-rc.0 and iroh 1.0-rc.0 ([#99](https://github.com/n0-computer/irpc/issues/99)) - ([b96a6d0](https://github.com/n0-computer/irpc/commit/b96a6d0abe689cc77b3b36885ca763a8a31ead51))
- Setup changelog generation - ([67aadf1](https://github.com/n0-computer/irpc/commit/67aadf19015839d2b65f2f5082b232e03acd170c))

## [0.14.0](https://github.com/n0-computer/irpc/compare/v0.13.0..v0.14.0) - 2026-04-20

### ⛰️  Features

- [**breaking**] Update to iroh@0.98 ([#98](https://github.com/n0-computer/irpc/issues/98)) - ([7ddd7cd](https://github.com/n0-computer/irpc/commit/7ddd7cd0f4b31724f362f784bcd053f6cb90b16b))

### ⚙️ Miscellaneous Tasks

- Release - ([3bfe01c](https://github.com/n0-computer/irpc/commit/3bfe01cdba985545c6572af69cf60404124fbc68))

## [0.13.0](https://github.com/n0-computer/irpc/compare/v0.12.0..v0.13.0) - 2026-03-16

### ⛰️  Features

- [**breaking**] Switch to noq & iroh@0.97 ([#97](https://github.com/n0-computer/irpc/issues/97)) - ([e58e01c](https://github.com/n0-computer/irpc/commit/e58e01c173b20ac61dc3848c9db4e45b59e90d87))

### 🐛 Bug Fixes

- *(example)* Avoid dropping `StorageApi` guard, test the example ([#89](https://github.com/n0-computer/irpc/issues/89)) - ([92b717b](https://github.com/n0-computer/irpc/commit/92b717b274d69e4cc13591179d1fa3ae891397e6))

### 📚 Documentation

- Improve irpc-iroh examples ([#86](https://github.com/n0-computer/irpc/issues/86)) - ([24ea6eb](https://github.com/n0-computer/irpc/commit/24ea6eb10c1dc41951838b9bb2c1922400655994))
- Document the quirks of the notify fn ([#91](https://github.com/n0-computer/irpc/issues/91)) - ([1a383b8](https://github.com/n0-computer/irpc/commit/1a383b82d6f4a426ac4ed260c83041481b94c097))

### ⚙️ Miscellaneous Tasks

- Fix ci - ([ac699f3](https://github.com/n0-computer/irpc/commit/ac699f3286451c5d9888be6d2b3b0a6379a45b3f))
- Release - ([fb34e50](https://github.com/n0-computer/irpc/commit/fb34e50754f07daaf09b26a605671fba62d43683))

## [0.12.0](https://github.com/n0-computer/irpc/compare/v0.11.0..v0.12.0) - 2026-01-28

### ⚙️ Miscellaneous Tasks

- Upgrade to `iroh` at the @ `feat-multipath` branch ([#85](https://github.com/n0-computer/irpc/issues/85)) - ([71c57d7](https://github.com/n0-computer/irpc/commit/71c57d7c50d5b983ec2044ccdb0fa5d1f40ccacd))
- Release - ([a6f836f](https://github.com/n0-computer/irpc/commit/a6f836f1151a4f35f8caf6ace34487601a38e889))

## [0.11.0](https://github.com/n0-computer/irpc/compare/v0.10.0..v0.11.0) - 2025-11-06

### ⛰️  Features

- Varint feature flag ([#54](https://github.com/n0-computer/irpc/issues/54)) - ([83c9dcb](https://github.com/n0-computer/irpc/commit/83c9dcbfe000f016e9a77419472a8b717036d5c8))
- Make it easier to use existing connections with irpc. ([#79](https://github.com/n0-computer/irpc/issues/79)) - ([6f0dec7](https://github.com/n0-computer/irpc/commit/6f0dec70e387a92a8a74b34861bd476d746aa61c))
- [**breaking**] Switch from `anyhow` and `thiserror` to `n0-error` ([#78](https://github.com/n0-computer/irpc/issues/78)) - ([c7cce20](https://github.com/n0-computer/irpc/commit/c7cce20240effaa5d32bd8b509198223b55d7327))

### ⚙️ Miscellaneous Tasks

- Add license and contribution in readme. ([#80](https://github.com/n0-computer/irpc/issues/80)) - ([428988e](https://github.com/n0-computer/irpc/commit/428988e091e6972940015f3978b590ec3f4aeab5))
- Release - ([732a57f](https://github.com/n0-computer/irpc/commit/732a57fc26689da87fd5078442fc33bc54e4001c))

## [0.10.0](https://github.com/n0-computer/irpc/compare/v0.9.0..v0.10.0) - 2025-10-21

### ⚙️ Miscellaneous Tasks

- Move to syn v2 ([#76](https://github.com/n0-computer/irpc/issues/76)) - ([c53f706](https://github.com/n0-computer/irpc/commit/c53f706900aac51a822f9b8a41fe5dd5130550ac))
- Upgrade to newest iroh release ([#77](https://github.com/n0-computer/irpc/issues/77)) - ([fcd5e6a](https://github.com/n0-computer/irpc/commit/fcd5e6a54a737082f3149f0cdbb355cfcbefcca0))
- Release - ([cd72805](https://github.com/n0-computer/irpc/commit/cd72805b5da22a61d1e20af0fbc36636b99c87ca))

## [0.9.0](https://github.com/n0-computer/irpc/compare/v0.8.0..v0.9.0) - 2025-10-09

### 🐛 Bug Fixes

- Make irpc-iroh compile in wasm, and check in CI ([#66](https://github.com/n0-computer/irpc/issues/66)) - ([092b7ab](https://github.com/n0-computer/irpc/commit/092b7abd000f8ad7cee1b0e86840a1c3dcc08229))
- Set keepalive, improve listen loops ([#67](https://github.com/n0-computer/irpc/issues/67)) - ([99df1dd](https://github.com/n0-computer/irpc/commit/99df1dd210e92db03cc9aaaa94823508e6fa3cda))

### 🚜 Refactor

- Modify error types ([#69](https://github.com/n0-computer/irpc/issues/69)) - ([2cdc1a0](https://github.com/n0-computer/irpc/commit/2cdc1a0d6524fcde3645f960964b7ddbc2ba8aa4))

### ⚙️ Miscellaneous Tasks

- Release - ([b103963](https://github.com/n0-computer/irpc/commit/b1039636c9cea2989ec1a22ad4afb736a1a8b30c))
- Release prep ([#70](https://github.com/n0-computer/irpc/issues/70)) - ([e348a13](https://github.com/n0-computer/irpc/commit/e348a13b1192b84642806f51cff8d24c344e4197))
- Release - ([56a63b8](https://github.com/n0-computer/irpc/commit/56a63b810b41b118b6bf1b3e8b72b4bb9dfeeb01))

## [0.8.0](https://github.com/n0-computer/irpc/compare/v0.7.0..v0.8.0) - 2025-09-18

### ⛰️  Features

- WIP support 0rtt for notify, rpc and server_streaming ([#62](https://github.com/n0-computer/irpc/issues/62)) - ([0db3188](https://github.com/n0-computer/irpc/commit/0db3188fd6e2f5acfe304f0b8b7b64500c249201))

### 🐛 Bug Fixes

- Also set crypto provider in `configure_client_insecure` ([#57](https://github.com/n0-computer/irpc/issues/57)) - ([9ee8f35](https://github.com/n0-computer/irpc/commit/9ee8f3562f32387b6b95a5f4fa208eed7d8e2a6d))
- Do not require &mut self for try_send ([#63](https://github.com/n0-computer/irpc/issues/63)) - ([ca56cd8](https://github.com/n0-computer/irpc/commit/ca56cd85358cf7dda8cb997d8daf44006e0743b4))

### 🚜 Refactor

- Move `wrap` into separate attribute, cleanup derive macro code ([#55](https://github.com/n0-computer/irpc/issues/55)) - ([f955176](https://github.com/n0-computer/irpc/commit/f9551766f21aac49d3ac1f4596a7de5cffa8d74f))

### ⚙️ Miscellaneous Tasks

- Release - ([6a7db54](https://github.com/n0-computer/irpc/commit/6a7db54edf6d60a1fb15695b1d13e4b0da8220e3))
- Upgrade to `iroh` and `iroh-base` @ v0.92 ([#65](https://github.com/n0-computer/irpc/issues/65)) - ([be8f1de](https://github.com/n0-computer/irpc/commit/be8f1de50d796a9673d1a8ee470e4938ad990cd1))
- Release - ([10ac517](https://github.com/n0-computer/irpc/commit/10ac517c78454d008f84a70f65da89e2e0db8cf2))
- Release - ([729d070](https://github.com/n0-computer/irpc/commit/729d0706a0ce8a4b77587f27a7af8f08b738cd51))

## [0.7.0](https://github.com/n0-computer/irpc/compare/v0.6.0..v0.7.0) - 2025-07-30

### ⚙️ Miscellaneous Tasks

- Release - ([7995bac](https://github.com/n0-computer/irpc/commit/7995bac1107640e22480a980e81081a9b30543cd))
- Release - ([28a91b0](https://github.com/n0-computer/irpc/commit/28a91b0f5d27bd0b88c2e7b515ea73fe12865613))
- Release - ([43db628](https://github.com/n0-computer/irpc/commit/43db628c4e153519d1f925d387dbeaf01d44c270))

## [0.6.0](https://github.com/n0-computer/irpc/compare/v0.5.0..v0.6.0) - 2025-07-30

### ⛰️  Features

- Add Client::notify ([#52](https://github.com/n0-computer/irpc/issues/52)) - ([b363f00](https://github.com/n0-computer/irpc/commit/b363f00518389cd74722f38f3f73be96582f6940))
- Add `wrap` option to macro to optionally generate wrapper types for enum variants ([#51](https://github.com/n0-computer/irpc/issues/51)) - ([fc66b12](https://github.com/n0-computer/irpc/commit/fc66b123706f6c59c31159c2434e74f4f50a2533))

### 🚜 Refactor

- Add associated types to `Service`, use protocol enum as `Service` ([#46](https://github.com/n0-computer/irpc/issues/46)) - ([283d457](https://github.com/n0-computer/irpc/commit/283d4572830126ae668bac9fb957d718af83542c))

### ⚙️ Miscellaneous Tasks

- Release - ([1aae569](https://github.com/n0-computer/irpc/commit/1aae569741944ee7dff1c0f854794994f5615dd7))
- Point to `iroh` main ([#42](https://github.com/n0-computer/irpc/issues/42)) - ([5cc6248](https://github.com/n0-computer/irpc/commit/5cc624832cfed2653a20442851c203935039d6bc))
- `0.6.0` release prep ([#56](https://github.com/n0-computer/irpc/issues/56)) - ([b72920e](https://github.com/n0-computer/irpc/commit/b72920e6545c472f628b49c2a22cd788f6a099b2))
- Release - ([a55cddd](https://github.com/n0-computer/irpc/commit/a55cdddf020339d163a7ca24c2cf191f3e40c62f))

## [0.5.0](https://github.com/n0-computer/irpc/compare/v0.1.0..v0.5.0) - 2025-06-26

### ⛰️  Features

- *(irpc-iroh)* Add iroh ProtocolHandler, restructure example - ([9acf53d](https://github.com/n0-computer/irpc/commit/9acf53d5f9e6ae26ced92bbd2f2b5262520726c7))
- Util methods for less boilerplate - ([33c2e93](https://github.com/n0-computer/irpc/commit/33c2e9335800586455cafcd7c3c9279e9926e42c))
- Easy manual connection loop - ([d711e88](https://github.com/n0-computer/irpc/commit/d711e8884cd22424a359aa7ef3d3c55a4a41851d))
- Add a max message size restriction - ([ae6aaff](https://github.com/n0-computer/irpc/commit/ae6aaff27ced384d1c87ac65618e29ea39baf16a))

### 🐛 Bug Fixes

- Feature flags in Cargo.toml - ([95de081](https://github.com/n0-computer/irpc/commit/95de081ab2f2cef122e29d61424978a51f459d9a))
- Sender::try_send should return whether the item was sent - ([6fbb4e3](https://github.com/n0-computer/irpc/commit/6fbb4e3a964e200bfaa90a49bd4902df4bad6538))
- Build on wasm32-unknown-unknown - ([8ef52df](https://github.com/n0-computer/irpc/commit/8ef52df3b65d8ae0e34e8c9eba48e00e34f0b02f))
- Feauture dependencies - ([85e5e90](https://github.com/n0-computer/irpc/commit/85e5e905ae945289867561347a5ce91738a5b578))
- Simplify example - ([8442b03](https://github.com/n0-computer/irpc/commit/8442b0318c4753c3670c59752e54b82c5b8bf773))
- Postcard is only available in rpc module - ([740ec1d](https://github.com/n0-computer/irpc/commit/740ec1d42f22138d400b5556f9fb58daf4d5b84c))

### 🚜 Refactor

- Use n0_future::boxed types - ([e30608e](https://github.com/n0-computer/irpc/commit/e30608ed3dedb4bff12601b09ed2135b40e96e5d))
- [**breaking**] Make `Sender` clone ([#25](https://github.com/n0-computer/irpc/issues/25)) - ([8db0017](https://github.com/n0-computer/irpc/commit/8db0017762b916a26b439884b204373e037953ba))

### 📚 Documentation

- Add docs - ([983db0a](https://github.com/n0-computer/irpc/commit/983db0a2f3df16d262e7a78832c30ce392542f09))
- Add - ([f9d7c38](https://github.com/n0-computer/irpc/commit/f9d7c384c5b2da38c5289987d0a70323a386b2dd))
- Improve bench example - ([07cfbaf](https://github.com/n0-computer/irpc/commit/07cfbafe734a2c2cef2fface496ede67fb783f48))
- Add docs for read_request - ([ad87a99](https://github.com/n0-computer/irpc/commit/ad87a990a2470e0bb9ebb89386f708f486bd13d7))

### ⚙️ Miscellaneous Tasks

- *(irpc, iroh-irpc)* Updates to latest iroh-quinn release and iroh v0.90.0 ([#38](https://github.com/n0-computer/irpc/issues/38)) - ([dee863e](https://github.com/n0-computer/irpc/commit/dee863e80da9a0283332ba8e0732ed1fe38ca388))
- Release 0.2.0 - ([1ae8b75](https://github.com/n0-computer/irpc/commit/1ae8b75fe6b20137ce847f40288095aa020b936c))
- Fmt - ([d2f78af](https://github.com/n0-computer/irpc/commit/d2f78af59c53c3b897e36882f643fe3c631693e6))
- Clippy - ([5b95bbe](https://github.com/n0-computer/irpc/commit/5b95bbee6e0c9746c8692c3ac43e6f26f4682318))
- Expand CI workflows to build no-default-features and wasm - ([86f0bd0](https://github.com/n0-computer/irpc/commit/86f0bd04bbe2c68e3a3f926e77fc08dc1232c2b9))
- Bump iroh to 0.35.0 - ([af100b9](https://github.com/n0-computer/irpc/commit/af100b9cf4ad5c102dc2550c97f27f6103fa427e))
- Update Cargo.lock - ([4e13805](https://github.com/n0-computer/irpc/commit/4e138054cc634d9b4730bae68a29aff46b85b06a))
- Clippy - ([7fe5de4](https://github.com/n0-computer/irpc/commit/7fe5de48167dd4b16d41bd56ae7a12fbb70cd9fc))
- Update clippy ([#37](https://github.com/n0-computer/irpc/issues/37)) - ([9e0a16c](https://github.com/n0-computer/irpc/commit/9e0a16c5e68eaa6d854b79aded3b3acc97ca9ee6))
- Release - ([cd8039c](https://github.com/n0-computer/irpc/commit/cd8039ca8d58b9cb32183a455d6274c8156463b6))

## [0.1.0] - 2025-04-03

### ⛰️  Features

- *(ci)* Add minimal crates version check - ([00b6d12](https://github.com/n0-computer/irpc/commit/00b6d1203e31a57c63f21fc59f8358fc3cdbfd42))
- *(http2)* Shut down the hyper server on drop. - ([124591a](https://github.com/n0-computer/irpc/commit/124591a336df5399cb102da99cfb213159947317))
- *(http2)* Shut down the hyper server on drop - ([cd86839](https://github.com/n0-computer/irpc/commit/cd868396ffd73bda310971ff59624e3d68ba8a3e))
- *(http2)* Move serialization and deserialization closer to the user - ([c1742a5](https://github.com/n0-computer/irpc/commit/c1742a5cb6698e8067844a6944fb5d07bfd3585d))
- *(http2)* Add config to http2 channel - ([8a98ba7](https://github.com/n0-computer/irpc/commit/8a98ba7cb4db3e258ae65f0c568e8524fc26fb4a))
- Add optional macros to reduce boilerplate - ([0010a98](https://github.com/n0-computer/irpc/commit/0010a9861e956a31fe609e84d2da3569c535eb42))
- Allow to split RPC types into a seperate crate. - ([1564ad8](https://github.com/n0-computer/irpc/commit/1564ad81ec2333732ce7268970ca76ced2ea59a5))
- Generate code only if needed and add client - ([e4d6d91](https://github.com/n0-computer/irpc/commit/e4d6d91d3006666ef0e733b8cf97bc8c51d1bb79))
- Better docs for macro-generated items - ([bbf8e97](https://github.com/n0-computer/irpc/commit/bbf8e97e9aafbe74eb1ee60b0fe3bfd997394bf3))
- Add convenience function for handling errors in rpc calls - ([26283b1](https://github.com/n0-computer/irpc/commit/26283b1ebc2c68d0e4847856d1b07ae55fd7036e))
- Lazy connections, take 2 - ([3ed491e](https://github.com/n0-computer/irpc/commit/3ed491ed3857e9ae495144df696b968d342ac91e))
- More efficient channel replacement - ([6d5808d](https://github.com/n0-computer/irpc/commit/6d5808df40a7ec008d79ab7f8553cc0253a31c33))
- Allow lazy connect in client of split example - ([97b7e1a](https://github.com/n0-computer/irpc/commit/97b7e1af7c517a70c270418a4d9c18bd58440461))
- Implement std::error::Error for combined channel errors - ([d7828e3](https://github.com/n0-computer/irpc/commit/d7828e31395dd9a096c4afc128eae03357c4db2e))
- Add minimal tracing support - ([850bde1](https://github.com/n0-computer/irpc/commit/850bde116a93d881294e7d80ac007137d075ba61))
- Add minimal tracing support - ([2de50d5](https://github.com/n0-computer/irpc/commit/2de50d52dcefd3fb1e5d557f7ef8f170481038e8))
- Make channels configurable - ([11d3071](https://github.com/n0-computer/irpc/commit/11d30715fc7912ba08e755a57138bc36cf675bfc))
- Expose local address of ServerChannel - ([b67dcdb](https://github.com/n0-computer/irpc/commit/b67dcdb87f50a349a9977c1dc662d321f3ac68d2))
- Add dummy endpoint - ([48cf5db](https://github.com/n0-computer/irpc/commit/48cf5db8af9d21418bc3b306d9b27dd27c6e8146))
- Add ability to creaete a RpcClient from a single connection - ([7a4f40f](https://github.com/n0-computer/irpc/commit/7a4f40f2ddaea0e858c12a882a19302c9c3ca853))
- Add AsRef and into_inner for RpcClient and RpcServer - ([ea8e119](https://github.com/n0-computer/irpc/commit/ea8e1195fda510b470feaf638f519f185151e8d6))
- Update quinn and rustls - ([20679f9](https://github.com/n0-computer/irpc/commit/20679f938d1c257cb51d64f46ba39cc46c580a73))
- Make wasm compatible ([#49](https://github.com/n0-computer/irpc/issues/49)) - ([6cbf62b](https://github.com/n0-computer/irpc/commit/6cbf62b2fdf150dca6a261dfdb16e338c7bd7cd0))
- Add additional `Sync` bounds to allow for better reuse of streams - ([54c4ade](https://github.com/n0-computer/irpc/commit/54c4adeef2c851cbe2e6ac542d221b6f020a430c))
- Add additional `Sync` bounds to allow for better reuse of streams ([#68](https://github.com/n0-computer/irpc/issues/68)) - ([bc589b7](https://github.com/n0-computer/irpc/commit/bc589b7a49e277b6847cb01675e92984152d033f))
- Allow to compose RPC services ([#67](https://github.com/n0-computer/irpc/issues/67)) - ([77785a2](https://github.com/n0-computer/irpc/commit/77785a21babe4e56d28541d1b3ba401dcf366441))
- Update and cleanup deps ([#80](https://github.com/n0-computer/irpc/issues/80)) - ([eba3a06](https://github.com/n0-computer/irpc/commit/eba3a06a1c8a8ac2f74b1e6dde02220f935e9be7))
- WIP First somewhat working version of attribute macro to declare services - ([bab7fe0](https://github.com/n0-computer/irpc/commit/bab7fe083fdaf6ee81c4f17f2b96a5e1f9d18011))
- Upgrade iroh-quinn to 0.12.0 ([#109](https://github.com/n0-computer/irpc/issues/109)) - ([a5fecdc](https://github.com/n0-computer/irpc/commit/a5fecdcd3b60d581328106dc79246a4b2b609709))
- Accept handler ([#116](https://github.com/n0-computer/irpc/issues/116)) - ([32d5bc1](https://github.com/n0-computer/irpc/commit/32d5bc1a08609f4f0b5650980088f07d81971a55))
- Use postcard encoding for all transports that require serialization ([#114](https://github.com/n0-computer/irpc/issues/114)) - ([badb606](https://github.com/n0-computer/irpc/commit/badb6068db23e6262c183ef8981228fd8ca1ef61))
- Update to iroh@0.29.0 - ([02b2559](https://github.com/n0-computer/irpc/commit/02b25594bbfd210acb29b65c59d80c3063565193))
- Update to iroh@0.29.0 ([#125](https://github.com/n0-computer/irpc/issues/125)) - ([07f1335](https://github.com/n0-computer/irpc/commit/07f1335f1616359b93ee60277a80a07df7552e18))

### 🐛 Bug Fixes

- *(ci)* Cancel stale repeat jobs ([#64](https://github.com/n0-computer/irpc/issues/64)) - ([d9b385c](https://github.com/n0-computer/irpc/commit/d9b385ce8ba66430c6a2744c0f595a74a9e3d578))
- *(http2)* Don't log normal occurrences as errors - ([a4e76da](https://github.com/n0-computer/irpc/commit/a4e76da0fbc53381a9d97fbaf3df336b2bc04e0d))
- Consistent naming - ([a23fc78](https://github.com/n0-computer/irpc/commit/a23fc784f30a9eae6314f4b2cc6151dcfae7b215))
- Add docs to macro - ([c868bc8](https://github.com/n0-computer/irpc/commit/c868bc8c297ae9bb3cae5ba467e703a8bf1b135f))
- Improve docs - ([1957794](https://github.com/n0-computer/irpc/commit/19577949db0becd2da2dc0d206bbb1a91daa5722))
- Docs typos - ([a965fce](https://github.com/n0-computer/irpc/commit/a965fceba8558e144f15fb338d1d7586453fbfb5))
- Derive Debug for generated client - ([9e72faf](https://github.com/n0-computer/irpc/commit/9e72faf76746f7e7cdc3e76d04a7c87c3bf03a8c))
- Rename macro to rpc_service - ([bdb71c1](https://github.com/n0-computer/irpc/commit/bdb71c1659a58a41f11393e1c18f58e8f74e6206))
- Hide the exports also behind feature flags - ([4cff83d](https://github.com/n0-computer/irpc/commit/4cff83dd9741e4fd3ae8982f96e203839dd17ec4))
- Get rid of get rid of channel!!! - ([e0b504d](https://github.com/n0-computer/irpc/commit/e0b504de6c81054996aa6271f1bae319046d405f))
- Add additional framing to messages - ([8a6038e](https://github.com/n0-computer/irpc/commit/8a6038e6f478f7782158c7a0e771672b9bf9722b))
- Do buffering in the forwarder - ([4e3d9fd](https://github.com/n0-computer/irpc/commit/4e3d9fd841396aaf0cb4024017605cef19b2cacd))
- Add flume dependency to quinn-transport - ([e64ba0b](https://github.com/n0-computer/irpc/commit/e64ba0b4e6725d4b1800fdd2c4b12bd1b39a97f8))
- Call wait_idle on endpoint drop to allow for time for the close msg to be sent - ([7ba3bee](https://github.com/n0-computer/irpc/commit/7ba3bee8f6fcc92f761964582f684072fb8a1bd0))
- Update MSRV to 1.65 - ([3cb7870](https://github.com/n0-computer/irpc/commit/3cb7870ffc1783290d536c74fe7d5481aa935a8b))
- Explicitly close channel when client is dropped - ([2c81d23](https://github.com/n0-computer/irpc/commit/2c81d2307d994790ca67a7758953b748133c6655))
- Do not use macros in tests - ([596a426](https://github.com/n0-computer/irpc/commit/596a426e20fd1e5ada2eebc2c2256d202700dc5d))
- Do two forwarder tasks - ([2e334f3](https://github.com/n0-computer/irpc/commit/2e334f345d5d2a82bb425c993478772e63d11dfe))
- Add explicit proc-macro dependency so the minimal-versions test works - ([cf2045c](https://github.com/n0-computer/irpc/commit/cf2045c6b7f25ddf84c1e99776cfcec60eae5778))
- Make socket name os comaptible - ([ec3314c](https://github.com/n0-computer/irpc/commit/ec3314c8410a07aa66cf2b1792262902b94caa95))
- Nightly failures ([#66](https://github.com/n0-computer/irpc/issues/66)) - ([865622e](https://github.com/n0-computer/irpc/commit/865622e99c0618dca16e530b5a44fe3f1f2bdced))
- Rpc client concurrently waits for requests and new connections ([#62](https://github.com/n0-computer/irpc/issues/62)) - ([3323574](https://github.com/n0-computer/irpc/commit/3323574c972dbdf4dc4e9ae81ab8f32d27b7f3c2))
- Try to make client streaming and bidi streaming work - ([2bb27d0](https://github.com/n0-computer/irpc/commit/2bb27d0b9ae912203f3292c527863e8203bbc619))
- Downgrade derive_more to non beta - ([9235fdf](https://github.com/n0-computer/irpc/commit/9235fdfe0efc4cbcd9694c248d1f112f32000666))
- Transport:quinn only spawn when tokio is available ([#95](https://github.com/n0-computer/irpc/issues/95)) - ([baa4f83](https://github.com/n0-computer/irpc/commit/baa4f837f161ecd80c3a6d46fad1788aadf06049))
- Add back bytes dependency for quinn transport ([#127](https://github.com/n0-computer/irpc/issues/127)) - ([1778e1d](https://github.com/n0-computer/irpc/commit/1778e1d9523ade44a670f11d4263f8b9e1115b0c))
- Fuse oneshot channel ([#147](https://github.com/n0-computer/irpc/issues/147)) - ([03ad748](https://github.com/n0-computer/irpc/commit/03ad748437d8f1fa10874f4e893fa5ce39496c5f))

### 🚜 Refactor

- *(http2)* Use a slice instead of a vec for the local addr - ([0d79990](https://github.com/n0-computer/irpc/commit/0d79990aacf48c1083aed30a3718ea35fb3225be))
- *(mem)* Return &[LocalAddr::Mem] directly - ([2004c46](https://github.com/n0-computer/irpc/commit/2004c461cfbbc6c3c4b0caf8c953f810accb10b1))
- Move more code out of the macro - ([796dccd](https://github.com/n0-computer/irpc/commit/796dccd183904659c8b2bfacf0ee33dae4967790))
- One less &mut - ([53af0f9](https://github.com/n0-computer/irpc/commit/53af0f90cd9288a6a27d25558495b9f1091698d3))
- Add some mut again - ([aab91dc](https://github.com/n0-computer/irpc/commit/aab91dc99efcbc472c405ad9e7862fc24723b8ee))
- Error remodeling - ([6bad622](https://github.com/n0-computer/irpc/commit/6bad6228be19c111e052ea30f4334c469173414b))
- Make lazy client a separate example - ([e92771e](https://github.com/n0-computer/irpc/commit/e92771ecbb7aad5c9442396bb351984f86ee94fa))
- Make the lazy example simpler - ([86c2b94](https://github.com/n0-computer/irpc/commit/86c2b942c8328a0edd6154574f22936518aabed1))
- Round and round we go - ([37a5703](https://github.com/n0-computer/irpc/commit/37a5703f09a36fa5587bb7a92e82133a588b41bf))
- Remove dead code - ([7d7ac15](https://github.com/n0-computer/irpc/commit/7d7ac154e5c7149318628e41887cd07838a52438))
- Add ClientChannelInner - ([8aafe34](https://github.com/n0-computer/irpc/commit/8aafe347c5390fb82f4e1033d389e3408a3d3ef9))
- WIP make both server and client side take quinn endpoints - ([4d99d71](https://github.com/n0-computer/irpc/commit/4d99d712448ef17719255f2bd9c762c4a4e8b2f4))
- WIP make benches work - ([639883c](https://github.com/n0-computer/irpc/commit/639883c7866d8de2149c932c62ab0e9638ed28e8))
- Make the par bench work without the multithreaded executor - ([ee62f93](https://github.com/n0-computer/irpc/commit/ee62f9383c98f3705e50a34cd52b1053cc33d366))
- WIP add substream source - ([6070939](https://github.com/n0-computer/irpc/commit/6070939fb36d9ed5a5dea9de5f301bc44f5e930d))
- WIP channel source - ([45fb792](https://github.com/n0-computer/irpc/commit/45fb792fbd226ee3dc851d8202d0b6df371adb2b))
- Combined has won - ([b743bf1](https://github.com/n0-computer/irpc/commit/b743bf138ad0ac600556fff692386ed4fce78610))
- Move some things around - ([3ceade3](https://github.com/n0-computer/irpc/commit/3ceade3d4025b2d3f257a955f998e3892962f066))
- Get rid of some boxes - ([ff786ab](https://github.com/n0-computer/irpc/commit/ff786ab152812a85b0907a5ea8504b6eda292ccb))
- Rename transports to the crate they wrap - ([bd65fe6](https://github.com/n0-computer/irpc/commit/bd65fe6b3f23dc336150af817c821c9548f4c1e7))
- Get rid of the lifetimes - ([6ea4862](https://github.com/n0-computer/irpc/commit/6ea486296647ae6d7eb13c784fc2afc931e99849))
- Rename the macros to declare_... - ([ae24dd1](https://github.com/n0-computer/irpc/commit/ae24dd11e44ce8a09cc8617c47a563c52be08e6b))
- Make macros optional, and also revert the trait name changes - ([884ceed](https://github.com/n0-computer/irpc/commit/884ceed646231a413c1f011068d3bcbb415b18fc))
- Remove all transports from the defaults - ([3c02ee5](https://github.com/n0-computer/irpc/commit/3c02ee5f3058539365bd7d588c1610fb7d8ea050))
- Use spans - ([15be738](https://github.com/n0-computer/irpc/commit/15be73800c511180834e8712c24121bd473e5edd))
- Add mapped methods for client and server - ([58b029e](https://github.com/n0-computer/irpc/commit/58b029ed2022040e0eb924d80884f521ed76c195))
- Better generics with helper trait - ([b41c76a](https://github.com/n0-computer/irpc/commit/b41c76ae2948341a37d97a2296fb4b9dc421a9a9))
- Naming - ([ee5272a](https://github.com/n0-computer/irpc/commit/ee5272af1040223152e2750d8680a0d128b1afd6))
- No more futures crate ([#73](https://github.com/n0-computer/irpc/issues/73)) - ([403fab0](https://github.com/n0-computer/irpc/commit/403fab014dea45b5d58978d9d4b8a9c80e145c1f))
- Fix hyper and combined transports - ([a56099e](https://github.com/n0-computer/irpc/commit/a56099e8e557776d0dc41f6c45006f879fb88f05))
- [**breaking**] Use `Service` generic in transport connections ([#76](https://github.com/n0-computer/irpc/issues/76)) - ([64ed5ef](https://github.com/n0-computer/irpc/commit/64ed5efea314a785ed9890fb78f857dadba3dc85))
- Use two stage accept - ([ac8f358](https://github.com/n0-computer/irpc/commit/ac8f358e0ea623a9906402639a0882794b0a06d0))
- Use two stage everywhere - ([b3c37ff](https://github.com/n0-computer/irpc/commit/b3c37ff88de533c6edb9c457a8c5ddd1f713bbf9))
- Use two stage accept ([#87](https://github.com/n0-computer/irpc/issues/87)) - ([c2520b8](https://github.com/n0-computer/irpc/commit/c2520b85f5fd37b78bd0fc4f87c2989605209bac))
- Remove the interprocess transport - ([bd72cdc](https://github.com/n0-computer/irpc/commit/bd72cdcceba82fe9953413e050d86e2f528f93ea))
- Rename iroh-net transport to iroh-transport - ([7ee875b](https://github.com/n0-computer/irpc/commit/7ee875b5a1cdf2bbfa377564d7a3c1792876f780))
- Rename iroh-net transport to iroh-transport ([#123](https://github.com/n0-computer/irpc/issues/123)) - ([69e7c4a](https://github.com/n0-computer/irpc/commit/69e7c4a4f1c90533db61c32eb7145073f0bb1659))

### 📚 Documentation

- *(http2)* Add comments for the new error cases - ([103b8f4](https://github.com/n0-computer/irpc/commit/103b8f400c39369710f110cbf9463813858708ff))
- Better comments - ([c7de505](https://github.com/n0-computer/irpc/commit/c7de505a8730e8cd213d20f6d072d9d7fa61e0f7))
- Yet another badge - ([c0c1ac3](https://github.com/n0-computer/irpc/commit/c0c1ac3a740ac2b5e28a8173329f8a7a2790f57f))
- Fix github badge - ([60c511f](https://github.com/n0-computer/irpc/commit/60c511f8d8afefaa35c5fff1011a3030bc1db7c0))
- Update todo comments and made some other comments nicer - ([311307c](https://github.com/n0-computer/irpc/commit/311307cb8acb9a7ca5a4c97adb1e7193a3c17c95))
- Update docs to match implementation - ([7b6bf32](https://github.com/n0-computer/irpc/commit/7b6bf325884c7f6843fd7656b269aa9b2506b2b0))
- Add some more text to the readme about why this thing exists in the first place - ([a512de5](https://github.com/n0-computer/irpc/commit/a512de5e80e2e7f19e14cfaf01873407247237aa))
- Better docs for the declare macros - ([ffc934c](https://github.com/n0-computer/irpc/commit/ffc934c7f2b79cf76565f52e135d14e3c0d637ac))

### ⚡ Performance

- Avoid a copy - ([b57564f](https://github.com/n0-computer/irpc/commit/b57564f3ee9cdbf9fcc1a70965484c40dfae2a40))
- Preallocate small buffer - ([e306eba](https://github.com/n0-computer/irpc/commit/e306ebaf46e69e6b0f5ed086559900ff4b64dc4b))

### 🎨 Styling

- Fmt - ([0152170](https://github.com/n0-computer/irpc/commit/01521701db70a14f11ba2e5937cd30a72d3e3b12))

### 🧪 Testing

- *(http2)* Add some tests for the not so happy path - ([c04cf77](https://github.com/n0-computer/irpc/commit/c04cf7790ed92ca68129c95f26cae0c3136333a6))
- Adapt examples - ([80f4921](https://github.com/n0-computer/irpc/commit/80f4921f959b9363204443c39aa81ba83a875c6e))

### ⚙️ Miscellaneous Tasks

- *(docs)* Enable all features for docs.rs builds - ([d3f55ce](https://github.com/n0-computer/irpc/commit/d3f55ced941448de4e3b571ae6bdf6bc3942d4fb))
- *(docs)* Enable all features for docs.rs builds ([#60](https://github.com/n0-computer/irpc/issues/60)) - ([e063747](https://github.com/n0-computer/irpc/commit/e063747f5eb47cde022845e1b2cecb5426b823c1))
- Fmt - ([20bb7a0](https://github.com/n0-computer/irpc/commit/20bb7a01cfb7c019bd91106e51ffb6e5acc0ad88))
- Rename main structs to include type - ([d61bf8d](https://github.com/n0-computer/irpc/commit/d61bf8d09f51d2df4728d4f76dbe69626ca9d0ac))
- Configure rust version and check during CI - ([da6f282](https://github.com/n0-computer/irpc/commit/da6f2827229514946a4c800c085c956e195fec44))
- Add more up to date n0 ci workflow - ([7adeaec](https://github.com/n0-computer/irpc/commit/7adeaec832ebf0e1da46f963d29f9cf16854c518))
- Update flume - ([637f9f2](https://github.com/n0-computer/irpc/commit/637f9f28a917b01a6db1459042466a3bdb3dde66))
- Update flume - ([c966283](https://github.com/n0-computer/irpc/commit/c96628305b6463f31db64ab6943317f2ca58c976))
- Clippy ([#61](https://github.com/n0-computer/irpc/issues/61)) - ([b25d30d](https://github.com/n0-computer/irpc/commit/b25d30d6749c7508cf3e2e425703be53fd52c49e))
- Fmt - ([63bc8d8](https://github.com/n0-computer/irpc/commit/63bc8d882453f45ee51187bca9e1399a928d417a))
- Fix feature flags for tests - ([9c4a7e6](https://github.com/n0-computer/irpc/commit/9c4a7e69b186c84d16464de55b4d80baab73c41b))
- Clippy - ([1652d5f](https://github.com/n0-computer/irpc/commit/1652d5fb39e464ad8929861614ad1a3153d3feea))
- Fix feature flags for tests ([#69](https://github.com/n0-computer/irpc/issues/69)) - ([488bb8c](https://github.com/n0-computer/irpc/commit/488bb8c62850bd1cc74eac7303d820f24c0a9151))
- Clippy - ([38601de](https://github.com/n0-computer/irpc/commit/38601de6e19e52723b30a200d1c22621c1d772f2))
- Update derive-more - ([78a3250](https://github.com/n0-computer/irpc/commit/78a32506214cfa564c06fac4f468952c22734c0c))
- Depend on iroh-quinn and bump version ([#75](https://github.com/n0-computer/irpc/issues/75)) - ([df54382](https://github.com/n0-computer/irpc/commit/df5438284ec30b5b5b527932b39a48659e4518e5))
- Fix merge issue - ([4e8deef](https://github.com/n0-computer/irpc/commit/4e8deef7c02d0e1b3a8dbfc1d5484636eadd9a21))
- Update Cargo.lock - ([89f1b65](https://github.com/n0-computer/irpc/commit/89f1b65718b00f8a3880bc6f799d308693c88507))
- Release - ([92f472b](https://github.com/n0-computer/irpc/commit/92f472bf34611cff39f238e7b0f45ca65287b9f8))
- Release - ([af9272b](https://github.com/n0-computer/irpc/commit/af9272b17a896056d5f02565e4090ecfdbe000b8))
- Release - ([e0b50a1](https://github.com/n0-computer/irpc/commit/e0b50a1d1e1f0b5865b1aff01d43ac4920ae3b44))
- Release ([#82](https://github.com/n0-computer/irpc/issues/82)) - ([3b01e85](https://github.com/n0-computer/irpc/commit/3b01e856822aca75126fbdd8ae640f2a440f80e8))
- Upgrade to Quinn 0.11 and Rustls 0.23 - ([220aa35](https://github.com/n0-computer/irpc/commit/220aa35b69e178361c53567f37416c0852593cbd))
- Upgrade to Quinn 0.11 and Rustls 0.23 - ([1c7e3c6](https://github.com/n0-computer/irpc/commit/1c7e3c6d98d38d92a13253486b565260b89686ba))
- Upgrade to Quinn 0.11 and Rustls 0.23 - ([2221339](https://github.com/n0-computer/irpc/commit/2221339d5e98cb2f8952b303c5ead24c8030f1f8))
- Upgrade to Quinn 0.11 and Rustls 0.23 ([#92](https://github.com/n0-computer/irpc/issues/92)) - ([93e64ab](https://github.com/n0-computer/irpc/commit/93e64ab904922a1879f6dcf58dca763b4d038070))
- Release - ([50dde15](https://github.com/n0-computer/irpc/commit/50dde1542ad12b24154eadb5c8bf7d713d79495f))
- Release ([#93](https://github.com/n0-computer/irpc/issues/93)) - ([9066a40](https://github.com/n0-computer/irpc/commit/9066a403feed8277503d6f2a512a834b7fcafef7))
- Release - ([10a16f7](https://github.com/n0-computer/irpc/commit/10a16f72a3aca254259879bba611d099f9e9edf5))
- Release ([#96](https://github.com/n0-computer/irpc/issues/96)) - ([277cde1](https://github.com/n0-computer/irpc/commit/277cde1fec1a341f35ed9b1ad5d9eda252c6ee9d))
- Remove direct rustls dependency - ([f67c218](https://github.com/n0-computer/irpc/commit/f67c2189c1c8a8b8fa9f902877a74d64a38e994c))
- Remove direct rustls dependency ([#94](https://github.com/n0-computer/irpc/issues/94)) - ([fe08b15](https://github.com/n0-computer/irpc/commit/fe08b157ae162ec71ca8ad77efea300132abce77))
- New version for quic-rpc-derive as well ([#104](https://github.com/n0-computer/irpc/issues/104)) - ([39f5b20](https://github.com/n0-computer/irpc/commit/39f5b2014d48300f4308a6451cb378725c9926c0))
- Release - ([64e0a7d](https://github.com/n0-computer/irpc/commit/64e0a7d1a9e9127fe8b6449964af10984097f186))
- Release - ([be04be1](https://github.com/n0-computer/irpc/commit/be04be152a2be26cc6752e76e947b50f3b5da958))
- Consistently format imports... - ([33fb08b](https://github.com/n0-computer/irpc/commit/33fb08b417874bfdce79c8a5d3972aee1ca7ba8b))
- Consistently format imports... ([#113](https://github.com/n0-computer/irpc/issues/113)) - ([08750c5](https://github.com/n0-computer/irpc/commit/08750c54cd82295e5819eeefd18d9f904fc51a02))
- Introduce a .rustfmt.toml file with configs for  automatic formatting ([#115](https://github.com/n0-computer/irpc/issues/115)) - ([a949899](https://github.com/n0-computer/irpc/commit/a949899deac2f626c03452028a86f9420bc93530))
- Update rcgen - ([27287e1](https://github.com/n0-computer/irpc/commit/27287e13fa125234898d9aabd7d9d640aba92a36))
- Update rcgen ([#118](https://github.com/n0-computer/irpc/issues/118)) - ([2e1daa9](https://github.com/n0-computer/irpc/commit/2e1daa91552f99c448f6508fb55630f2933ee705))
- Prune some deps ([#119](https://github.com/n0-computer/irpc/issues/119)) - ([dc75b95](https://github.com/n0-computer/irpc/commit/dc75b951bcd6b3b2239ab7a71e2fedcd12152853))
- Remove `cc` version pinning - ([6da6783](https://github.com/n0-computer/irpc/commit/6da6783ca95f90e38f22091d0d5c8e6b13f6a3ec))
- Remove `cc` version pinning ([#122](https://github.com/n0-computer/irpc/issues/122)) - ([a5606c2](https://github.com/n0-computer/irpc/commit/a5606c260275d433f00c3aed2fb57ed082900c38))
- Release 0.16.0 ([#124](https://github.com/n0-computer/irpc/issues/124)) - ([a19ce1b](https://github.com/n0-computer/irpc/commit/a19ce1be542382a65ca7c56843de49a108bf21db))
- Publish 0.17.0 - ([b105da2](https://github.com/n0-computer/irpc/commit/b105da2f4f8786e1830bdf49a2369f37330ce6dd))
- Release v0.17.1 ([#128](https://github.com/n0-computer/irpc/issues/128)) - ([ef57dc1](https://github.com/n0-computer/irpc/commit/ef57dc1695bbe94c315d8c8684790154eb185fe0))
- Release 0.17.2 ([#132](https://github.com/n0-computer/irpc/issues/132)) - ([e76144a](https://github.com/n0-computer/irpc/commit/e76144a9901e581f9396b00c6fac1728bfb5de5a))
- Release v0.17.3 ([#134](https://github.com/n0-computer/irpc/issues/134)) - ([5c478c2](https://github.com/n0-computer/irpc/commit/5c478c25a746760b0677c73b1f670e4da5e942da))
- Add project tracking ([#137](https://github.com/n0-computer/irpc/issues/137)) - ([a45bd07](https://github.com/n0-computer/irpc/commit/a45bd073ee4ec4a8fd1fbdf517b0fb08a488315e))
- Upgrade to `iroh@v0.31.0` ([#138](https://github.com/n0-computer/irpc/issues/138)) - ([4d1e5e6](https://github.com/n0-computer/irpc/commit/4d1e5e6aff58a29e8e5b8a4639eaa362453e9831))
- Release - ([8eb1caa](https://github.com/n0-computer/irpc/commit/8eb1caad598933c0b01514bcd2804d3b25a835aa))
- Remove individual repo project tracking ([#140](https://github.com/n0-computer/irpc/issues/140)) - ([c605285](https://github.com/n0-computer/irpc/commit/c605285d126a3a237bff0810c9b78ac1f289e9e7))
- Upgrade to latest iroh ([#141](https://github.com/n0-computer/irpc/issues/141)) - ([dc34601](https://github.com/n0-computer/irpc/commit/dc346011e7272a4ed44e224dc652a9b7f1348d19))
- Release - ([db8c5e1](https://github.com/n0-computer/irpc/commit/db8c5e1abd089d9d70d0a1cc76de4ef13959751d))
- Release - ([4cb98ba](https://github.com/n0-computer/irpc/commit/4cb98ba5675c0078cb68554b0be8ad5ed524356a))
- Patch crates to use main branch of iroh dependencies ([#144](https://github.com/n0-computer/irpc/issues/144)) - ([90db0ae](https://github.com/n0-computer/irpc/commit/90db0aef7c1fe2245286f52270f39b505560acb3))
- Upgrade to `iroh@v0.33 ([#145](https://github.com/n0-computer/irpc/issues/145)) - ([44240d0](https://github.com/n0-computer/irpc/commit/44240d0cc5d590c63abbf7e7578bfc10f4b9e098))
- Release - ([ec27edb](https://github.com/n0-computer/irpc/commit/ec27edbc1cba730d8bf54a61be06ffc62df3948e))
- Patch to use main branch of iroh dependencies ([#146](https://github.com/n0-computer/irpc/issues/146)) - ([c47fef2](https://github.com/n0-computer/irpc/commit/c47fef2a0cf5f94643e45034b15f9ccababf6a79))
- Release - ([8dae62c](https://github.com/n0-computer/irpc/commit/8dae62ced69d3e120ad2b1306442dafe3b5235ff))
- Release ([#148](https://github.com/n0-computer/irpc/issues/148)) - ([a5f8a50](https://github.com/n0-computer/irpc/commit/a5f8a50b3d81c264dafcba4e12492a6884194265))
- Upgrade to latest `iroh` ([#150](https://github.com/n0-computer/irpc/issues/150)) - ([7293b12](https://github.com/n0-computer/irpc/commit/7293b12bf494868bcf06ec201ee4f29136fb7da5))
- Release - ([9df2837](https://github.com/n0-computer/irpc/commit/9df2837a6ad73bb3f31cebac9931f27c50cde695))
- Release - ([3e2b3ee](https://github.com/n0-computer/irpc/commit/3e2b3ee982c934ecb3527bb163be4a8570f62483))

### Fix

- Typos - ([b39a1ac](https://github.com/n0-computer/irpc/commit/b39a1ac757add613787a6d66174733bc2f168251))

### Change

- Improve split macro example - ([7d5dc82](https://github.com/n0-computer/irpc/commit/7d5dc82da29933fd922343e0722bab17e1011f5a))

### Cleanup

- Move socket name generation into the lib - ([4f40732](https://github.com/n0-computer/irpc/commit/4f40732e33e812d40b97c3d031c3230096bd9ff9))

### Pr

- *(http2)* Log remote addr again - ([5388bb2](https://github.com/n0-computer/irpc/commit/5388bb2daf69aaeac4b1038c3f7a40937eec16dc))
- Make ChannelConfigError a proper error - ([e4a548b](https://github.com/n0-computer/irpc/commit/e4a548b64fb97935a607d90f7da98d64ed89d8c5))
- Rename extra constructors and add some comments - ([2c9a08b](https://github.com/n0-computer/irpc/commit/2c9a08bf5b0d2fcd62d93374862932a5c717af2f))

### Ref

- Rename main structs not conflict with trait names - ([6fba32a](https://github.com/n0-computer/irpc/commit/6fba32a4ecf67cb4700f96005c4519f3a9d5bd5b))

### Wip

- Modularize example - ([1782411](https://github.com/n0-computer/irpc/commit/17824114e5ddd50c3bfd590e884e977f401e6e0b))
- Better approach - setup - ([92b9b60](https://github.com/n0-computer/irpc/commit/92b9b60beeb933bc85a8f977242336a504b735f6))
- Easier generics - ([6d710b7](https://github.com/n0-computer/irpc/commit/6d710b74cf45d8a04cbbcd9d803718b9aceb8012))


