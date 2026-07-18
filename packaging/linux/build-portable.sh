#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds a Self-Contained Portable Linux Release Archive From an Existing
# Davenstein Release Binary and DVPK Asset Package
#
# Portable Package Contents:
#     - Davenstein Release Executable
#     - assets.pak
#     - README.md
#     - Davenstein Application Icon
#     - portable.flag
#     - run-davenstein.sh Launcher
#
# portable.flag Enables Davenstein's Portable Storage Mode so Saves, High
# Scores, and Other Writable Data Remain With the Extracted Application
#
# Package Construction Uses a Temporary Staging Directory That is Removed
# Automatically When the Script Exits or is Interrupted
#
# Archive Metadata Uses Numeric Root Ownership to Produce Consistent Release
# Archives Regardless of the Local User That Runs the Build
#
# Release Automation May Override:
#     VERSION              Complete Release Version or Git Tag
#     ARCH                 Target Linux Architecture
#     BINARY_PATH          Existing Davenstein Release Binary
#
# Portable Release Output:
#     Davenstein-<version>-linux-<architecture>.tar.gz
#     Davenstein-<version>-linux-<architecture>.tar.gz.sha256
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/portable"

# Create Isolated Temporary Staging Root for Portable Package Assembly
STAGE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/davenstein-portable.XXXXXX")

# Release Version
# Release Automation May Override VERSION With the Complete Git Tag Version
RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}

# Target Architecture and Existing Release Binary
ARCH=${ARCH:-x86_64}
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/release/Davenstein"}

# Versioned Architecture-Specific Portable Archive Paths
ARCHIVE_BASENAME="Davenstein-${RELEASE_VERSION}-linux-${ARCH}"
STAGE_DIR="$STAGE_ROOT/$ARCHIVE_BASENAME"
ARCHIVE_PATH="$BUILD_DIR/$ARCHIVE_BASENAME.tar.gz"

# Remove Temporary Staging Files on Normal Exit or Interruption
trap 'rm -rf "$STAGE_ROOT"' EXIT HUP INT TERM

# Refuse Unversioned Portable Release Output
if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein version\n' >&2
    exit 1
fi

# Validate Every Required Portable Package Input Before Modifying Output
for required_file in \
    "$BINARY_PATH" \
    "$ROOT_DIR/target/release/assets.pak" \
    "$ROOT_DIR/README.md" \
    "$ROOT_DIR/LICENSE.md" \
    "$ROOT_DIR/LICENSE-MIT" \
    "$ROOT_DIR/LICENSE-APACHE" \
    "$ROOT_DIR/COPYRIGHT.md" \
    "$ROOT_DIR/THIRD_PARTY_ASSETS.md" \
    "$ROOT_DIR/packaging/linux/davenstein.png"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required portable-build input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

# Remove Existing Archive and Checksum for Selected Version and Architecture
rm -f "$ARCHIVE_PATH" "$ARCHIVE_PATH.sha256"

# Create Persistent Output Directory and Temporary Package Root
install -d "$BUILD_DIR"
install -d -m 755 "$STAGE_DIR"

# Install Davenstein Release Executable
install -m 755 \
    "$BINARY_PATH" \
    "$STAGE_DIR/Davenstein"

# Keep assets.pak Beside Executable for Runtime Package Resolution
install -m 644 \
    "$ROOT_DIR/target/release/assets.pak" \
    "$STAGE_DIR/assets.pak"

# Include Project README With Portable Release
install -m 644 \
    "$ROOT_DIR/README.md" \
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

# Include Davenstein Application Icon With Portable Release
install -m 644 \
    "$ROOT_DIR/packaging/linux/davenstein.png" \
    "$STAGE_DIR/davenstein.png"

# Enable Portable Storage Mode Beside Davenstein Executable
: > "$STAGE_DIR/portable.flag"

# Create Launcher That Runs Davenstein From Extracted Package Directory
# This Preserves Relative Runtime Paths Regardless of the Caller Working Directory
cat > "$STAGE_DIR/run-davenstein.sh" <<'LAUNCHER'
#!/bin/sh
set -eu

# Resolve and Enter Extracted Portable Package Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

# Replace Launcher Process With Davenstein and Forward All Arguments
exec ./Davenstein "$@"
LAUNCHER

# Apply Executable and Data File Permissions
chmod 755 "$STAGE_DIR/run-davenstein.sh"
chmod 644 "$STAGE_DIR/portable.flag"

# Create Compressed Portable Archive With Reproducible Numeric Ownership
tar \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    -C "$STAGE_ROOT" \
    -czf "$ARCHIVE_PATH" \
    "$ARCHIVE_BASENAME"

# Generate Matching SHA-256 Checksum From Portable Output Directory
cd "$BUILD_DIR"
sha256sum "$ARCHIVE_BASENAME.tar.gz" \
    > "$ARCHIVE_BASENAME.tar.gz.sha256"

printf 'Created %s\n' "$ARCHIVE_PATH"
printf 'Created %s\n' "$ARCHIVE_PATH.sha256"
