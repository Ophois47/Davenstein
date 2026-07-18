#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds a Self-Contained Portable FreeBSD Release Archive From an Existing
# Cross-Compiled Davenstein Executable and a Previously Generated DVPK Asset
# Archive
#
# Compilation, Target-Aware Stripping, and Packaging Remain Separate so Each
# Stage Can be Tested Independently in Local Builds and Continuous Integration
#
# Portable Package Contents:
#     - Davenstein FreeBSD Executable
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
#     ARCH                 Public Release Architecture Name
#     BINARY_PATH          Existing FreeBSD Davenstein Executable
#     ASSETS_PATH          Existing DVPK Asset Package
#     ICON_PATH            Davenstein Application Icon
#
# Portable Release Output:
#     Davenstein-<version>-freebsd-<architecture>.tar.gz
#     Davenstein-<version>-freebsd-<architecture>.tar.gz.sha256
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/portable"

# Create Isolated Temporary Staging Root for Portable Package Assembly
STAGE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/davenstein-freebsd-portable.XXXXXX")

# Release Version
# Release Automation May Override VERSION With the Complete Git Tag Version
RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}

# Public Release Architecture and Existing Package Inputs
ARCH=${ARCH:-x86_64}
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/x86_64-unknown-freebsd/release/Davenstein"}
ASSETS_PATH=${ASSETS_PATH:-"$ROOT_DIR/target/release/assets.pak"}
ICON_PATH=${ICON_PATH:-"$ROOT_DIR/packaging/linux/davenstein.png"}

# Versioned Architecture-Specific Portable Archive Paths
ARCHIVE_BASENAME="Davenstein-${RELEASE_VERSION}-freebsd-${ARCH}"
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
    "$ASSETS_PATH" \
    "$ROOT_DIR/README.md" \
    "$ICON_PATH"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required FreeBSD portable-build input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

# Validate Archive and Checksum Utilities
for required_command in tar sha256sum
do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        printf '%s is required to build the FreeBSD portable archive\n' \
            "$required_command" >&2
        exit 1
    fi
done

# Remove Existing Archive and Checksum for Selected Version and Architecture
rm -f "$ARCHIVE_PATH" "$ARCHIVE_PATH.sha256"

# Create Persistent Output Directory and Temporary Package Root
install -d "$BUILD_DIR"
install -d -m 755 "$STAGE_DIR"

# Install Davenstein FreeBSD Executable
install -m 755 \
    "$BINARY_PATH" \
    "$STAGE_DIR/Davenstein"

# Keep assets.pak Beside Executable for Runtime Package Resolution
install -m 644 \
    "$ASSETS_PATH" \
    "$STAGE_DIR/assets.pak"

# Include Project README With Portable Release
install -m 644 \
    "$ROOT_DIR/README.md" \
    "$STAGE_DIR/README.md"

# Include Davenstein Application Icon With Portable Release
install -m 644 \
    "$ICON_PATH" \
    "$STAGE_DIR/davenstein.png"

# Enable Portable Storage Mode Beside Davenstein Executable
: > "$STAGE_DIR/portable.flag"

# Create Launcher That Runs Davenstein From Extracted Package Directory
# This Preserves Relative Runtime Paths Regardless of the Caller Working Directory
cat > "$STAGE_DIR/run-davenstein.sh" <<'LAUNCHER'
#!/bin/sh
set -eu

# Resolve and Enter Extracted Portable Package Directory
# assets.pak and portable.flag Remain Discoverable Beside the Executable
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
