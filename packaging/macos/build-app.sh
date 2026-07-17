#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/macos"
APP_BUNDLE="$BUILD_DIR/Davenstein.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
ICONSET_DIR="$BUILD_DIR/Davenstein.iconset"
ICON_SOURCE="$SCRIPT_DIR/Davenstein.png"
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
    "$ICON_SOURCE" \
    "$PLIST_TEMPLATE"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required macOS bundle input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

for required_command in ditto sips iconutil
do
    command -v "$required_command" >/dev/null 2>&1 || {
        printf '%s is required to build the macOS application\n' \
            "$required_command" >&2
        exit 1
    }
done

rm -rf "$APP_BUNDLE" "$ICONSET_DIR"
rm -f "$ZIP_PATH" "$ZIP_PATH.sha256"

install -d "$BUILD_DIR"
install -d -m 755 "$MACOS_DIR"
install -d -m 755 "$RESOURCES_DIR"
install -d -m 755 "$ICONSET_DIR"

install -m 755 \
    "$BINARY_PATH" \
    "$MACOS_DIR/Davenstein"

install -m 644 \
    "$ASSETS_PATH" \
    "$MACOS_DIR/assets.pak"

generate_icon() {
    size=$1
    filename=$2

    sips \
        -z "$size" "$size" \
        "$ICON_SOURCE" \
        --out "$ICONSET_DIR/$filename" \
        >/dev/null
}

generate_icon 16 icon_16x16.png
generate_icon 32 icon_16x16@2x.png
generate_icon 32 icon_32x32.png
generate_icon 64 icon_32x32@2x.png
generate_icon 128 icon_128x128.png
generate_icon 256 icon_128x128@2x.png
generate_icon 256 icon_256x256.png
generate_icon 512 icon_256x256@2x.png
generate_icon 512 icon_512x512.png
generate_icon 1024 icon_512x512@2x.png

iconutil \
    -c icns \
    "$ICONSET_DIR" \
    -o "$RESOURCES_DIR/Davenstein.icns"

rm -rf "$ICONSET_DIR"

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
