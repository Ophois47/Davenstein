#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
PAYLOAD_DIR="$ROOT_DIR/packaging/windows/payload"
BUILD_DIR="$ROOT_DIR/target/portable"

RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}
ARCHIVE_BASENAME="Davenstein-${RELEASE_VERSION}-windows-x86_64"
STAGE_DIR="$BUILD_DIR/$ARCHIVE_BASENAME"
ARCHIVE_PATH="$BUILD_DIR/$ARCHIVE_BASENAME-portable.zip"

if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein version\n' >&2
    exit 1
fi

for required_file in \
    "$PAYLOAD_DIR/Davenstein.exe" \
    "$PAYLOAD_DIR/assets.pak" \
    "$PAYLOAD_DIR/README.md" \
    "$ROOT_DIR/packaging/windows/Davenstein.ico"
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
    "$PAYLOAD_DIR/Davenstein.exe" \
    "$STAGE_DIR/Davenstein.exe"

install -m 644 \
    "$PAYLOAD_DIR/assets.pak" \
    "$STAGE_DIR/assets.pak"

install -m 644 \
    "$PAYLOAD_DIR/README.md" \
    "$STAGE_DIR/README.md"

install -m 644 \
    "$ROOT_DIR/packaging/windows/Davenstein.ico" \
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
