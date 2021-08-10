# Oku

<a href="https://Dirout.github.io/oku">
<p align="center">
<img src="https://raw.githubusercontent.com/Dirout/oku/master/branding/logo.svg" width="256" height="256">
</p>
</a>

Oku is a hive browser written in Rust.

ipfs://bafybeihtb3t3tjrc25stq4egznl7v7ixjtabdxhnm7tx3q7r7sgfc5uyju/index.html

ipfs://bafybeiccfclkdtucu6y4yc5cpr6y3yuinr67svmii46v5cfcrkp47ihehy/README.txt
ipfs://bafybeiccfclkdtucu6y4yc5cpr6y3yuinr67svmii46v5cfcrkp47ihehy/albums/QXBvbGxvIDE3IE1hZ2F6aW5lIDE0Ny9B/21687952681_e2e01394d2_o.jpg
ipfs://bafybeiccfclkdtucu6y4yc5cpr6y3yuinr67svmii46v5cfcrkp47ihehy/frontend/frontend.html

ipfs://bafybeiboj7xkq4jxxbs5r65ditneqanwltqfzis4f4ii2u5bu36twro3yy/media/earth.gif
ipfs://bafybeiboj7xkq4jxxbs5r65ditneqanwltqfzis4f4ii2u5bu36twro3yy/home.html
ipfs://bafybeiboj7xkq4jxxbs5r65ditneqanwltqfzis4f4ii2u5bu36twro3yy

ipfs://bafybeihfg3d7rdltd43u3tfvncx7n5loqofbsobojcadtmokrljfthuc7y/about.html
ipfs://bafybeihfg3d7rdltd43u3tfvncx7n5loqofbsobojcadtmokrljfthuc7y/983%20-%20Privacy/983%20-%20Privacy.png
ipfs://bafybeihfg3d7rdltd43u3tfvncx7n5loqofbsobojcadtmokrljfthuc7y/983%20-%20Privacy/983%20-%20Privacy%20-%20transcript.txt

brew install protobuf
brew install glib
brew install gtk+4
export PKG_CONFIG_PATH="/opt/homebrew/opt/icu4c/lib/pkgconfig"

brew install libsoup
brew install cmake
brew install ninja
brew install libgcrypt
sudo port -t install at-spi2-core

If you need to have icu4c first in your PATH, run:
  echo 'export PATH="/opt/homebrew/opt/icu4c/bin:$PATH"' >> ~/.zshrc
  echo 'export PATH="/opt/homebrew/opt/icu4c/sbin:$PATH"' >> ~/.zshrc

For compilers to find icu4c you may need to set:
  export LDFLAGS="-L/opt/homebrew/opt/icu4c/lib"
  export CPPFLAGS="-I/opt/homebrew/opt/icu4c/include"

For pkg-config to find icu4c you may need to set:
  export PKG_CONFIG_PATH="/opt/homebrew/opt/icu4c/lib/pkgconfig"

https://trac.webkit.org/wiki/BuildingGtk#BuildingtheGTKportonMacOS