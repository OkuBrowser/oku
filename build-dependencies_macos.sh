# Cleanup
rm -rf ./gtk4-dependencies
mkdir gtk4-dependencies && cd gtk4-dependencies

brew install gtk4 gtk+3 libiconv libpsl

# libunistring
curl https://ftp.gnu.org/gnu/libunistring/libunistring-latest.tar.gz --output libunistring.tar
mkdir ./libunistring
tar -xvf libunistring.tar -C libunistring --strip-components=1
cd ./libunistring
pwd
./configure --disable-dependency-tracking -disable-silent-rules
make
make check
sudo make install
cd ../

# libsoup 3
git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/libsoup.git
# # Get latest version of glib-2.0 as a build dependency
# cd ./libsoup/subprojects
# git clone --depth 1 --recurse-submodules --shallow-submodules https://gitlab.gnome.org/GNOME/glib.git
# # Get latest version of libpsl as a build dependency
# git clone --depth 1 --recurse-submodules --shallow-submodules https://github.com/rockdaboot/libpsl
# # # Get latest version of libunistring as a build dependency
# # mkdir -p ./libpsl/subprojects
# # cd ./libpsl/subprojects
# # git clone --depth 1 --recurse-submodules --shallow-submodules https://git.savannah.gnu.org/git/libunistring.git
# # cd ../../
# cd ../
# Build libsoup
cd ./libsoup
mkdir build && cd build
meson --prefix=/usr --buildtype=release .. && ninja
sudo ninja install
cd ../../

git clone --depth 1 --recurse-submodules --shallow-submodules https://github.com/WebKit/WebKit.git
./WebKit/Tools/Scripts/build-webkit --gtk --makeargs="-USE_GTK4=ON"