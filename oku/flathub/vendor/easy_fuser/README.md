# easy_fuser

[![CI Ubuntu](https://github.com/Alogani/easy_fuser/actions/workflows/ubuntu.yml/badge.svg?branch=main)](https://github.com/Alogani/easy_fuser/actions/workflows/ubuntu.yml?query=branch%3Amain)
[![Crates.io](https://img.shields.io/crates/v/easy_fuser.svg)](https://crates.io/crates/easy_fuser)
[![Documentation](https://docs.rs/easy_fuser/badge.svg)](https://docs.rs/easy_fuser)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/Alogani/easy_fuser/blob/master/LICENSE.md)
[![dependency status](https://deps.rs/repo/github/Alogani/easy_fuser/status.svg)](https://deps.rs/repo/github/Alogani/easy_fuser)

> [!IMPORTANT]
> This crate is a school project and should not be considered production ready until proven otherwise, keep in mind and have fun !
> **Breaking Changes in v0.5.0**: The crate has been reorganized to support separate code generation structures per mode.
> - Instead of a single prelude (`easy_fuser::prelude`), use the mode-specific preludes: `easy_fuser::fuse_serial::prelude::*`, `easy_fuser::fuse_parallel::prelude::*`, or `easy_fuser::fuse_async::prelude::*`.
> - Template/Preset implementations (like `DefaultFuseHandler` and `MirrorFs`) are now located in the `easy_fuser::fuse_presets` module (instead of `easy_fuser::templates`).
> - The presets no longer implement `FuseHandler` directly. Users now implement `FuseHandler` for their custom struct and delegate operations using the `delegate_fs!` macro from the `easy_fuser_macro` crate.

## About

`easy_fuser` is a high-level, ergonomic wrapper around the `fuser` crate, designed to simplify
the process of implementing FUSE (Filesystem in Userspace) filesystems in Rust. It abstracts away
many of the complexities, offering a more intuitive and Rust-idiomatic approach to filesystem development.

## Key Features

- **Simplified API**: Provides a higher-level interface compared to `fuser`, reducing boilerplate
  and making filesystem implementation more straightforward.

- **Flexible Concurrency Models**: Offers three distinct concurrency models to suit different
  use cases and performance requirements.

- **Flexible File Identification**: Supports both path-based and inode-based operations,
  allowing you to choose between `Inode`, `PathBuf`, or `Vec<OsString>` as your file identifier type. This
  offers flexibility in how you represent and manage file identities, suitable for different
  filesystem structures and performance requirements.

- **Error Handling**: Provides a structured error handling system, facilitating the management
  of filesystem-specific errors.

- **Composable Presets and Examples**: Includes pre-built, composable presets and a comprehensive
  examples folder to help you get started quickly, understand various implementation patterns,
  and easily combine different filesystem behaviors. These presets are designed to be mixed
  and matched via delegation, allowing for flexible and modular filesystem creation.

## File Identification Flexibility

`easy_fuser` supports two main approaches for file identification:

1. **Path-based Operations**: Work with file paths directly, which can be more intuitive for
   certain use cases.
2. **Inode-based Operations**: Use inode numbers for more efficient control, especially useful
   for complex filesystem structures or when performance is critical.

You can choose the approach that best fits your filesystem's needs and switch between them
as necessary.

## Usage

To use `easy_fuser`, follow these steps:

1. Import the appropriate prelude for your concurrency mode (e.g. `easy_fuser::fuse_parallel::prelude::*`).
2. Implement the `FuseHandler` trait for your filesystem structure, specifying the `TId` type (e.g. `PathBuf`).
3. (Optional) Compose presets (like `DefaultFuseHandler` or `MirrorFs` from `easy_fuser::fuse_presets`) and delegate operations to them using the `delegate_fs!` macro.
4. Mount or spawn-mount your filesystem.

Here's a basic example:

```rust,ignore
#[cfg(feature = "serial")]
use easy_fuser::fuse_serial::prelude::*;
#[cfg(all(feature = "parallel", not(feature = "serial")))]
use easy_fuser::fuse_parallel::prelude::*;
#[cfg(all(feature = "async", not(feature = "parallel"), not(feature = "serial")))]
use easy_fuser::fuse_async::prelude::*;

use easy_fuser::fuse_presets::DefaultFuseHandler;
use easy_fuser_macro::delegate_fs;
use std::path::{Path, PathBuf};

struct MyFS {
    default_fs: DefaultFuseHandler<PathBuf>,
}

impl FuseHandler for MyFS {
    type TId = PathBuf;

    // Delegate all standard FUSE methods to default_fs
    delegate_fs! { default_fs, [
        access, bmap, copy_file_range, create, fallocate, flush, forget, fsync, fsyncdir,
        getattr, getlk, getxattr, ioctl, link, listxattr, lookup, lseek, mkdir, mknod,
        open, opendir, read, readdir, readlink, release, releasedir, removexattr, rename,
        rmdir, setattr, setlk, setxattr, statfs, symlink, unlink, write
    ]}
}

fn main() -> std::io::Result<()> {
    let fs = MyFS { default_fs: DefaultFuseHandler::new() };
    
    // Mount the filesystem, optionally configuring the number of threads.
    // In parallel mode, Some(4) runs FUSE handlers on 4 worker threads.
    // In serial mode, the thread count argument is ignored.
    // In async mode, the thread count argument configures tokio threads.
    // If you pass None, a default configuration is used.
    mount(fs, Path::new("/mnt/myfs"), &[], Some(4))?;
    
    Ok(())
}
```

### Async Delegation

When using the `async` concurrency model, the `FuseHandler` trait is decorated with `#[async_trait]`. Because outer attribute macros expand before inner macro invocations, a standard delegation macro like `delegate_fs!` cannot be desugared by `#[async_trait]`.

To solve this, `easy_fuser` provides two specialized async delegation macros that perform **manual signature desugaring** matching the expected output format of `#[async_trait]`:

1. **`delegate_fs_async!`**: Use this when delegating to a field/target that itself exposes **asynchronous** methods (returning Futures).
2. **`delegate_fs_sync_to_async!`**: Use this when delegating to a field/target that exposes **synchronous/blocking** methods. The macro automatically wraps the synchronous method call in a pinned async block.

#### Example for Async Mode

```rust,ignore
use easy_fuser::fuse_async::prelude::*;
use easy_fuser::fuse_presets::mirror_fs::MirrorFs;
use easy_fuser::fuse_presets::DefaultFuseHandler;
use easy_fuser_macro::delegate_fs_sync_to_async;
use std::path::PathBuf;

struct MyAsyncFS {
    // MirrorFs has standard synchronous/blocking methods
    mirror_fs: MirrorFs,
    default_fs: DefaultFuseHandler<PathBuf>,
}

#[async_trait]
impl FuseHandler for MyAsyncFS {
    type TId = PathBuf;

    // Delegate to the synchronous MirrorFs target inside an async handler
    delegate_fs_sync_to_async! { mirror_fs, [ read, write, getattr ] }

    // Delegate remaining methods to default_fs
    delegate_fs_sync_to_async! { default_fs, [ statfs, link ] }
}
```


## Feature Flags

This crate provides three feature flags for different concurrency models:

- `serial`: Enables single-threaded operation. Use this for simplicity and when concurrent
  access is not required. The thread count argument (`Option<usize>`) is accepted for API consistency but ignored.

- `parallel`: Enables multi-threaded operation using a thread pool. This is suitable for
  scenarios where you want to handle multiple filesystem operations concurrently on separate
  threads. It can improve performance on multi-core systems. Pass `Some(threads)` to specify the pool size, or `None` to automatically use a default based on the system's CPU count.

- `async`: Enables asynchronous operation using tokio. This is ideal for high-concurrency scenarios and
  when you want to integrate the filesystem with asynchronous Rust code. Pass `Some(threads)` to configure tokio's worker threads, or `None` to use the default multi-threaded runtime. When this feature is enabled, you use `easy_fuser::fuse_async::prelude::*` which decorates `FuseHandler` with `#[async_trait]`.

Example usage in Cargo.toml:
```toml
[dependencies]
easy_fuser = { version = "0.5.0", features = ["parallel"] }
```

By leveraging `easy_fuser`, you can focus more on your filesystem's logic and less on the
intricacies of FUSE implementation, making it easier to create robust, efficient, and
maintainable filesystem solutions in Rust.

## Presets / Templates

`easy_fuser` provides a set of template implementations (presets) under the `easy_fuser::fuse_presets` module to help you get started quickly:

- **DefaultFuseHandler**: A backbone implementation that acts as a NullFs, implementing every
  operation. It can also be used as a PanicFs for debugging purposes.
- **FdHandlerHelper**: Provides boilerplate for operations on open files (ReadOnly and ReadWrite variants available).
- **MirrorFs**: A passthrough filesystem template that can be leveraged for creating more complex filesystems.

These presets serve as composable building blocks, allowing you to mix and match functionalities to create custom, complex filesystem implementations with ease using delegation (via `delegate_fs!`).

## Examples

Please check the README inside the examples folder for additional details and references.

## Common Caveats

When working with FUSE filesystems, be aware of the following:

1. **Crashes & Proper Unmounting**:
If a program crashes or is stopped abruptly (e.g., using Ctrl+C), it may leave the mountpoint in an inconsistent state.

To properly unmount the filesystem and stop the program (or to resolve a bad state after a crash), use the following command:

  ```bash
  fusermount -u <mountpoint>
  ```

This is the preferred method for both unmounting and resolving any issues with the mountpoint. You will find more information in the documentation of `mount` and `spawn_mount`.

2. **Modifying the source directory while mounted**: This is not well-supported behavior and can result in unexpected outcomes.

## Important Notes

libfuse and by extension fuser contains a lot of flags as arguments. We tried to identify them as much as possible, but cannot guarantee it due to the lack of clear documentation on this subject.
