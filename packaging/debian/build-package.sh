#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds a Native Debian amd64 Package From an Existing Davenstein Release
# Binary and DVPK Asset Package
#
# Installed Layout:
#     /usr/games/Davenstein
#     /usr/lib/davenstein/Davenstein
#     /usr/lib/davenstein/assets.pak
#     /usr/share/applications/davenstein.desktop
#     /usr/share/icons/hicolor/256x256/apps/davenstein.png
#     /usr/share/doc/davenstein/
#
# The Public Launcher Executes the Real Binary From /usr/lib/davenstein so
# assets.pak Remains Beside the Executable for Existing Runtime Asset Discovery
#
# No portable.flag is Installed, Preserving Davenstein's Installed Storage Mode
#
# Release Automation May Override:
#     VERSION              Complete Release Version or Git Tag
#     DEBIAN_VERSION       Complete Internal Debian Version
#     DEB_ARCH             Debian Architecture
#     BINARY_PATH          Existing Davenstein Release Binary
#     ASSETS_PATH          Existing DVPK Asset Package
#     MAINTAINER           Debian Maintainer Identity
#
# Debian Output:
#     Davenstein-<version>-linux-amd64.deb
#     Davenstein-<version>-linux-amd64.deb.sha256
#

# Resolve Repository Paths Relative to this Script
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/debian"

# Create Isolated Temporary Roots for Package Assembly and Dependency Analysis
STAGE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/davenstein-debian.XXXXXX")
PACKAGE_ROOT="$STAGE_ROOT/package"
ANALYSIS_ROOT="$STAGE_ROOT/analysis"

# Remove Temporary Packaging Files on Normal Exit or Interruption
trap 'rm -rf "$STAGE_ROOT"' EXIT HUP INT TERM

# Resolve Release Version From Environment or Cargo Package Metadata
RELEASE_VERSION=${VERSION:-$(sed -nE \
    's/^version = "([^"]+)"/\1/p' \
    "$ROOT_DIR/Cargo.toml" |
    head -n 1)}

# Git Tags May Prefix the Release Version With v
RELEASE_VERSION=${RELEASE_VERSION#v}

if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein release version\n' >&2
    exit 1
fi

# Convert Semantic Prerelease Separators to Debian Version Ordering
# Example: 1.0.0-alpha.9 Becomes 1.0.0~alpha.9-1
case "$RELEASE_VERSION" in
    *-*)
        upstream_version=${RELEASE_VERSION%%-*}
        prerelease_version=${RELEASE_VERSION#*-}
        DEFAULT_DEBIAN_VERSION="${upstream_version}~${prerelease_version}-1"
        ;;
    *)
        DEFAULT_DEBIAN_VERSION="${RELEASE_VERSION}-1"
        ;;
esac

DEBIAN_VERSION=${DEBIAN_VERSION:-"$DEFAULT_DEBIAN_VERSION"}
DEB_ARCH=${DEB_ARCH:-amd64}
MAINTAINER=${MAINTAINER:-"David Petnick <Ophois47@users.noreply.github.com>"}

# Existing Build Inputs
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/release/Davenstein"}
ASSETS_PATH=${ASSETS_PATH:-"$ROOT_DIR/target/release/assets.pak"}

# Versioned Debian Package Output
OUTPUT_BASENAME="Davenstein-${RELEASE_VERSION}-linux-${DEB_ARCH}.deb"
OUTPUT_PATH="$BUILD_DIR/$OUTPUT_BASENAME"

# This Builder Currently Produces Only the Validated x86_64 Debian Package
if [ "$DEB_ARCH" != "amd64" ]; then
    printf 'Unsupported Debian architecture: %s\n' "$DEB_ARCH" >&2
    exit 1
fi

# Validate Required Debian Packaging Tools Before Modifying Output
for required_command in \
    dpkg \
    dpkg-deb \
    dpkg-shlibdeps \
    dpkg-architecture \
    date \
    file \
    gzip \
    install \
    ln \
    sed \
    strip \
    sha256sum \
    md5sum
do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        printf 'Required Debian packaging tool was not found: %s\n' \
            "$required_command" >&2
        exit 1
    fi
done

# Validate Debian Version Syntax
if ! dpkg --validate-version "$DEBIAN_VERSION"; then
    printf 'Invalid Debian package version: %s\n' "$DEBIAN_VERSION" >&2
    exit 1
fi

# Refuse to Package a Binary for an Unexpected Architecture
if ! file "$BINARY_PATH" | grep -Eiq 'ELF 64-bit.*x86-64'; then
    printf 'Davenstein binary is not an x86_64 ELF executable: %s\n' \
        "$BINARY_PATH" >&2
    exit 1
fi

# Validate Every Required Package Input Before Modifying Output
for required_file in \
    "$BINARY_PATH" \
    "$ASSETS_PATH" \
    "$ROOT_DIR/README.md" \
    "$ROOT_DIR/LICENSE.md" \
    "$ROOT_DIR/LICENSE-MIT" \
    "$ROOT_DIR/LICENSE-APACHE" \
    "$ROOT_DIR/COPYRIGHT.md" \
    "$ROOT_DIR/THIRD_PARTY_ASSETS.md" \
    "$ROOT_DIR/packaging/linux/davenstein.desktop" \
    "$ROOT_DIR/packaging/linux/davenstein.png"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required Debian package input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

# Reconstruct Package Staging From Scratch
install -d -m 755 \
    "$PACKAGE_ROOT/DEBIAN" \
    "$PACKAGE_ROOT/usr/games" \
    "$PACKAGE_ROOT/usr/lib/davenstein" \
    "$PACKAGE_ROOT/usr/share/applications" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/256x256/apps" \
    "$PACKAGE_ROOT/usr/share/doc/davenstein" \
    "$PACKAGE_ROOT/usr/share/man/man6" \
    "$ANALYSIS_ROOT/debian"

# Install the Real Davenstein Executable Into the Package-Private Directory
install -m 755 \
    "$BINARY_PATH" \
    "$PACKAGE_ROOT/usr/lib/davenstein/Davenstein"

# Strip Only the Staged Copy; Never Modify the Existing Release Binary
strip --strip-unneeded \
    "$PACKAGE_ROOT/usr/lib/davenstein/Davenstein"

# Keep assets.pak Beside the Real Executable for Runtime Package Resolution
install -m 644 \
    "$ASSETS_PATH" \
    "$PACKAGE_ROOT/usr/lib/davenstein/assets.pak"

# Create the Public Launcher Without Enabling Portable Storage Mode
cat > "$PACKAGE_ROOT/usr/games/Davenstein" <<'LAUNCHER'
#!/bin/sh
set -eu

exec /usr/lib/davenstein/Davenstein "$@"
LAUNCHER

chmod 755 "$PACKAGE_ROOT/usr/games/Davenstein"

# Install Desktop Integration
install -m 644 \
    "$ROOT_DIR/packaging/linux/davenstein.desktop" \
    "$PACKAGE_ROOT/usr/share/applications/davenstein.desktop"

# Use Debian's Conventional Games Directory in the Installed Desktop Entry
sed -i \
    's|^Exec=.*$|Exec=/usr/games/Davenstein|' \
    "$PACKAGE_ROOT/usr/share/applications/davenstein.desktop"

install -m 644 \
    "$ROOT_DIR/packaging/linux/davenstein.png" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/256x256/apps/davenstein.png"

# Install Project Documentation and Legal Notices
install -m 644 \
    "$ROOT_DIR/README.md" \
    "$PACKAGE_ROOT/usr/share/doc/davenstein/README.md"

for legal_file in \
    LICENSE.md \
    LICENSE-MIT \
    COPYRIGHT.md \
    THIRD_PARTY_ASSETS.md
do
    install -m 644 \
        "$ROOT_DIR/$legal_file" \
        "$PACKAGE_ROOT/usr/share/doc/davenstein/$legal_file"
done

# Use Debian's Canonical Copy of the Apache License
ln -s \
    ../../common-licenses/Apache-2.0 \
    "$PACKAGE_ROOT/usr/share/doc/davenstein/LICENSE-APACHE"

# Provide Debian's Conventional Copyright Notice
cat > "$PACKAGE_ROOT/usr/share/doc/davenstein/copyright" <<COPYRIGHT
Davenstein
Copyright (c) 2025-2026 David Petnick

The original Davenstein software is available under either the MIT License or
the Apache License, Version 2.0, at the recipient's option.

The complete MIT license text is installed as LICENSE-MIT. On Debian
systems, LICENSE-APACHE links to the canonical Apache License, Version 2.0,
at /usr/share/common-licenses/Apache-2.0. Additional licensing information is
installed as LICENSE.md and COPYRIGHT.md.

Wolfenstein-derived and other third-party materials are not covered by
Davenstein's MIT OR Apache-2.0 software license. No ownership of those
materials is claimed, and no sublicense to those materials is granted.

See THIRD_PARTY_ASSETS.md for provenance and rights information.

Project source:
https://github.com/Ophois47/Davenstein
COPYRIGHT

chmod 644 "$PACKAGE_ROOT/usr/share/doc/davenstein/copyright"

# Add the Debian Package Changelog Required for a Non-Native Package
CHANGELOG_DATE=${DEBIAN_CHANGELOG_DATE:-$(date -Ru)}

cat > "$PACKAGE_ROOT/usr/share/doc/davenstein/changelog.Debian" <<CHANGELOG
davenstein ($DEBIAN_VERSION) unstable; urgency=medium

  * Added native Debian amd64 release packaging.

 -- $MAINTAINER  $CHANGELOG_DATE
CHANGELOG

gzip -n -9 \
    "$PACKAGE_ROOT/usr/share/doc/davenstein/changelog.Debian"

# Add a Section-Six Manual Page for the Installed Game Launcher
cat > "$PACKAGE_ROOT/usr/share/man/man6/Davenstein.6" <<MANPAGE
.TH DAVENSTEIN 6 "July 2026" "Davenstein $RELEASE_VERSION" "Games"
.SH NAME
Davenstein \- a Wolfenstein 3-D recreation written in Rust
.SH SYNOPSIS
.B Davenstein
.RI [ arguments ]
.SH DESCRIPTION
Davenstein is a ground-up native recreation of Wolfenstein 3-D written in
Rust using the Bevy game engine.
.PP
The Debian package runs the game in installed storage mode. Saves, high
scores, and other writable data are stored in the current user's application
data directory rather than beneath /usr.
.SH FILES
.TP
.I /usr/lib/davenstein/Davenstein
The native game executable.
.TP
.I /usr/lib/davenstein/assets.pak
The packaged Davenstein game assets.
.TP
.I /usr/share/doc/davenstein
Project documentation, licenses, copyright, and third-party asset notices.
.SH AUTHOR
Davenstein was created and is maintained by David Petnick.
.SH HOMEPAGE
https://github.com/Ophois47/Davenstein
MANPAGE

gzip -n -9 \
    "$PACKAGE_ROOT/usr/share/man/man6/Davenstein.6"

# Create Minimal Source Metadata Required by dpkg-shlibdeps
cat > "$ANALYSIS_ROOT/debian/control" <<CONTROL
Source: davenstein
Section: games
Priority: optional
Maintainer: $MAINTAINER
Standards-Version: 4.7.2
Homepage: https://github.com/Ophois47/Davenstein

Package: davenstein
Architecture: $DEB_ARCH
Depends: \${shlibs:Depends}
Description: Wolfenstein 3-D recreation written in Rust
 Davenstein is a ground-up native recreation of Wolfenstein 3-D written in
 Rust using the Bevy game engine.
CONTROL

# Resolve Debian Package Dependencies From the Staged ELF Executable
SHLIBS_OUTPUT=$(
    cd "$ANALYSIS_ROOT"
    dpkg-shlibdeps \
        -O \
        -e"$PACKAGE_ROOT/usr/lib/davenstein/Davenstein"
)

SHLIBS_DEPENDS=$(
    printf '%s\n' "$SHLIBS_OUTPUT" |
        sed -n 's/^shlibs:Depends=//p'
)

if [ -z "$SHLIBS_DEPENDS" ]; then
    printf 'dpkg-shlibdeps did not produce package dependencies\n' >&2
    exit 1
fi

# Record Approximate Installed Size in Kibibytes
INSTALLED_SIZE=$(
    du -sk "$PACKAGE_ROOT/usr" |
        awk '{ print $1 }'
)

# Generate Final Binary Package Metadata
cat > "$PACKAGE_ROOT/DEBIAN/control" <<CONTROL
Package: davenstein
Version: $DEBIAN_VERSION
Section: games
Priority: optional
Architecture: $DEB_ARCH
Maintainer: $MAINTAINER
Installed-Size: $INSTALLED_SIZE
Depends: $SHLIBS_DEPENDS
Homepage: https://github.com/Ophois47/Davenstein
Description: Wolfenstein 3-D recreation written in Rust
 Davenstein is a ground-up native recreation of Wolfenstein 3-D written in
 Rust using the Bevy game engine.
 .
 This package installs Davenstein as a system application while retaining
 per-user save data and high-score storage.
CONTROL

chmod 644 "$PACKAGE_ROOT/DEBIAN/control"

# Record Checksums for Every Installed Package File
(
    cd "$PACKAGE_ROOT"
    find usr -type f -print |
        LC_ALL=C sort |
        xargs md5sum
) > "$PACKAGE_ROOT/DEBIAN/md5sums"

chmod 644 "$PACKAGE_ROOT/DEBIAN/md5sums"

# Remove Only the Selected Debian Output and Its Matching Checksum
install -d -m 755 "$BUILD_DIR"
rm -f "$OUTPUT_PATH" "$OUTPUT_PATH.sha256"

# Build With Numeric Root Ownership Without Requiring Root Privileges
dpkg-deb \
    --root-owner-group \
    --build \
    "$PACKAGE_ROOT" \
    "$OUTPUT_PATH"

# Generate Matching SHA-256 Checksum
cd "$BUILD_DIR"
sha256sum "$OUTPUT_BASENAME" \
    > "$OUTPUT_BASENAME.sha256"

printf 'Created %s\n' "$OUTPUT_PATH"
printf 'Created %s\n' "$OUTPUT_PATH.sha256"
printf 'Debian version: %s\n' "$DEBIAN_VERSION"
printf 'Dependencies: %s\n' "$SHLIBS_DEPENDS"
