#!/usr/bin/env bash

set -euo pipefail

sudo apt-get update

sudo apt-get install -y \
    build-essential \
    pkg-config \
    libwayland-dev \
    libasound2-dev \
    libudev-dev

echo
echo "##=>> Verifying Native Davenstein Build Dependencies ..."

pkg-config --exists wayland-client
pkg-config --exists alsa
pkg-config --exists libudev

echo "##=>> All Native Davenstein Build Dependencies Installed!"
