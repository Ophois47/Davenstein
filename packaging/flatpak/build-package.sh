#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds a Native Flatpak Bundle From the Current Davenstein Checkout
#
# The Same Builder Runs Natively on x86_64 and AArch64 Hosts
# GitHub Actions Uses a Matching Native Runner for Each Public Bundle Because
# Flatpak Only Supports Host-Compatible Build Architectures
#
# The Manifest Performs a Locked Offline Cargo Build Using cargo-sources.json
# Regenerate That File Whenever Cargo.lock Changes
#
# Installed Layout:
#     /app/bin/Davenstein
#     /app/bin/assets.pak
#     /app/share/applications/io.github.ophois47.davenstein.desktop
#     /app/share/icons/hicolor/256x256/apps/io.github.ophois47.davenstein.png
#     /app/share/metainfo/io.github.ophois47.davenstein.metainfo.xml
#
# No portable.flag is Installed, Preserving Installed Storage Mode
#
# Release Automation May Override:
#     VERSION          Complete Release Version or Git Tag
#     FLATPAK_ARCH     Native Flatpak Architecture
#
# Flatpak Output:
#     Davenstein-<version>-linux-<architecture>.flatpak
#     Davenstein-<version>-linux-<architecture>.flatpak.sha256
#

# Resolve Repository Paths Relative to this Script
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
OUTPUT_DIR="$ROOT_DIR/target/flatpak"

APP_ID="io.github.ophois47.davenstein"
FLATPAK_BRANCH="stable"
RUNTIME_VERSION="25.08"

MANIFEST_PATH="$SCRIPT_DIR/$APP_ID.yml"
DESKTOP_PATH="$SCRIPT_DIR/$APP_ID.desktop"
METAINFO_PATH="$SCRIPT_DIR/$APP_ID.metainfo.xml"

# Resolve Release Version From Environment or Cargo Package Metadata
RELEASE_VERSION=${VERSION:-$(sed -nE \
    's/^version = "([^"]+)"/\1/p' \
    "$ROOT_DIR/Cargo.toml" |
    head -n 1)}

# Git Tags May Prefix the Release Version With v
RELEASE_VERSION=${RELEASE_VERSION#v}

if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein release version\n' >&2
    exit 1
fi

# Build Only the Native Architecture Reported by Flatpak Unless CI Overrides It
FLATPAK_ARCH=${FLATPAK_ARCH:-$(flatpak --default-arch)}

case "$FLATPAK_ARCH" in
    x86_64 | aarch64)
        ;;
    *)
        printf 'Unsupported Flatpak architecture: %s\n' "$FLATPAK_ARCH" >&2
        exit 1
        ;;
esac

OUTPUT_BASENAME="Davenstein-${RELEASE_VERSION}-linux-${FLATPAK_ARCH}.flatpak"
OUTPUT_PATH="$OUTPUT_DIR/$OUTPUT_BASENAME"

# Keep State, Build Output, and the Temporary Repository on One Filesystem
# This Also Avoids Windows-Mount Semantics During Local WSL Builds
TEMP_PARENT=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
TEMP_ROOT=$(mktemp -d "$TEMP_PARENT/davenstein-flatpak.XXXXXX")
STATE_DIR="$TEMP_ROOT/state"
APP_BUILD_DIR="$TEMP_ROOT/build"
REPO_DIR="$TEMP_ROOT/repo"

# Remove Temporary Flatpak State on Normal Exit or Interruption
trap 'rm -rf "$TEMP_ROOT"' EXIT HUP INT TERM

# Validate Required Packaging Tools Before Modifying Output
for required_command in \
    appstreamcli \
    chmod \
    desktop-file-validate \
    eu-elfcompress \
    eu-strip \
    flatpak \
    flatpak-builder \
    grep \
    head \
    install \
    mktemp \
    python3 \
    rm \
    sed \
    sha256sum
do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        printf 'Required Flatpak packaging tool was not found: %s\n' \
            "$required_command" >&2
        exit 1
    fi
done

# Confirm the Cargo Sources Generator Python Dependencies Are Present
if ! python3 -c 'import aiohttp, tomlkit, yaml' >/dev/null 2>&1; then
    printf 'Flatpak Cargo sources generator needs Python packages aiohttp, tomlkit, and PyYAML\n' >&2
    printf 'Install them with: pip install aiohttp tomlkit PyYAML\n' >&2
    exit 1
fi

# Regenerate the Vendored Cargo Sources From Cargo.lock so the Offline Build
# Always Matches the Committed Lockfile, Never a Stale Committed Copy
printf 'Regenerating cargo-sources.json from Cargo.lock\n'
python3 "$SCRIPT_DIR/flatpak-cargo-generator.py" \
    "$ROOT_DIR/Cargo.lock" \
    -o "$SCRIPT_DIR/cargo-sources.json"

# Validate Manifest Inputs and All Published Documentation
for required_file in \
    "$MANIFEST_PATH" \
    "$DESKTOP_PATH" \
    "$METAINFO_PATH" \
    "$SCRIPT_DIR/cargo-sources.json" \
    "$ROOT_DIR/Cargo.lock" \
    "$ROOT_DIR/README.md" \
    "$ROOT_DIR/LICENSE.md" \
    "$ROOT_DIR/LICENSE-MIT" \
    "$ROOT_DIR/LICENSE-APACHE" \
    "$ROOT_DIR/COPYRIGHT.md" \
    "$ROOT_DIR/THIRD_PARTY_ASSETS.md" \
    "$ROOT_DIR/packaging/linux/davenstein.png"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required Flatpak-build input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

# Prevent Tagged Builds From Publishing Stale AppStream Release Metadata
if ! grep -Fq \
    "<release version=\"$RELEASE_VERSION\"" \
    "$METAINFO_PATH"
then
    printf 'Flatpak MetaInfo does not describe release %s\n' \
        "$RELEASE_VERSION" >&2
    exit 1
fi

desktop-file-validate "$DESKTOP_PATH"
appstreamcli validate --pedantic "$METAINFO_PATH"

# Require the Runtime, SDK, and Rust Extension Declared by the Manifest
for runtime_id in \
    org.freedesktop.Platform \
    org.freedesktop.Sdk \
    org.freedesktop.Sdk.Extension.rust-stable
do
    if ! flatpak info \
        --user \
        --arch="$FLATPAK_ARCH" \
        "$runtime_id//$RUNTIME_VERSION" \
        >/dev/null 2>&1
    then
        printf 'Required Flatpak runtime was not installed: %s/%s/%s\n' \
            "$runtime_id" "$FLATPAK_ARCH" "$RUNTIME_VERSION" >&2
        exit 1
    fi
done

# Reconstruct Public Output From Scratch
rm -f "$OUTPUT_PATH" "$OUTPUT_PATH.sha256"
install -d "$OUTPUT_DIR"

# Build and Export Into a Temporary Local OSTree Repository
flatpak-builder \
    --user \
    --force-clean \
    --arch="$FLATPAK_ARCH" \
    --state-dir="$STATE_DIR" \
    --repo="$REPO_DIR" \
    "$APP_BUILD_DIR" \
    "$MANIFEST_PATH"

# Create the Single-File Bundle Published on GitHub Releases
flatpak build-bundle \
    --arch="$FLATPAK_ARCH" \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    "$REPO_DIR" \
    "$OUTPUT_PATH" \
    "$APP_ID" \
    "$FLATPAK_BRANCH"

# Normalize Public Artifact Permissions
chmod 644 "$OUTPUT_PATH"

# Generate and Verify a Basename-Only SHA-256 Checksum
cd "$OUTPUT_DIR"
sha256sum "$OUTPUT_BASENAME" > "$OUTPUT_BASENAME.sha256"
chmod 644 "$OUTPUT_BASENAME.sha256"
sha256sum --check "$OUTPUT_BASENAME.sha256"

printf 'Created %s\n' "$OUTPUT_PATH"
printf 'Created %s\n' "$OUTPUT_PATH.sha256"
