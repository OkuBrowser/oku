# Changelog

## [1.0.0-rc.0](https://github.com/n0-computer/n0-error/compare/v0.1.3..v1.0.0-rc.0) - 2026-05-06

### 🐛 Bug Fixes

- Configure git identity in cleanup workflow ([#38](https://github.com/n0-computer/n0-error/issues/38)) - ([44113ca](https://github.com/n0-computer/n0-error/commit/44113cac595bdeaf30b1c6583ba6690754ea3a4a))

### ⚙️ Miscellaneous Tasks

- Misc updates ([#46](https://github.com/n0-computer/n0-error/issues/46)) - ([79dd4d6](https://github.com/n0-computer/n0-error/commit/79dd4d62ff6741910806d0f7cfd9819e4c40314d))

## [0.1.3](https://github.com/n0-computer/n0-error/compare/v0.1.2..0.1.3) - 2026-01-15

### 🐛 Bug Fixes

- Fix macro for struct errors with sources ([#33](https://github.com/n0-computer/n0-error/issues/33)) - ([f5d25e2](https://github.com/n0-computer/n0-error/commit/f5d25e25bc19588a73550427f047af66002e85e5))

### Deps

- Cargo update ([#34](https://github.com/n0-computer/n0-error/issues/34)) - ([9b24aaa](https://github.com/n0-computer/n0-error/commit/9b24aaaddbad2ec2a7c000f1f29c899a4c0c95f7))

## [0.1.2](https://github.com/n0-computer/n0-error/compare/v0.1.1..v0.1.2) - 2025-11-12

### 🐛 Bug Fixes

- Remove anyhow from default features again ([#19](https://github.com/n0-computer/n0-error/issues/19)) - ([7c36206](https://github.com/n0-computer/n0-error/commit/7c362066fd902b228cc104c1a8be987f5a23b8e4))

### ⚙️ Miscellaneous Tasks

- Prep 0.1.2 release ([#20](https://github.com/n0-computer/n0-error/issues/20)) - ([f7dd44d](https://github.com/n0-computer/n0-error/commit/f7dd44d94cc94d441c072f4502e5c71ad4371638))

## [0.1.1](https://github.com/n0-computer/n0-error/compare/v0.1.0..v0.1.1) - 2025-11-12

### ⛰️  Features

- Impl StackError for Arc<T: StackError> ([#15](https://github.com/n0-computer/n0-error/issues/15)) - ([e094af6](https://github.com/n0-computer/n0-error/commit/e094af6171657f65103979116a2978ca8898aa60))
- Impl StackError for anyhow:::Error ([#14](https://github.com/n0-computer/n0-error/issues/14)) - ([b11a365](https://github.com/n0-computer/n0-error/commit/b11a365e971ca2bd3cce7d59b128176f9fdb3fc8))

### 🐛 Bug Fixes

- Revert version change inadvertently done by last commit - ([6fa9d40](https://github.com/n0-computer/n0-error/commit/6fa9d40ac7f7ff6d69d727cbbd73ce5658d8e994))

### ⚙️ Miscellaneous Tasks

- Release prep for 0.1.1 ([#18](https://github.com/n0-computer/n0-error/issues/18)) - ([f3538ae](https://github.com/n0-computer/n0-error/commit/f3538aef5d3e87f0a6f298c65ab25df72eedab5f))
- Add workspace entry to Cargo.toml - ([554fcfa](https://github.com/n0-computer/n0-error/commit/554fcfa058f2a2a5fa67a9e3393cabb2e2c9a453))

### Deps

- Remove derive_more and heck ([#17](https://github.com/n0-computer/n0-error/issues/17)) - ([f27edc5](https://github.com/n0-computer/n0-error/commit/f27edc5d27ace51734cdf3cf353a5f2a1a3df855))

## [0.1.0] - 2025-10-31

### ⛰️  Features

- Support structs - ([d0a2330](https://github.com/n0-computer/n0-error/commit/d0a2330cacd2e3f48f3b79203b1d11a392da5201))
- Implement new version after discussion and review of design - ([c8caee0](https://github.com/n0-computer/n0-error/commit/c8caee09561f095b0079a26015ecb806d2ae2a8e))
- Add Err macro - ([73eb1ea](https://github.com/n0-computer/n0-error/commit/73eb1ea12271d26df2f4e7672c94e1f88dc11743))
- Support tuple structs and enum variants - ([665803b](https://github.com/n0-computer/n0-error/commit/665803bfd09daf45c7e36208b849223aaebc11c3))
- Tuple support - ([69df266](https://github.com/n0-computer/n0-error/commit/69df266c38669d31bd5accbe39d26c8b93f5bcc2))
- Support tuple structs and enum variants - ([ed20998](https://github.com/n0-computer/n0-error/commit/ed20998c57f5daf45396ac3114dca6321f00d57d))
- From<std::io::Error> for AnyError - ([b35213c](https://github.com/n0-computer/n0-error/commit/b35213ce3f7734c446070511b569f742928f7203))
- Downcast errors - ([bf33c7d](https://github.com/n0-computer/n0-error/commit/bf33c7dda44dcfb1c2b96c506ddeace0233442e2))
- Downcast errors - ([9c431d2](https://github.com/n0-computer/n0-error/commit/9c431d2a338daf58182d739e20448ac524f4e144))

### 🐛 Bug Fixes

- Expose NoneError - ([a51431d](https://github.com/n0-computer/n0-error/commit/a51431dba41021f1dd0bfbcb1115fc3c5ec75116))
- Macro hygiene - ([0070b10](https://github.com/n0-computer/n0-error/commit/0070b103873f285627240e3c772fecec3f0b4528))
- Remove deprecated macros - ([f886f3f](https://github.com/n0-computer/n0-error/commit/f886f3f9360e501a720cca5a9c4b11732bbb9507))
- Generic errors - ([652923b](https://github.com/n0-computer/n0-error/commit/652923bf05d9d27ec368c9d86d265ac258c5aa4a))
- Cargo.toml - ([f27f09d](https://github.com/n0-computer/n0-error/commit/f27f09df59f237e3474be9a182e99224c2b60925))

### 🚜 Refactor

- Various changes after review - ([dd52706](https://github.com/n0-computer/n0-error/commit/dd527069b8c43f37f8a353e1c88218d990c4b084))
- Change formatting if no display is provided - ([1e30e84](https://github.com/n0-computer/n0-error/commit/1e30e84098e442ac42e06b47c746da8bf4f855e6))
- [**breaking**] Various changes after review - ([9a672e6](https://github.com/n0-computer/n0-error/commit/9a672e6ea960a202f761f686b4afe1ce507aaba7))
- Rename ensure_e macro to ensure - ([ce4cfe0](https://github.com/n0-computer/n0-error/commit/ce4cfe034c5590bf90bbef5699939b487e487f82))
- Replace #[display] with #[error] - ([f8d6706](https://github.com/n0-computer/n0-error/commit/f8d6706e067dfa5add761c0aef726d780e405ff5))
- Rename try to try_or, simplify macros - ([108bdf5](https://github.com/n0-computer/n0-error/commit/108bdf5f90d7722fbdc44cf43eaf3456a0cadbd3))
- Improve macro, use stack_error - ([faddd3f](https://github.com/n0-computer/n0-error/commit/faddd3fc573a09401a6505a89daad4d3992842cc))
- Add `stack_error` attribute macro - ([c00949c](https://github.com/n0-computer/n0-error/commit/c00949c842d9907ca6446a7c2fcbee9915a2773a))

### 📚 Documentation

- Mark as experimental - ([5e50fb8](https://github.com/n0-computer/n0-error/commit/5e50fb8cbf50d98808ffc276850d2756a2c42f00))
- Improve - ([e3d7f3c](https://github.com/n0-computer/n0-error/commit/e3d7f3c44399c4c4dafb43331ae0ee98ee9ebb95))
- Fix - ([672bfd8](https://github.com/n0-computer/n0-error/commit/672bfd820aa6474e640c2bcae5495548e38e40c3))
- Improve docs all over the place - ([1c518f2](https://github.com/n0-computer/n0-error/commit/1c518f29bc8719665041cfe727d8e261567e7a77))
- Fix readme - ([41312a1](https://github.com/n0-computer/n0-error/commit/41312a19c196c8dd85767d595f2d345477dd6817))

### ⚙️ Miscellaneous Tasks

- Fmt - ([17da780](https://github.com/n0-computer/n0-error/commit/17da780e9525ed75d70bc83502faad07d2754ea5))
- Add ci - ([b4f4199](https://github.com/n0-computer/n0-error/commit/b4f41997c764a26aca51bbbe1c0e7885b808a6b4))
- Fmt - ([4f311ae](https://github.com/n0-computer/n0-error/commit/4f311ae9a8a914b6c11de18cc554c6fcdc59c0bb))
- Add nextest config - ([a00ff9b](https://github.com/n0-computer/n0-error/commit/a00ff9bdf550f22d64a429a8b62d7bb9d90a861a))
- Fmt - ([334a696](https://github.com/n0-computer/n0-error/commit/334a6969798ade65a1b89c38326514f798f140ca))
- Remove leftover code - ([8363fef](https://github.com/n0-computer/n0-error/commit/8363fefb8311b197f504f54dbbd903ae839bc5cc))

### Deps

- Ensure minimum version of anyhow for into_boxed_dyn_error - ([79fbfc5](https://github.com/n0-computer/n0-error/commit/79fbfc51256be0cfb47a7d68e97eb7fcaeb61f28))
- Remove darling - ([3ed2f22](https://github.com/n0-computer/n0-error/commit/3ed2f229307e3394df209603441c4037a4cc0220))

### Wip

- Add some tests - ([e90f401](https://github.com/n0-computer/n0-error/commit/e90f401d13af78e38802a556e57b0a89ff5ef12c))
- Refactor macros - ([13e2536](https://github.com/n0-computer/n0-error/commit/13e2536a43bc9feb13f74f8107b3745ca4ee34c3))


