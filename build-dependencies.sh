#!/bin/bash

# Cleanup
rm -rf ./gtk4-dependencies
mkdir gtk4-dependencies && cd gtk4-dependencies

# libsoup 3
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/libsoup.git
# Get latest version of glib-2.0 as a build dependency
cd ./libsoup/subprojects
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/glib.git
cd ../
# Build libsoup
mkdir build && cd build
meson --prefix=/usr --buildtype=release .. && ninja
sudo ninja install
cd ../../

# WebKit with GTK4 support
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
cd ../
