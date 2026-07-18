#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds Davenstein's Linux AppImage Package From the Current Release Binary
# and a Newly Generated DVPK Asset Archive
#
# Supported Architectures:
#     - x86_64
#     - aarch64
#
# Build Process:
#     - Resolve Repository Paths Relative to this Script
#     - Select and Verify Architecture-Matched linuxdeploy Tooling
#     - Build Davenstein With the Committed Cargo Lock File
#     - Rebuild pak_builder From Current Repository Source
#     - Generate a Fresh assets.pak
#     - Construct a Clean AppDir
#     - Package the AppDir as a Versioned AppImage
#     - Generate a Matching SHA-256 Checksum
#
# linuxdeploy is Pinned to a Dated Upstream Release and Verified Against an
# Architecture-Specific Checksum Stored in the Repository
#
# Downloaded Third-Party Tooling and Temporary Packaging Files Remain Beneath
# target/appimage so Generated Files Never Enter the Repository
#
# Release Automation May Override:
#     ARCH                 Target AppImage Architecture
#     VERSION              Complete Release Version or Git Tag
#     LINUXDEPLOY          Custom linuxdeploy Executable
#
# AppImage Output:
#     Davenstein-<version>-<architecture>.AppImage
#     Davenstein-<version>-<architecture>.AppImage.sha256
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/appimage"
APPDIR="$BUILD_DIR/Davenstein.AppDir"
TOOLS_DIR="$BUILD_DIR/tools"

# Default to x86_64 for Existing Local and CI Callers
ARCH=${ARCH:-x86_64}

# Supported AppImage Architectures
# Each Architecture Requires Pinned linuxdeploy Tooling and Validated Packaging
case "$ARCH" in
    x86_64|aarch64)
        ;;
    *)
        printf 'Unsupported AppImage Architecture: %s\n' "$ARCH" >&2
        exit 1
        ;;
esac

# Architecture-Matched linuxdeploy Tooling
# Prevents Host Tools From Being Mixed Across Package Architectures
LINUXDEPLOY_FILENAME="linuxdeploy-${ARCH}.AppImage"
DEFAULT_LINUXDEPLOY="$TOOLS_DIR/$LINUXDEPLOY_FILENAME"
LINUXDEPLOY=${LINUXDEPLOY:-"$DEFAULT_LINUXDEPLOY"}

# Pinned linuxdeploy Release
# Dated Release Prevents Mutable Continuous Builds From Changing Unexpectedly
LINUXDEPLOY_RELEASE="1-alpha-20251107-1"
LINUXDEPLOY_URL="https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_RELEASE/$LINUXDEPLOY_FILENAME"

# Architecture-Specific Upstream linuxdeploy Checksum
LINUXDEPLOY_CHECKSUM_FILE="$ROOT_DIR/packaging/linux/linuxdeploy-${ARCH}.sha256"

# Release Version
# Release Automation May Override VERSION With the Complete Git Tag Version
RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}

# Versioned Architecture-Specific AppImage Output Name
OUTPUT_NAME="Davenstein-${RELEASE_VERSION}-${ARCH}.AppImage"

# Prevent Generic VERSION Variable From Affecting appimagetool Behavior
unset VERSION

# Downloads the Pinned Architecture-Matched linuxdeploy Release
# Temporary Download is Moved Into Place Only After curl Completes Successfully
download_linuxdeploy() {
    temporary_path="${DEFAULT_LINUXDEPLOY}.download"

    # curl is Required Only When Automatic Tool Provisioning is Needed
    command -v curl >/dev/null 2>&1 || {
        printf 'curl is Required to Download linuxdeploy\n' >&2
        exit 1
    }

    mkdir -p "$TOOLS_DIR"
    rm -f "$temporary_path"

    printf 'Downloading linuxdeploy %s\n' "$LINUXDEPLOY_RELEASE"

    # Retry Transient Network Failures Without Retaining Partial Tool Downloads
    curl -fL \
        --retry 3 \
        --retry-delay 2 \
        -o "$temporary_path" \
        "$LINUXDEPLOY_URL"

    chmod +x "$temporary_path"
    mv "$temporary_path" "$DEFAULT_LINUXDEPLOY"
}

# Verifies Automatically Managed linuxdeploy Against Repository Checksum
verify_default_linuxdeploy() {
    # Checksum File Must Exist for the Selected Architecture
    if [ ! -f "$LINUXDEPLOY_CHECKSUM_FILE" ]; then
        printf 'linuxdeploy Checksum File Not Found at %s\n' \
            "$LINUXDEPLOY_CHECKSUM_FILE" >&2
        return 1
    fi

    # Read First Nonempty Checksum and Compare it With Downloaded Executable
    expected_checksum=$(awk 'NF { print $1; exit }' "$LINUXDEPLOY_CHECKSUM_FILE")
    actual_checksum=$(sha256sum "$DEFAULT_LINUXDEPLOY" | awk '{ print $1 }')

    if [ -z "$expected_checksum" ]; then
        printf 'linuxdeploy Checksum File is Empty\n' >&2
        return 1
    fi

    if [ "$actual_checksum" != "$expected_checksum" ]; then
        printf 'linuxdeploy Checksum Mismatch\n' >&2
        printf 'Expected: %s\n' "$expected_checksum" >&2
        printf 'Actual:   %s\n' "$actual_checksum" >&2
        return 1
    fi
}

# Automatically Provision and Verify Default linuxdeploy Tooling
if [ "$LINUXDEPLOY" = "$DEFAULT_LINUXDEPLOY" ]; then
    # Download Tool When No Executable Cached Copy Exists
    if [ ! -x "$DEFAULT_LINUXDEPLOY" ]; then
        download_linuxdeploy
    fi

    # Replace Cached Tool When Checksum Verification Fails
    if ! verify_default_linuxdeploy; then
        printf 'Replacing Invalid linuxdeploy Download\n' >&2
        rm -f "$DEFAULT_LINUXDEPLOY"
        download_linuxdeploy

        # Refuse Packaging When Fresh Download Still Fails Verification
        if ! verify_default_linuxdeploy; then
            printf 'Downloaded linuxdeploy Failed Checksum Verification\n' >&2
            exit 1
        fi
    fi
elif [ ! -x "$LINUXDEPLOY" ]; then
    # Custom Tool Overrides Must Refer to an Existing Executable
    printf 'Custom linuxdeploy Executable Not Found at %s\n' \
        "$LINUXDEPLOY" >&2
    exit 1
fi

# Refuse Unversioned AppImage Output
if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could Not Determine Davenstein Version\n' >&2
    exit 1
fi

cd "$ROOT_DIR"

# Build Current Davenstein Release Binary
# --locked Requires Dependency Resolution From the Committed Cargo.lock
cargo build --release --locked --bin Davenstein

mkdir -p target/release

# Build Standalone DVPK Generator From Current Repository Source
rustc --edition=2024 -O \
    src/pak_builder.rs \
    -o target/release/pak_builder

# Generate Fresh Asset Package From Complete Current Asset Tree
./target/release/pak_builder \
    --root assets \
    --out target/release/assets.pak

# Reconstruct AppDir From Scratch
# Prevents Files From Earlier Builds Surviving Into New Packages
rm -rf "$APPDIR"

# Remove Existing Outputs Only for Selected Architecture
# Other Architecture Package Families Remain Unchanged
rm -f \
    "$BUILD_DIR"/Davenstein-"$ARCH".AppImage \
    "$BUILD_DIR"/Davenstein-*-"$ARCH".AppImage \
    "$BUILD_DIR"/Davenstein-*-"$ARCH".AppImage.sha256

mkdir -p "$APPDIR/usr/bin"

# Install Release Binary Into AppDir
install -m 755 \
    target/release/Davenstein \
    "$APPDIR/usr/bin/Davenstein"

# Keep assets.pak Beside Executable for Runtime Package Resolution
install -m 644 \
    target/release/assets.pak \
    "$APPDIR/usr/bin/assets.pak"

cd "$BUILD_DIR"

# Build AppImage With Clean Linux Tool Search Path
# Excludes WSL-Injected Windows Paths While linuxdeploy Searches for Plugins
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
APPIMAGE_EXTRACT_AND_RUN=1 \
LINUXDEPLOY_OUTPUT_VERSION="$RELEASE_VERSION" \
"$LINUXDEPLOY" \
    --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/Davenstein" \
    --desktop-file "$ROOT_DIR/packaging/linux/davenstein.desktop" \
    --icon-file "$ROOT_DIR/packaging/linux/davenstein.png" \
    --output appimage

# Verify Versioned AppImage Deliverable Was Created
if [ ! -f "$OUTPUT_NAME" ]; then
    printf 'Expected AppImage was not created at %s\n' "$BUILD_DIR/$OUTPUT_NAME" >&2
    exit 1
fi

# Generate Matching SHA-256 Checksum for Release Verification
sha256sum "$OUTPUT_NAME" > "$OUTPUT_NAME.sha256"

printf 'Created %s\n' "$BUILD_DIR/$OUTPUT_NAME"
printf 'Created %s\n' "$BUILD_DIR/$OUTPUT_NAME.sha256"
