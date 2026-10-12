#!/bin/sh
glib-compile-resources --target="resources.gresource" --sourcedir="oku/data/hicolor/scalable/actions" "oku/resources.gresource.xml"
cargo clippy -p oku-core --fix --allow-dirty --features="cli,fuse,persistent"
cargo clippy -p oku --fix --allow-dirty
cargo fmt --all
umount ~/.local/share/oku/mount
RUST_BACKTRACE=full cargo run -p oku