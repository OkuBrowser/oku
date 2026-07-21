# pkg-config
brew reinstall pkg-config

# Requirements in Homebrew
brew install cmake ninja enchant gtk4 libsecret libnotify libpng libsoup openjpeg jpeg-turbo webp woff2 zlib

# libbacktrace
rm -rf libbacktrace
git clone --depth=1 https://github.com/ianlancetaylor/libbacktrace.git libbacktrace
cd libbacktrace 
mkdir build 
cd build 
../configure 
sudo make 
sudo make install
cd ../../

# Hyphen
rm -rf hyphen
git clone --depth=1 https://github.com/hunspell/hyphen hyphen
cd hyphen
autoreconf -fvi
./configure
make
make install
cd ../

# WebKitGTK
rm -rf WebKit
git clone --depth=1 https://github.com/WebKit/WebKit.git WebKit
cd WebKit
CFLAGS="-I$(brew --prefix)/include -I/usr/local/include" LDFLAGS="-L$(brew --prefix)/lib -L/usr/local/lib" cmake --fresh . -GNinja -DPORT=GTK -DENABLE_X11_TARGET=OFF -DENABLE_WAYLAND_TARGET=OFF -DENABLE_QUARTZ_TARGET=ON -DENABLE_TOOLS=ON -DENABLE_MINIBROWSER=OFF -DENABLE_VIDEO=OFF -DENABLE_WEB_AUDIO=OFF -DENABLE_GEOLOCATION=OFF -DENABLE_WEBGL=OFF -DENABLE_GAMEPAD=OFF -DUSE_SOUP2=OFF -DUSE_LIBDRM=FALSE -DENABLE_JOURNALD_LOG=FALSE -DUSE_SYSTEM_SYSPROF_CAPTURE=NO -DUSE_SKIA=OFF -DUSE_LIBBACKTRACE=OFF
cp -r /usr/local/include/* WTF/DerivedSources
cmake --build .
cmake --build . -- install
cd ../