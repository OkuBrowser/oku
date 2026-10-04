#!/bin/sh
./prebuild.sh
flatpak run org.flatpak.Builder --force-clean --user --install --install-deps-from=flathub --ccache --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=repo builddir flathub/io.github.OkuBrowser.oku.json
flatpak build-bundle repo oku.flatpak io.github.OkuBrowser.oku --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
ostree commit --repo=repo --canonical-permissions --branch=screenshots/x86_64 builddir/files/share/app-info/media