#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/macos"
APP_BUNDLE="$BUILD_DIR/Davenstein.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
PLIST_TEMPLATE="$SCRIPT_DIR/Info.plist"

CARGO_VERSION=$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)
RELEASE_VERSION=${VERSION:-"$CARGO_VERSION"}
BUNDLE_SHORT_VERSION=${BUNDLE_SHORT_VERSION:-"$CARGO_VERSION"}
BUNDLE_VERSION=${BUNDLE_VERSION:-"$CARGO_VERSION"}
ARCH=${ARCH:-aarch64}
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/release/Davenstein"}
ASSETS_PATH=${ASSETS_PATH:-"$ROOT_DIR/target/release/assets.pak"}

OUTPUT_BASENAME="Davenstein-${RELEASE_VERSION}-macos-${ARCH}"
ZIP_PATH="$BUILD_DIR/$OUTPUT_BASENAME.zip"

for value_name in \
    CARGO_VERSION \
    RELEASE_VERSION \
    BUNDLE_SHORT_VERSION \
    BUNDLE_VERSION
do
    eval "value=\${$value_name}"

    if [ -z "$value" ]; then
        printf '%s could not be determined\n' "$value_name" >&2
        exit 1
    fi
done

if ! printf '%s\n' "$BUNDLE_SHORT_VERSION" |
    grep -Eq '^[0-9]+(\.[0-9]+){0,2}$'
then
    printf 'Invalid CFBundleShortVersionString: %s\n' \
        "$BUNDLE_SHORT_VERSION" >&2
    exit 1
fi

if ! printf '%s\n' "$BUNDLE_VERSION" |
    grep -Eq '^[0-9]+(\.[0-9]+)*$'
then
    printf 'Invalid CFBundleVersion: %s\n' \
        "$BUNDLE_VERSION" >&2
    exit 1
fi

for required_file in \
    "$BINARY_PATH" \
    "$ASSETS_PATH" \
    "$PLIST_TEMPLATE"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required macOS bundle input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

command -v ditto >/dev/null 2>&1 || {
    printf 'ditto is required to create the macOS application archive\n' >&2
    exit 1
}

rm -rf "$APP_BUNDLE"
rm -f "$ZIP_PATH" "$ZIP_PATH.sha256"

install -d "$BUILD_DIR"
install -d -m 755 "$MACOS_DIR"

install -m 755 \
    "$BINARY_PATH" \
    "$MACOS_DIR/Davenstein"

install -m 644 \
    "$ASSETS_PATH" \
    "$MACOS_DIR/assets.pak"

sed \
    -e "s/__BUNDLE_SHORT_VERSION__/$BUNDLE_SHORT_VERSION/g" \
    -e "s/__BUNDLE_VERSION__/$BUNDLE_VERSION/g" \
    "$PLIST_TEMPLATE" \
    > "$CONTENTS_DIR/Info.plist"

cd "$BUILD_DIR"

ditto \
    -c \
    -k \
    --sequesterRsrc \
    --keepParent \
    "Davenstein.app" \
    "$OUTPUT_BASENAME.zip"

shasum -a 256 "$OUTPUT_BASENAME.zip" \
    > "$OUTPUT_BASENAME.zip.sha256"

printf 'Created %s\n' "$APP_BUNDLE"
printf 'Created %s\n' "$ZIP_PATH"
printf 'Created %s\n' "$ZIP_PATH.sha256"
