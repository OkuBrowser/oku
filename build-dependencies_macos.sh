#!/bin/bash

# Cleanup
rm -rf ./gtk4-dependencies
mkdir gtk4-dependencies && cd gtk4-dependencies

# libunistring
git clone --depth 1 --recurse-submodules --shallow-submodules https://git.savannah.gnu.org/git/libunistring.git
cd ./libunistring
./gitsub.sh pull
./autogen.sh
./configure
make
sudo make install
cd ../

# libpsl
git clone --depth 1 --recurse-submodules --shallow-submodules https://github.com/rockdaboot/libpsl
cd ./libpsl
./autogen.sh
./configure
make
make check
sudo make install
cd ../

# GLib
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/glib.git
cd ./glib
meson _build && ninja -C _build
sudo ninja -C _build install
cd ../

# glib-networking
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/glib-networking.git
# Get latest version of glib-2.0 as a build dependency
cd ./glib-networking
mkdir subprojects && cd subprojects
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/glib.git
cd ../
mkdir build && cd build
meson --buildtype=release .. && ninja
sudo ninja install
cd ../../

# libsoup 3
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/libsoup.git
# Get latest version of glib-2.0 as a build dependency
cd ./libsoup/subprojects
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/glib.git
git clone --depth 1 --recurse-submodules --shallow-submodules https://github.com/rockdaboot/libpsl
cd ../
# Build libsoup
mkdir build && cd build
meson --buildtype=release .. && ninja
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
