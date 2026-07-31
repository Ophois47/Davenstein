#!/usr/bin/env bash

set -euo pipefail

# Resolve The Repository From This Script Instead Of The Calling Directory
script_directory="$(
    cd "$(dirname "${BASH_SOURCE[0]}")"
    pwd
)"
repository_root="$(
    cd "$script_directory/.."
    pwd
)"

# Keep The Temporary Asset Packer Under target So Git Never Sees Build Output
tool_directory="${CARGO_TARGET_DIR:-$repository_root/target}/ios-tools"
asset_packer="$tool_directory/pak_builder"
asset_output="$script_directory/ios-src/assets.pak"

mkdir -p "$tool_directory"

# Build The Existing Repository Asset Packer Without Creating Cargo.lock
rustc \
    --edition=2024 \
    -O \
    "$repository_root/src/pak_builder.rs" \
    -o "$asset_packer"

# Produce The Exact assets.pak Copied By The Xcode Resources Build Phase
"$asset_packer" \
    --root "$repository_root/assets" \
    --out "$asset_output"

test -s "$asset_output"

printf 'Built %s\n' "$asset_output"
