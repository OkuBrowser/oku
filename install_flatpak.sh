#!/bin/sh
./prebuild.sh
curl --output "flatpak-cargo-generator.py" "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"
python3 ./flatpak-cargo-generator.py ./Cargo.lock -o "./build-aux/cargo-sources.json"
flatpak run org.flatpak.Builder --force-clean --user --install --install-deps-from=flathub --ccache --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=repo builddir build-aux/com.github.OkuBrowser.json
flatpak build-bundle repo oku.flatpak com.github.OkuBrowser --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo