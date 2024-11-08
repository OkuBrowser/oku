#!/bin/sh
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest ./build-aux/io.github.OkuBrowser.oku.json
flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo repo