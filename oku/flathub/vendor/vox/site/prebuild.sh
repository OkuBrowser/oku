#!/bin/sh
mkdir -p ./output/
grass ./assets/style/styles.scss > ./output/styles.css
cp ./assets/favicon/* ./output
cp ./assets/logo.png ./output