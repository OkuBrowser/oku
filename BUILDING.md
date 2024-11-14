# Building

Note: Currently, the Oku browser is only available on operating systems using the Linux kernel.
The [protocol included with Oku](https://github.com/OkuBrowser/oku-fs) may be used via a command-line frontend, available on Linux, macOS, and Windows.

## Binary

### Prerequisites

To build, please install:
* A copy of the [Rust toolchain](https://www.rust-lang.org/tools/install)
    * It is recommended that you install Rust using [`rustup.rs`](https://rustup.rs/), though many Linux distributions also package the Rust toolchain as well.
* [GTK and its dependencies](https://www.gtk.org/docs/installations/linux), including GDK, GLib, and Pango
    * It is recommended that you obtain these development packages from your distribution.
* [WebKitGTK](https://webkitgtk.org/)
    * Some distributions provide multiple versions of WebKitGTK; look for packages resembling `webkitgtk-6.0`.
* [`libfuse`](https://github.com/libfuse/libfuse/)
    * It is recommended that you obtain this development package from your distribution.
* The [Vox static site generator](https://emmyoh.github.io/vox/)
    * This can be installed using the Rust toolchain by running `cargo install --git https://github.com/emmyoh/vox --features="cli"`.

### Commands

Note: Before new builds, please run `./prebuild.sh` to complete necessary pre-compilation tasks.

After pre-requisites are installed and pre-compilation tasks are complete, you may run:
* `cargo build` for debug builds.
* `cargo build --release` for release builds.
* `cargo install --path .` to install Oku.

### Troubleshooting

#### Missing Icon

If the browser's icon is missing at runtime, copy the icon files to the appropriate location on your system (eg, `cp -avr ./data/hicolor /app/share/icons/`).

#### Broken Replica Mount After Crash

If the browser has been restarted after a crash and replicas are now inaccessible in the file manager, try running `umount ~/.local/share/oku/mount`.

## Flatpak

### Prerequisites

In addition to the prerequisites above, the Flatpak build requires:

* [Flatpak](https://flatpak.org/setup/)
* `flatpak-builder`
    * Run `flatpak install org.flatpak.Builder` to install.

### Commands

Run `./install_flatpak.sh`, assuming prerequisites are installed.
This will output `oku.flatpak` and install the Flatpak.

### Troubleshooting

#### Not Opening

Try starting Oku from the command-line by running `flatpak run io.github.OkuBrowser.oku`.
This should reveal any runtime issues.

#### Crashing Upon Start with Ubuntu

See [the following](https://github.com/OkuBrowser/oku/issues/290#issuecomment-2380092489) for a workaround.

#### Broken Replica Mount After Crash

If the browser has been restarted after a crash and replicas are now inaccessible in the file manager, try running `umount ~/.var/app/io.github.OkuBrowser.oku/data/oku/mount`.