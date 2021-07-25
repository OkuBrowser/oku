#!/bin/bash

# Cleanup
rm -rf ./gtk4-dependencies
mkdir gtk4-dependencies && cd gtk4-dependencies

# GLib
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/glib.git
cd ./glib
meson _build && ninja -C _build
sudo ninja -C _build install
cd ../

# libsoup 3
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/libsoup.git
cd ./libsoup
mkdir build && cd build
meson --prefix=/usr --buildtype=release .. && ninja
sudo ninja install
cd ../../

# WebKitGTK 5.0
git clone --depth 1 --recurse-submodules --shallow-submodules https://github.com/WebKit/WebKit.git
cd ./WebKit
mkdir build && cd build
cmake -DCMAKE_BUILD_TYPE=Release \
-DCMAKE_INSTALL_PREFIX=/usr \
-DCMAKE_SKIP_RPATH=ON -DPORT=GTK \
-DLIB_INSTALL_DIR=/usr/lib \
-DUSE_GTK4=ON \
-DUSE_AVIF=ON \
-GNinja .. && ninja
sudo ninja install
cd ../../
