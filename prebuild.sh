#!/bin/sh
vox build ./src/browser_pages
glib-compile-resources --target="resources.gresource" --sourcedir="data/hicolor/scalable/actions" "resources.gresource.xml"