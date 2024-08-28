#!/bin/sh
curl --output "flatpak-cargo-generator.py" "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"
python3 ./flatpak-cargo-generator.py ./Cargo.lock -o "./build-aux/cargo-sources.json"
flatpak-builder --install repo build-aux/com.github.dirout.oku.json --force-clean --user --install-deps-from=flathub -y