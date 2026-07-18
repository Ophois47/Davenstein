#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds a Native macOS Application Bundle From an Existing Davenstein
# Executable and a Previously Generated DVPK Asset Archive
#
# Application Bundle Construction Process:
#     - Resolve Repository and Packaging Paths Relative to this Script
#     - Validate Cargo and Bundle Version Values
#     - Validate Required Bundle Inputs and Native macOS Tools
#     - Construct a Clean Davenstein.app Bundle
#     - Install Davenstein and assets.pak Under Contents/MacOS
#     - Generate a Complete macOS Icon Set From the Source PNG
#     - Convert the Icon Set Into Davenstein.icns
#     - Generate Info.plist From the Versioned Template
#     - Archive the Application Bundle With ditto
#     - Generate a Matching SHA-256 Checksum
#
# Application Bundle Layout:
#     Davenstein.app/Contents/Info.plist
#     Davenstein.app/Contents/MacOS/Davenstein
#     Davenstein.app/Contents/MacOS/assets.pak
#     Davenstein.app/Contents/Resources/Davenstein.icns
#
# assets.pak Remains Beside the Executable Because the Davenstein Runtime
# Resolves the DVPK Asset Package Relative to the Executable Location
#
# Release Automation May Override:
#     VERSION                  Complete Release Version or Git Tag
#     BUNDLE_SHORT_VERSION     CFBundleShortVersionString Value
#     BUNDLE_VERSION           CFBundleVersion Value
#     ARCH                     Public Release Architecture Name
#     BINARY_PATH              Existing macOS Davenstein Executable
#     ASSETS_PATH              Existing DVPK Asset Package
#
# macOS Release Output:
#     Davenstein-<version>-macos-<architecture>.zip
#     Davenstein-<version>-macos-<architecture>.zip.sha256
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

# macOS Bundle Build and Directory Layout
BUILD_DIR="$ROOT_DIR/target/macos"
APP_BUNDLE="$BUILD_DIR/Davenstein.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
DOCUMENTATION_DIR="$RESOURCES_DIR/Documentation"
ICONSET_DIR="$BUILD_DIR/Davenstein.iconset"

# macOS Bundle Source Files
ICON_SOURCE="$SCRIPT_DIR/Davenstein.png"
PLIST_TEMPLATE="$SCRIPT_DIR/Info.plist"

# Cargo Package Version Used as Default for Release and Bundle Metadata
CARGO_VERSION=$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)

# Release and macOS Bundle Versions
RELEASE_VERSION=${VERSION:-"$CARGO_VERSION"}
BUNDLE_SHORT_VERSION=${BUNDLE_SHORT_VERSION:-"$CARGO_VERSION"}
BUNDLE_VERSION=${BUNDLE_VERSION:-"$CARGO_VERSION"}

# Public Release Architecture and Existing Bundle Inputs
ARCH=${ARCH:-aarch64}
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/release/Davenstein"}
ASSETS_PATH=${ASSETS_PATH:-"$ROOT_DIR/target/release/assets.pak"}

# Versioned Architecture-Specific macOS Archive Paths
OUTPUT_BASENAME="Davenstein-${RELEASE_VERSION}-macos-${ARCH}"
ZIP_PATH="$BUILD_DIR/$OUTPUT_BASENAME.zip"

# Validate Every Required Version Value Before Modifying Build Output
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

# CFBundleShortVersionString Accepts One to Three Numeric Components
if ! printf '%s\n' "$BUNDLE_SHORT_VERSION" |
    grep -Eq '^[0-9]+(\.[0-9]+){0,2}$'
then
    printf 'Invalid CFBundleShortVersionString: %s\n' \
        "$BUNDLE_SHORT_VERSION" >&2
    exit 1
fi

# CFBundleVersion Accepts One or More Numeric Components
if ! printf '%s\n' "$BUNDLE_VERSION" |
    grep -Eq '^[0-9]+(\.[0-9]+)*$'
then
    printf 'Invalid CFBundleVersion: %s\n' \
        "$BUNDLE_VERSION" >&2
    exit 1
fi

# Validate Every Required Bundle Input Before Modifying Build Output
for required_file in \
    "$BINARY_PATH" \
    "$ASSETS_PATH" \
    "$ROOT_DIR/LICENSE.md" \
    "$ROOT_DIR/LICENSE-MIT" \
    "$ROOT_DIR/LICENSE-APACHE" \
    "$ROOT_DIR/COPYRIGHT.md" \
    "$ROOT_DIR/THIRD_PARTY_ASSETS.md" \
    "$ICON_SOURCE" \
    "$PLIST_TEMPLATE"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required macOS bundle input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

# Validate Native macOS Bundle and Icon Utilities
for required_command in ditto sips iconutil
do
    command -v "$required_command" >/dev/null 2>&1 || {
        printf '%s is required to build the macOS application\n' \
            "$required_command" >&2
        exit 1
    }
done

# Remove Previous Application Bundle, Temporary Icon Set, Archive, and Checksum
rm -rf "$APP_BUNDLE" "$ICONSET_DIR"
rm -f "$ZIP_PATH" "$ZIP_PATH.sha256"

# Construct Clean macOS Application Bundle Directory Layout
install -d "$BUILD_DIR"
install -d -m 755 "$MACOS_DIR"
install -d -m 755 "$RESOURCES_DIR"
install -d -m 755 "$DOCUMENTATION_DIR"
install -d -m 755 "$ICONSET_DIR"

# Install Davenstein Executable Into Application Bundle
install -m 755 \
    "$BINARY_PATH" \
    "$MACOS_DIR/Davenstein"

# Keep assets.pak Beside Executable for Runtime Package Resolution
install -m 644 \
    "$ASSETS_PATH" \
    "$MACOS_DIR/assets.pak"

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
        "$DOCUMENTATION_DIR/$legal_file"
done

# Generates One Required macOS Icon Image From the Source PNG
generate_icon() {
    size=$1
    filename=$2

    sips \
        -z "$size" "$size" \
        "$ICON_SOURCE" \
        --out "$ICONSET_DIR/$filename" \
        >/dev/null
}

# Generate Standard and Retina Icon Sizes Required by iconutil
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

# Convert Complete Icon Set Into Native macOS ICNS Resource
iconutil \
    -c icns \
    "$ICONSET_DIR" \
    -o "$RESOURCES_DIR/Davenstein.icns"

# Remove Temporary Icon Set After ICNS Generation
rm -rf "$ICONSET_DIR"

# Generate Info.plist With Validated Bundle Version Metadata
sed \
    -e "s/__BUNDLE_SHORT_VERSION__/$BUNDLE_SHORT_VERSION/g" \
    -e "s/__BUNDLE_VERSION__/$BUNDLE_VERSION/g" \
    "$PLIST_TEMPLATE" \
    > "$CONTENTS_DIR/Info.plist"

# Create Versioned ZIP Archive While Preserving macOS Resource Metadata
cd "$BUILD_DIR"

ditto \
    -c \
    -k \
    --sequesterRsrc \
    --keepParent \
    "Davenstein.app" \
    "$OUTPUT_BASENAME.zip"

# Generate Matching SHA-256 Checksum for Release Verification
shasum -a 256 "$OUTPUT_BASENAME.zip" \
    > "$OUTPUT_BASENAME.zip.sha256"

printf 'Created %s\n' "$APP_BUNDLE"
printf 'Created %s\n' "$ZIP_PATH"
printf 'Created %s\n' "$ZIP_PATH.sha256"
