#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PAYLOAD_DIR="$ROOT_DIR/packaging/windows/payload"
BUILD_DIR="$ROOT_DIR/target/portable"

RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}
ARCH=${ARCH:-x86_64}
BINARY_PATH=${BINARY_PATH:-"$PAYLOAD_DIR/Davenstein.exe"}
ASSETS_PATH=${ASSETS_PATH:-"$PAYLOAD_DIR/assets.pak"}
README_PATH=${README_PATH:-"$PAYLOAD_DIR/README.md"}
ICON_PATH=${ICON_PATH:-"$ROOT_DIR/packaging/windows/Davenstein.ico"}
ARCHIVE_BASENAME="Davenstein-${RELEASE_VERSION}-windows-${ARCH}"
STAGE_DIR="$BUILD_DIR/$ARCHIVE_BASENAME"
ARCHIVE_PATH="$BUILD_DIR/$ARCHIVE_BASENAME-portable.zip"

if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein version\n' >&2
    exit 1
fi

for required_file in \
    "$BINARY_PATH" \
    "$ASSETS_PATH" \
    "$README_PATH" \
    "$ROOT_DIR/LICENSE.md" \
    "$ROOT_DIR/LICENSE-MIT" \
    "$ROOT_DIR/LICENSE-APACHE" \
    "$ROOT_DIR/COPYRIGHT.md" \
    "$ROOT_DIR/THIRD_PARTY_ASSETS.md" \
    "$ICON_PATH"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required portable-build input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

command -v zip >/dev/null 2>&1 || {
    printf 'zip is required to build the Windows portable archive\n' >&2
    exit 1
}

rm -rf "$STAGE_DIR"
rm -f "$ARCHIVE_PATH" "$ARCHIVE_PATH.sha256"

install -d "$STAGE_DIR"

install -m 755 \
    "$BINARY_PATH" \
    "$STAGE_DIR/Davenstein.exe"

install -m 644 \
    "$ASSETS_PATH" \
    "$STAGE_DIR/assets.pak"

install -m 644 \
    "$README_PATH" \
    "$STAGE_DIR/README.md"

# Include Software Licenses, Copyright, and Third-Party Asset Information
for legal_file in \
    LICENSE.md \
    LICENSE-MIT \
    LICENSE-APACHE \
    COPYRIGHT.md \
    THIRD_PARTY_ASSETS.md
do
    install -m 644 \
        "$ROOT_DIR/$legal_file" \
        "$STAGE_DIR/$legal_file"
done

install -m 644 \
    "$ICON_PATH" \
    "$STAGE_DIR/Davenstein.ico"

: > "$STAGE_DIR/portable.flag"
chmod 644 "$STAGE_DIR/portable.flag"

cd "$BUILD_DIR"
zip -qr \
    "$ARCHIVE_BASENAME-portable.zip" \
    "$ARCHIVE_BASENAME"

sha256sum "$ARCHIVE_BASENAME-portable.zip" \
    > "$ARCHIVE_BASENAME-portable.zip.sha256"

printf 'Created %s\n' "$ARCHIVE_PATH"
printf 'Created %s\n' "$ARCHIVE_PATH.sha256"
