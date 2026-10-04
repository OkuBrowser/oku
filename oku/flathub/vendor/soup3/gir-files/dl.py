#!/usr/bin/env python3
import os
import shutil

GIR_FILES = [
    "Atk-1.0",
    "cairo-1.0",
    "fontconfig-2.0",
    "freetype2-2.0",
    "Gdk-3.0",
    "Gdk-4.0",
    "GdkPixbuf-2.0",
    "GdkPixdata-2.0",
    "GdkWayland-4.0",
    "GdkX11-3.0",
    "GdkX11-4.0",
    "Gio-2.0",
    "GioUnix-2.0",
    "GL-1.0",
    "GLib-2.0",
    "GLibUnix-2.0",
    "GModule-2.0",
    "GObject-2.0",
    "Graphene-1.0",
    "Gsk-4.0",
    "Gtk-3.0",
    "Gtk-4.0",
    "HarfBuzz-0.0",
    "libxml2-2.0",
    "Pango-1.0",
    "PangoCairo-1.0",
    "PangoFc-1.0",
    "PangoFT2-1.0",
    "PangoOT-1.0",
    "PangoXft-1.0",
    "Vulkan-1.0",
    "win32-1.0",
    "xfixes-4.0",
    "xft-2.0",
    "xlib-2.0",
    "xrandr-1.3",
]

dest_dir = os.path.abspath("./")

for file in GIR_FILES:
    src = f"/usr/share/gir-1.0/{file}.gir"
    dest = f"{dest_dir}/{file}.gir"
    try:
        shutil.copy(src, dest)
    except FileNotFoundError:
        print(f"gir file not found {file}")
