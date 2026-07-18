#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds a Native FreeBSD Package From an Existing Cross-Compiled Davenstein
# Executable and a Previously Generated DVPK Asset Archive
#
# This Script Must Run Under FreeBSD Because pkg create, pkg rquery, Package
# Dependency Resolution, and Package ABI Detection Require the Native FreeBSD
# Package Management Environment
#
# Package Construction Process:
#     - Resolve Repository and Packaging Paths Relative to this Script
#     - Validate Required Package Inputs and Native FreeBSD Tools
#     - Detect and Validate the FreeBSD Package ABI
#     - Construct a Clean FreeBSD Package Staging Root
#     - Install Davenstein, assets.pak, Desktop Metadata, Icon, and README
#     - Generate a Prefix-Aware Launcher Under /usr/local/bin
#     - Resolve Exact Installed FreeBSD Dependency Versions
#     - Generate the Native FreeBSD Package Manifest
#     - Build the Package With pkg create
#     - Rename the Package to the Public Release Naming Convention
#     - Generate a Matching SHA-256 Checksum
#
# Installed Package Layout:
#     /usr/local/bin/Davenstein
#     /usr/local/libexec/davenstein/Davenstein
#     /usr/local/libexec/davenstein/assets.pak
#     /usr/local/share/applications/davenstein.desktop
#     /usr/local/share/icons/hicolor/256x256/apps/davenstein.png
#     /usr/local/share/doc/davenstein/README.md
#
# Release Automation May Override:
#     VERSION              Complete Release Version or Git Tag
#     ARCH                 Public Release Architecture Name
#     BINARY_PATH          Existing FreeBSD Davenstein Executable
#     ASSETS_PATH          Existing DVPK Asset Package
#     ICON_PATH            Davenstein Application Icon
#     DESKTOP_PATH         FreeBSD Desktop Entry
#     PLIST_PATH           FreeBSD Package File List
#     PKG_ABI              FreeBSD Package ABI
#
# Native Package Output:
#     Davenstein-<version>-freebsd-<architecture>.pkg
#     Davenstein-<version>-freebsd-<architecture>.pkg.sha256
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

# FreeBSD Package Build, Staging, Metadata, and Dependency Paths
BUILD_DIR="$ROOT_DIR/target/freebsd"
STAGE_ROOT="$BUILD_DIR/package-root"
METADATA_DIR="$BUILD_DIR/metadata"
DEPENDENCY_ROWS="$BUILD_DIR/dependencies.rows"
DEPENDENCY_UCL="$BUILD_DIR/dependencies.ucl"

# Release Version
# Release Automation May Override VERSION With the Complete Git Tag Version
RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}

# Public Release Architecture and Existing Package Inputs
ARCH=${ARCH:-x86_64}
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/x86_64-unknown-freebsd/release/Davenstein"}
ASSETS_PATH=${ASSETS_PATH:-"$ROOT_DIR/target/release/assets.pak"}
ICON_PATH=${ICON_PATH:-"$ROOT_DIR/packaging/linux/davenstein.png"}
DESKTOP_PATH=${DESKTOP_PATH:-"$SCRIPT_DIR/davenstein.desktop"}
PLIST_PATH=${PLIST_PATH:-"$SCRIPT_DIR/pkg-plist"}

# Versioned Architecture-Specific FreeBSD Package Output Paths
PACKAGE_BASENAME="Davenstein-${RELEASE_VERSION}-freebsd-${ARCH}"
PACKAGE_PATH="$BUILD_DIR/$PACKAGE_BASENAME.pkg"
CHECKSUM_PATH="$PACKAGE_PATH.sha256"

# Refuse Unversioned FreeBSD Package Output
if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein version\n' >&2
    exit 1
fi

# Validate Every Required Package Input Before Modifying Build Output
for required_file in \
    "$BINARY_PATH" \
    "$ASSETS_PATH" \
    "$ICON_PATH" \
    "$DESKTOP_PATH" \
    "$PLIST_PATH" \
    "$ROOT_DIR/README.md"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required FreeBSD package input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

# Validate Native FreeBSD Package Tools and Supporting Utilities
for required_command in \
    awk \
    install \
    pkg \
    sha256
do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        printf '%s is required to build the native FreeBSD package\n' \
            "$required_command" >&2
        exit 1
    fi
done

# FreeBSD Package ABI Uses Native Architecture Terminology
# Public Release Artifact Uses x86_64 While pkg Uses amd64
PACKAGE_ABI=${PKG_ABI:-$(pkg config ABI)}

# Supported Native FreeBSD Package ABI
case "$PACKAGE_ABI" in
    FreeBSD:*:amd64)
        ;;
    *)
        printf 'Unsupported FreeBSD package ABI: %s\n' "$PACKAGE_ABI" >&2
        exit 1
        ;;
esac

# Remove Previous Staging Roots and Generated Metadata
rm -rf "$STAGE_ROOT" "$METADATA_DIR"

# Remove Previous Dependency Files and Selected Package Outputs
rm -f \
    "$DEPENDENCY_ROWS" \
    "$DEPENDENCY_UCL" \
    "$PACKAGE_PATH" \
    "$CHECKSUM_PATH" \
    "$BUILD_DIR"/davenstein-"$RELEASE_VERSION"*.pkg

# Create FreeBSD Package Build and Metadata Directories
install -d "$BUILD_DIR"
install -d "$METADATA_DIR"

# Construct Native FreeBSD Package Filesystem Layout
install -d -m 755 \
    "$STAGE_ROOT/usr/local/bin" \
    "$STAGE_ROOT/usr/local/libexec/davenstein" \
    "$STAGE_ROOT/usr/local/share/applications" \
    "$STAGE_ROOT/usr/local/share/doc/davenstein" \
    "$STAGE_ROOT/usr/local/share/icons/hicolor/256x256/apps"

# Install Davenstein Release Executable Into Private Application Directory
install -m 755 \
    "$BINARY_PATH" \
    "$STAGE_ROOT/usr/local/libexec/davenstein/Davenstein"

# Keep assets.pak Beside Executable for Runtime Package Resolution
install -m 644 \
    "$ASSETS_PATH" \
    "$STAGE_ROOT/usr/local/libexec/davenstein/assets.pak"

# Install FreeBSD Desktop Entry
install -m 644 \
    "$DESKTOP_PATH" \
    "$STAGE_ROOT/usr/local/share/applications/davenstein.desktop"

# Install Davenstein Application Icon Into Standard hicolor Theme Path
install -m 644 \
    "$ICON_PATH" \
    "$STAGE_ROOT/usr/local/share/icons/hicolor/256x256/apps/davenstein.png"

# Install Project README as Package Documentation
install -m 644 \
    "$ROOT_DIR/README.md" \
    "$STAGE_ROOT/usr/local/share/doc/davenstein/README.md"

# Create Public Launcher Under Standard FreeBSD Binary Prefix
cat > "$STAGE_ROOT/usr/local/bin/Davenstein" <<'LAUNCHER'
#!/bin/sh
set -eu

# Resolve Installation Prefix Relative to this Launcher
# Caller Working Directory Does Not Affect Davenstein Startup
PREFIX=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
GAME_DIR="$PREFIX/libexec/davenstein"

# Enter Private Application Directory Because assets.pak Resides Beside Executable
cd "$GAME_DIR"

# Replace Launcher Process With Davenstein and Forward All Arguments
exec "$GAME_DIR/Davenstein" "$@"
LAUNCHER

# Mark Public Launcher as Executable
chmod 755 "$STAGE_ROOT/usr/local/bin/Davenstein"

# Initialize Temporary Dependency Row File
: > "$DEPENDENCY_ROWS"

# Resolve Exact Installed Versions for Every Runtime Package Dependency
while IFS='|' read -r package_name package_origin
do
    # Match Both Package Name and Ports Origin to Avoid Ambiguous Results
    dependency=$(
        pkg rquery '%n|%o|%v' "$package_name" |
            awk -F '|' \
                -v expected_name="$package_name" \
                -v expected_origin="$package_origin" \
                '$1 == expected_name && $2 == expected_origin {
                    print
                    exit
                }'
    )

    # Refuse Package Creation When a Required Dependency Cannot be Resolved
    if [ -z "$dependency" ]; then
        printf 'Could not resolve FreeBSD dependency %s from %s\n' \
            "$package_name" \
            "$package_origin" >&2
        exit 1
    fi

    # Extract Exact Installed Package Version From pkg rquery Result
    package_version=${dependency##*|}

    # Write One UCL Dependency Entry Without a Trailing Comma
    printf '  "%s": { origin: "%s", version: "%s" }\n' \
        "$package_name" \
        "$package_origin" \
        "$package_version" \
        >> "$DEPENDENCY_ROWS"
done <<'DEPENDENCIES'
alsa-lib|audio/alsa-lib
libX11|x11/libX11
libXcursor|x11/libXcursor
libXi|x11/libXi
libXrandr|x11/libXrandr
libudev-devd|devel/libudev-devd
libxkbcommon|x11/libxkbcommon
wayland|graphics/wayland
DEPENDENCIES

# Add Commas Between UCL Dependency Entries
# Final Dependency Entry Remains Without a Trailing Comma
awk '
    NR > 1 {
        print previous ","
    }

    {
        previous = $0
    }

    END {
        if (NR > 0) {
            print previous
        }
    }
' "$DEPENDENCY_ROWS" > "$DEPENDENCY_UCL"

# Generate Native FreeBSD Package Manifest
cat > "$METADATA_DIR/+MANIFEST" <<EOF
name: "davenstein"
version: "$RELEASE_VERSION"
origin: "games/davenstein"
comment: "A Wolfenstein 3-D recreation written in Rust with the Bevy engine"
desc: "Davenstein is a ground-up recreation of Wolfenstein 3-D implemented as a native Rust application using the Bevy game engine."
maintainer: "dpetnick89@gmail.com"
www: "https://github.com/Ophois47/Davenstein"
prefix: "/usr/local"
arch: "$PACKAGE_ABI"
deps: {
$(cat "$DEPENDENCY_UCL")
}
EOF

# Build Native FreeBSD Package From Staging Root, Manifest, and Package List
pkg create \
    --verbose \
    --root-dir "$STAGE_ROOT" \
    --metadata "$METADATA_DIR" \
    --plist "$PLIST_PATH" \
    --out-dir "$BUILD_DIR"

# Locate Package Name Generated by pkg create
CREATED_PACKAGE=

for candidate in "$BUILD_DIR"/davenstein-"$RELEASE_VERSION"*.pkg
do
    if [ -f "$candidate" ]; then
        CREATED_PACKAGE=$candidate
        break
    fi
done

# Refuse Release Output When pkg create Did Not Produce Expected Package
if [ -z "$CREATED_PACKAGE" ]; then
    printf 'pkg create did not produce the expected package\n' >&2
    exit 1
fi

# Rename Native Package to Public Release Artifact Naming Convention
mv "$CREATED_PACKAGE" "$PACKAGE_PATH"

# Generate Matching FreeBSD SHA-256 Checksum
package_hash=$(sha256 -q "$PACKAGE_PATH")

printf '%s  %s\n' \
    "$package_hash" \
    "$(basename "$PACKAGE_PATH")" \
    > "$CHECKSUM_PATH"

printf 'Created %s\n' "$PACKAGE_PATH"
printf 'Created %s\n' "$CHECKSUM_PATH"
