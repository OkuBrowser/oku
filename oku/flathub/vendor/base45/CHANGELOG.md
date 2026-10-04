# [3.2.0](https://github.com/opendevtools/base45/compare/v3.1.0...v3.2.0) (2025-12-16)


### Features

* support no_std ([a5a46c7](https://github.com/opendevtools/base45/commit/a5a46c76c61ad4e430941d4c6d9cc305a7670894))

# [3.1.0](https://github.com/opendevtools/base45/compare/v3.0.0...v3.1.0) (2023-06-30)


### Features

* `array_chunks` (performance feature), `alloc` improvements ([#3](https://github.com/opendevtools/base45/issues/3)) ([9e0e933](https://github.com/opendevtools/base45/commit/9e0e9333243f87c485c32ef2ed208ed3a1d509f2))
* performance improvements ([#2](https://github.com/opendevtools/base45/issues/2)) ([63db6f2](https://github.com/opendevtools/base45/commit/63db6f2495a593d7df7acb506716687d8fef3033))

# [3.0.0](https://github.com/opendevtools/base45/compare/v2.0.1...v3.0.0) (2021-06-03)

### Features

- return decoding errors ([#1](https://github.com/opendevtools/base45/issues/1)) ([185d311](https://github.com/opendevtools/base45/commit/185d311951e0f567dc9ccfd4c2cbf9219271ce45))

### BREAKING CHANGES

- Update the return value of `base45::decode` to `Result<_,_>`. This allows us to handle errors gracefully instead of panicking

## [2.0.1](https://github.com/opendevtools/base45/compare/v2.0.0...v2.0.1) (2021-04-02)

### Bug Fixes

- handle invalid strings ([c9957b5](https://github.com/opendevtools/base45/commit/c9957b5f43e8df798bb31086ea29c27452333e79))

# [2.0.0](https://github.com/opendevtools/base45/compare/v1.0.2...v2.0.0) (2021-04-01)

### Features

- add encode_from_buffer ([f8b3892](https://github.com/opendevtools/base45/commit/f8b3892987a13f9fb2376ccb78adfb00d1e71950))
- decode returns buffer ([dca8ddf](https://github.com/opendevtools/base45/commit/dca8ddf956df2219aea07e926928409a5426863f))

### BREAKING CHANGES

- decode used to return a string, now it returns a
  buffer and the user should convert it to a string

## [1.0.2](https://github.com/opendevtools/base45/compare/v1.0.1...v1.0.2) (2021-03-22)

### Bug Fixes

- create release zip ([2587c27](https://github.com/opendevtools/base45/commit/2587c272f21b6eb7f02685a1841cfc280aeba53a))

## [1.0.1](https://github.com/opendevtools/base45/compare/v1.0.0...v1.0.1) (2021-03-22)

### Bug Fixes

- update docs ([28b8e00](https://github.com/opendevtools/base45/commit/28b8e006d315d73d445b969ffedb37b8705d6164))

# 1.0.0 (2021-03-22)

### Bug Fixes

- remove public size ([b263d2f](https://github.com/opendevtools/base45/commit/b263d2fdb28015b3c16d162b1a010a8619a08a17))

### Features

- initial commit ([c6389d7](https://github.com/opendevtools/base45/commit/c6389d774e77b814684d965e3f26433e28b7f4c6))
