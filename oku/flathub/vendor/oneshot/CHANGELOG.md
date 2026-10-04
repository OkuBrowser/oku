# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

### Categories each change fall into

* **Added**: for new features.
* **Changed**: for changes in existing functionality.
* **Deprecated**: for soon-to-be removed features.
* **Removed**: for now removed features.
* **Fixed**: for any bug fixes.
* **Security**: in case of vulnerabilities.


## [Unreleased]


## [0.1.13] - 2026-01-26
### Fixed
- Fix documentation building on docs.rs by not using removed `doc_auto_cfg` feature.


## [0.1.12] - 2026-01-25
### Fixed
- Fix race condition that could lead to use-after-free if the `Receiver` was polled asynchronously,
  but then dropped before completion. RUSTSEC-2026-0005 https://github.com/faern/oneshot/pull/74
- Fix race conditions/UB around atomic memory orderings. These were found by running tests under
  miri. https://github.com/faern/oneshot/pull/72


## [0.1.11] - 2025-02-22
### Fixed
- Handle the `UNPARKING` state correctly in `Receiver::drop()`. Fixes a panic that could
  occur if a `Receiver` had been first polled as a future and then was being dropped
  in parallel with the `Sender` sending a message.


## [0.1.10] - 2025-02-04
### Added
- Add `is_closed` and `has_message` to the `Receiver`. Allows polling for the channel
  state without modifying the channel or pulling the message from it.
- Make the cargo features show up on docs.rs for better discoverability.


## [0.1.9] - 2025-02-02
### Added
- Implement `Sync` for `Sender`. There is not a whole lot someone can do with a `&Sender`,
  but this allows storing the sender in places that are overly conservative and require
  a `Sync` bound on the content.


## [0.1.8] - 2024-06-13
### Changed
- Change how loom concurrency testing is triggered. To get rid of `loom` in the dependency tree
  `oneshot` pulls in, it has in addition to being gated behind `cfg(oneshot_loom)` also been made
  an optional dependency. This makes this library way smaller for downstream consumers.
  This has the downside that the crate now exposes a `loom` feature.
  DOWNSTREAM USERS ARE NOT SUPPOSED TO EVER ENABLE THIS. No stability or semver
  guarantees exist around the `loom` feature.
  This change ultimately makes no difference for any user of `oneshot` in regular usage.


## [0.1.7] - 2024-05-24
### Added
* Add `is_closed` method to the `Sender`.


## [0.1.6] - 2023-09-14
### Added
* Add `into_raw` and `from_raw` methods on both `Sender` and `Receiver`. Allows passing `oneshot`
  channels over FFI without an extra layer of heap allocation.


## [0.1.5] - 2022-09-01
### Fixed
- Handle the UNPARKING state correctly in all recv methods. `try_recv` will now not panic
  if used on a `Receiver` that is being unparked from an async wait. The other `recv` methods
  will still panic (as they should), but with a better error message.


## [0.1.4] - 2022-08-30
### Changed
- Upgrade to Rust edition 2021. Also increases the MSRV to Rust 1.60.
- Add null-pointer optimization to `Sender`, `Receiver` and `SendError`.
  This reduces the call stack size of Sender::send and it makes
  `Option<Sender>` and `Option<Receiver>` pointer sized (#18).
- Relax the memory ordering of all atomic operations from `SeqCst` to the most appropriate
  lower ordering (#17 + #20).

### Fixed
- Fix undefined behavior due to multiple mutable references to the same channel instance (#18).
- Fix race condition that could happen during unparking of a receiving `Receiver` (#17 + #20).


## [0.1.3] - 2021-11-23
### Fixed
- Keep the *last* `Waker` in `Future::poll`, not the *first* one. Stops breaking the contract
  on how futures should work.


## [0.1.2] - 2020-08-11
### Fixed
- Fix unreachable code panic that happened if the `Receiver` of an empty but open channel was
  polled and then dropped.


## [0.1.1] - 2020-05-10
Initial implementation. Supports basically all the (for now) intended functionality.
Sender is as lock-free as I think it can get and the receiver can both do thread blocking
and be awaited asynchronously. The receiver also has a wait-free `try_recv` method.

The crate has two features. They are activated by default, but the user can opt out of async
support as well as usage of libstd (making the crate `no_std` but still requiring liballoc)


## [0.1.0] - 2019-05-30
Name reserved on crate.io by someone other than the author of this crate.
