#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Archives an Existing macOS Application Bundle and Generates a Matching
# SHA-256 Checksum
#
# This Script Does Not Construct, Sign, Notarize, or Staple the Application
# Bundle. Release Automation Must Complete Those Operations Before Creating
# the Final Distribution Archive
#
# Release Automation May Override:
#     VERSION                  Complete Release Version or Git Tag
#     ARCH                     Public Release Architecture Name
#     APP_BUNDLE              Existing Davenstein.app Bundle
#     OUTPUT_DIR              Archive and Checksum Output Directory
#
# macOS Release Output:
#     Davenstein-<version>-macos-<architecture>.zip
#     Davenstein-<version>-macos-<architecture>.zip.sha256
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

# Cargo Package Version Used as the Default Release Version
CARGO_VERSION=$(
    sed -nE \
        's/^version = "([^"]+)"/\1/p' \
        "$ROOT_DIR/Cargo.toml" |
        head -n 1
)

# Release Metadata and Existing Application Bundle
RELEASE_VERSION=${VERSION:-"$CARGO_VERSION"}
ARCH=${ARCH:-aarch64}
APP_BUNDLE=${APP_BUNDLE:-"$ROOT_DIR/target/macos/Davenstein.app"}
OUTPUT_DIR=${OUTPUT_DIR:-"$ROOT_DIR/target/macos"}

# Versioned Architecture-Specific macOS Archive Paths
OUTPUT_BASENAME="Davenstein-${RELEASE_VERSION}-macos-${ARCH}"
ZIP_PATH="$OUTPUT_DIR/$OUTPUT_BASENAME.zip"
CHECKSUM_PATH="$ZIP_PATH.sha256"

# Validate Required Release Values
if [ -z "$RELEASE_VERSION" ]; then
    printf '%s\n' 'RELEASE_VERSION could not be determined' >&2
    exit 1
fi

if [ -z "$ARCH" ]; then
    printf '%s\n' 'ARCH must not be empty' >&2
    exit 1
fi

# Reject Characters That Are Unsafe in Release Filenames
case "$RELEASE_VERSION" in
    *[!A-Za-z0-9._+-]*)
        printf 'Invalid release version: %s\n' "$RELEASE_VERSION" >&2
        exit 1
        ;;
esac

case "$ARCH" in
    *[!A-Za-z0-9._-]*)
        printf 'Invalid architecture name: %s\n' "$ARCH" >&2
        exit 1
        ;;
esac

# Validate the Existing Application Bundle Before Archiving
test -d "$APP_BUNDLE" || {
    printf 'Application bundle was not found at %s\n' "$APP_BUNDLE" >&2
    exit 1
}

for required_file in \
    "$APP_BUNDLE/Contents/Info.plist" \
    "$APP_BUNDLE/Contents/MacOS/Davenstein" \
    "$APP_BUNDLE/Contents/Resources/assets.pak"
do
    test -f "$required_file" || {
        printf 'Required application bundle file was not found at %s\n' \
            "$required_file" >&2
        exit 1
    }
done

# Validate Native macOS Archive and Checksum Utilities
for required_command in ditto shasum
do
    command -v "$required_command" >/dev/null 2>&1 || {
        printf '%s is required to archive the macOS application\n' \
            "$required_command" >&2
        exit 1
    }
done

# Create a Clean Versioned Distribution Archive
install -d -m 755 "$OUTPUT_DIR"
rm -f "$ZIP_PATH" "$CHECKSUM_PATH"

ditto \
    -c \
    -k \
    --sequesterRsrc \
    --keepParent \
    "$APP_BUNDLE" \
    "$ZIP_PATH"

# Generate a Portable Checksum Containing Only the Archive Filename
(
    cd "$OUTPUT_DIR"

    shasum -a 256 "$OUTPUT_BASENAME.zip" \
        > "$OUTPUT_BASENAME.zip.sha256"
)

chmod 644 "$ZIP_PATH" "$CHECKSUM_PATH"

printf 'Created %s\n' "$ZIP_PATH"
printf 'Created %s\n' "$CHECKSUM_PATH"
