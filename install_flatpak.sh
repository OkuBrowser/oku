#!/bin/sh
./prebuild.sh
curl --output "flatpak-cargo-generator.py" "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"
python3 ./flatpak-cargo-generator.py ./Cargo.lock -o "./build-aux/cargo-sources.json"
flatpak run org.flatpak.Builder --force-clean --user --install --install-deps-from=flathub --ccache --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=repo builddir build-aux/io.github.OkuBrowser.oku.json
flatpak build-bundle repo oku.flatpak io.github.OkuBrowser.oku --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
ostree commit --repo=repo --canonical-permissions --branch=screenshots/x86_64 builddir/files/share/app-info/media