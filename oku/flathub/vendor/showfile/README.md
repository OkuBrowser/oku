# showfile &emsp;  [![Latest Version]][crates.io] [![Documentation]][docs]

[Documentation]: https://docs.rs/showfile/badge.svg
[docs]: https://docs.rs/showfile
[Latest Version]: https://img.shields.io/crates/v/showfile.svg
[crates.io]: https://crates.io/crates/showfile

A simple Rust crate to show the location of a file in the local file manager
(Explorer, Finder, etc.). Supported platforms are Windows, macOS, Linux.

## Usage

```rust
showfile::show_path_in_file_manager("C:\\Users\\Alice\\hello.txt");
showfile::show_path_in_file_manager("/Users/Bob/hello.txt");
showfile::show_uri_in_file_manager("file:///home/charlie/hello.txt");
```

# Feature Flags

On Linux, D-Bus is used to invoke the file manager. The D-Bus crate in use can be selected with
one of these flags:

- [`rustbus`](https://github.com/KillingSpark/rustbus) (default)
- [`zbus`](https://dbus2.github.io/zbus/)
- [`gio`](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/gio/)

One of these flags must be specified to build the project. These flags do nothing on Windows
and macOS. If only targeting those platforms, it can be left at the default.


## Details

This crate is a simple wrapper around these system functions:

- Windows: [`SHOpenFolderAndSelectItems`](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shopenfolderandselectitems)
- macOS: [`NSWorkspace activateFileViewerSelectingURLs:`](https://developer.apple.com/documentation/appkit/nsworkspace/1524549-activatefileviewerselecting)
- Linux: [`org.freedesktop.FileManager1.ShowItems`](https://www.freedesktop.org/wiki/Specifications/file-manager-interface/)

