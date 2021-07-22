#!/bin/bash

# Cleanup
rm -rf ./gtk4-dependencies
mkdir gtk4-dependencies && cd gtk4-dependencies

# libsoup 3
git clone --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/libsoup.git
git checkout d9f97292b71e7f14f91158750c81f33bb8386973
git reset --hard
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
git clone --depth 1 --recurse-submodules --shallow-submodules --branch Safari-612.1.24 https://github.com/WebKit/WebKit.git
cd ./WebKit
cmake -DPORT=GTK -DUSE_GTK4=ON -DENABLE_EXPERIMENTAL_FEATURES=ON -DCMAKE_BUILD_TYPE=RelWithDebInfo -GNinja && ninja
sudo ninja install
cd ../
