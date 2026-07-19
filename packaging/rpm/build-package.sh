#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Builds a Native RPM x86_64 Package From an Existing Davenstein Release
# Binary and DVPK Asset Package
#
# Installed Layout:
#     /usr/bin/Davenstein
#     /usr/libexec/davenstein/Davenstein
#     /usr/libexec/davenstein/assets.pak
#     /usr/share/applications/davenstein.desktop
#     /usr/share/icons/hicolor/256x256/apps/davenstein.png
#     /usr/share/doc/davenstein/
#     /usr/share/licenses/davenstein/
#     /usr/share/man/man6/Davenstein.6.gz
#
# The Public Launcher Executes the Real Binary From RPM's libexec Directory so
# assets.pak Remains Beside the Executable for Existing Runtime Asset Discovery
#
# No portable.flag is Installed, Preserving Davenstein's Installed Storage Mode
#
# Release Automation May Override:
#     VERSION              Complete Release Version or Git Tag
#     RPM_VERSION          Internal RPM Version Field
#     RPM_RELEASE          Internal RPM Release Field
#     RPM_ARCH             RPM Architecture
#     BINARY_PATH          Existing Davenstein Release Binary
#     ASSETS_PATH          Existing DVPK Asset Package
#     PACKAGER             RPM Packager Identity
#
# RPM Output:
#     Davenstein-<version>-linux-x86_64.rpm
#     Davenstein-<version>-linux-x86_64.rpm.sha256
#

# Resolve Repository Paths Relative to this Script
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/rpm"

# Create Isolated Temporary Roots for Payload Assembly and rpmbuild
STAGE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/davenstein-rpm.XXXXXX")
PAYLOAD_ROOT="$STAGE_ROOT/payload"
RPM_TOPDIR="$STAGE_ROOT/rpmbuild"
SPEC_PATH="$RPM_TOPDIR/SPECS/davenstein.spec"

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

# Convert Semantic Prerelease Versions Into RPM Version and Release Ordering
#
# RPM compares Version and Release separately. A prerelease such as
# 1.0.0-alpha.9 therefore becomes:
#
#     Version: 1.0.0
#     Release: 0.alpha.9.1
#
# The leading zero keeps the prerelease ordered before the eventual stable
# package, whose Release field begins at 1.
case "$RELEASE_VERSION" in
    *-*)
        DEFAULT_RPM_VERSION=${RELEASE_VERSION%%-*}
        prerelease_version=${RELEASE_VERSION#*-}
        prerelease_version=$(
            printf '%s\n' "$prerelease_version" |
                sed 's/[^A-Za-z0-9._+]/./g'
        )
        DEFAULT_RPM_RELEASE="0.${prerelease_version}.1"
        ;;
    *)
        DEFAULT_RPM_VERSION=$RELEASE_VERSION
        DEFAULT_RPM_RELEASE=1
        ;;
esac

RPM_VERSION=${RPM_VERSION:-"$DEFAULT_RPM_VERSION"}
RPM_RELEASE=${RPM_RELEASE:-"$DEFAULT_RPM_RELEASE"}
RPM_ARCH=${RPM_ARCH:-x86_64}
PACKAGER=${PACKAGER:-"David Petnick"}

# Existing Build Inputs
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/release/Davenstein"}
ASSETS_PATH=${ASSETS_PATH:-"$ROOT_DIR/target/release/assets.pak"}

# Preserve the User-Facing Release Version in the Download Filename While RPM
# Uses Its Own Version and Release Fields Internally
OUTPUT_BASENAME="Davenstein-${RELEASE_VERSION}-linux-${RPM_ARCH}.rpm"
OUTPUT_PATH="$BUILD_DIR/$OUTPUT_BASENAME"

# This Builder Currently Produces Only the Validated x86_64 RPM Package
if [ "$RPM_ARCH" != "x86_64" ]; then
    printf 'Unsupported RPM architecture: %s\n' "$RPM_ARCH" >&2
    exit 1
fi

# Validate Required RPM Packaging Tools Before Modifying Output
for required_command in \
    awk \
    cp \
    date \
    desktop-file-validate \
    file \
    find \
    gzip \
    install \
    rpmbuild \
    rpm \
    sed \
    sha256sum \
    strip
do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        printf 'Required RPM packaging tool was not found: %s\n' \
            "$required_command" >&2
        exit 1
    fi
done

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
        printf 'Required RPM package input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

# Reconstruct the Package Payload and rpmbuild Tree From Scratch
install -d -m 755 \
    "$PAYLOAD_ROOT/usr/bin" \
    "$PAYLOAD_ROOT/usr/libexec/davenstein" \
    "$PAYLOAD_ROOT/usr/share/applications" \
    "$PAYLOAD_ROOT/usr/share/icons/hicolor/256x256/apps" \
    "$PAYLOAD_ROOT/usr/share/doc/davenstein" \
    "$PAYLOAD_ROOT/usr/share/licenses/davenstein" \
    "$PAYLOAD_ROOT/usr/share/man/man6" \
    "$RPM_TOPDIR/BUILD" \
    "$RPM_TOPDIR/BUILDROOT" \
    "$RPM_TOPDIR/RPMS" \
    "$RPM_TOPDIR/SOURCES" \
    "$RPM_TOPDIR/SPECS" \
    "$RPM_TOPDIR/SRPMS"

# Install the Real Davenstein Executable Into a Package-Private Directory
install -m 755 \
    "$BINARY_PATH" \
    "$PAYLOAD_ROOT/usr/libexec/davenstein/Davenstein"

# Strip Only the Staged Copy; Never Modify the Existing Release Binary
strip --strip-unneeded \
    "$PAYLOAD_ROOT/usr/libexec/davenstein/Davenstein"

# Keep assets.pak Beside the Real Executable for Runtime Package Resolution
install -m 644 \
    "$ASSETS_PATH" \
    "$PAYLOAD_ROOT/usr/libexec/davenstein/assets.pak"

# Create the Public Launcher Without Enabling Portable Storage Mode
cat > "$PAYLOAD_ROOT/usr/bin/Davenstein" <<'LAUNCHER'
#!/bin/sh
set -eu

exec /usr/libexec/davenstein/Davenstein "$@"
LAUNCHER

chmod 755 "$PAYLOAD_ROOT/usr/bin/Davenstein"

# Install Desktop Integration and Point It at the Public RPM Launcher
install -m 644 \
    "$ROOT_DIR/packaging/linux/davenstein.desktop" \
    "$PAYLOAD_ROOT/usr/share/applications/davenstein.desktop"

sed -i \
    's|^Exec=.*$|Exec=/usr/bin/Davenstein|' \
    "$PAYLOAD_ROOT/usr/share/applications/davenstein.desktop"

desktop-file-validate \
    "$PAYLOAD_ROOT/usr/share/applications/davenstein.desktop"

install -m 644 \
    "$ROOT_DIR/packaging/linux/davenstein.png" \
    "$PAYLOAD_ROOT/usr/share/icons/hicolor/256x256/apps/davenstein.png"

# Install Project Documentation Separately From Files Marked as RPM Licenses
install -m 644 \
    "$ROOT_DIR/README.md" \
    "$PAYLOAD_ROOT/usr/share/doc/davenstein/README.md"

for license_file in \
    LICENSE.md \
    LICENSE-MIT \
    LICENSE-APACHE \
    COPYRIGHT.md \
    THIRD_PARTY_ASSETS.md
do
    install -m 644 \
        "$ROOT_DIR/$license_file" \
        "$PAYLOAD_ROOT/usr/share/licenses/davenstein/$license_file"
done

# Add a Section-Six Manual Page for the Installed Game Launcher
MANPAGE_DATE=${RPM_MANPAGE_DATE:-$(date -u '+%B %Y')}

cat > "$PAYLOAD_ROOT/usr/share/man/man6/Davenstein.6" <<MANPAGE
.TH DAVENSTEIN 6 "$MANPAGE_DATE" "Davenstein $RELEASE_VERSION" "Games"
.SH NAME
Davenstein \- a Wolfenstein 3-D recreation written in Rust
.SH SYNOPSIS
.B Davenstein
.RI [ arguments ]
.SH DESCRIPTION
Davenstein is a ground-up native recreation of Wolfenstein 3-D written in
Rust using the Bevy game engine.
.PP
The RPM package runs the game in installed storage mode. Saves, high scores,
and other writable data are stored in the current user's application data
directory rather than beneath /usr.
.SH FILES
.TP
.I /usr/libexec/davenstein/Davenstein
The native game executable.
.TP
.I /usr/libexec/davenstein/assets.pak
The packaged Davenstein game assets.
.TP
.I /usr/share/doc/davenstein
Project documentation.
.TP
.I /usr/share/licenses/davenstein
Software licenses, copyright information, and third-party asset notices.
.SH AUTHOR
Davenstein was created and is maintained by David Petnick.
.SH HOMEPAGE
https://github.com/Ophois47/Davenstein
MANPAGE

gzip -n -9 \
    "$PAYLOAD_ROOT/usr/share/man/man6/Davenstein.6"

# Generate the RPM Specification From the Validated Staged Payload
#
# Standard RPM path macros describe the installed files even though the
# payload is assembled in a temporary directory before rpmbuild executes.
CHANGELOG_DATE=${RPM_CHANGELOG_DATE:-$(date -u '+%a %b %d %Y')}

cat > "$SPEC_PATH" <<SPEC
%global debug_package %{nil}
%global payload_root $PAYLOAD_ROOT

Name:           davenstein
Version:        $RPM_VERSION
Release:        $RPM_RELEASE%{?dist}
Summary:        Wolfenstein 3-D recreation written in Rust

License:        MIT OR Apache-2.0
URL:            https://github.com/Ophois47/Davenstein
BuildArch:      $RPM_ARCH
BuildRequires:  desktop-file-utils
Requires:       hicolor-icon-theme
Packager:       $PACKAGER

%description
Davenstein is a ground-up native recreation of Wolfenstein 3-D written in
Rust using the Bevy game engine.

This package installs Davenstein as a system application while retaining
per-user save data and high-score storage.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a %{payload_root}/. %{buildroot}/

%check
test -x %{buildroot}%{_bindir}/Davenstein
test -x %{buildroot}%{_libexecdir}/davenstein/Davenstein
test -s %{buildroot}%{_libexecdir}/davenstein/assets.pak
test ! -e %{buildroot}%{_libexecdir}/davenstein/portable.flag
desktop-file-validate \
    %{buildroot}%{_datadir}/applications/davenstein.desktop

%files
%{_bindir}/Davenstein
%{_libexecdir}/davenstein/
%{_datadir}/applications/davenstein.desktop
%{_datadir}/icons/hicolor/256x256/apps/davenstein.png
%dir %{_docdir}/%{name}
%doc %{_docdir}/%{name}/README.md
%dir %{_licensedir}/%{name}
%license %{_licensedir}/%{name}/LICENSE.md
%license %{_licensedir}/%{name}/LICENSE-MIT
%license %{_licensedir}/%{name}/LICENSE-APACHE
%license %{_licensedir}/%{name}/COPYRIGHT.md
%license %{_licensedir}/%{name}/THIRD_PARTY_ASSETS.md
%{_mandir}/man6/Davenstein.6*

%changelog
* $CHANGELOG_DATE $PACKAGER - $RPM_VERSION-$RPM_RELEASE
- Added native RPM x86_64 release packaging.
SPEC

# Remove Only the Selected RPM Output and Its Matching Checksum
install -d -m 755 "$BUILD_DIR"
rm -f "$OUTPUT_PATH" "$OUTPUT_PATH.sha256"

# Build the Binary RPM Without Producing Separate Debug Packages
rpmbuild \
    --define "_topdir $RPM_TOPDIR" \
    --define "_build_id_links none" \
    -bb \
    "$SPEC_PATH"

# Locate the Single Main RPM Produced by the Specification
BUILT_RPM=$(
    find "$RPM_TOPDIR/RPMS/$RPM_ARCH" \
        -maxdepth 1 \
        -type f \
        -name 'davenstein-*.rpm' \
        -print |
        head -n 1
)

if [ -z "$BUILT_RPM" ] || [ ! -f "$BUILT_RPM" ]; then
    printf 'rpmbuild did not produce the expected x86_64 package\n' >&2
    exit 1
fi

# Rename the Distribution-Specific Internal RPM Filename to Davenstein's
# Stable Cross-Platform Release Naming Convention
install -m 644 "$BUILT_RPM" "$OUTPUT_PATH"

# Verify Core RPM Metadata Before Publishing the Artifact
if [ "$(rpm -qp --queryformat '%{NAME}' "$OUTPUT_PATH")" != "davenstein" ]; then
    printf 'Generated RPM has an unexpected package name\n' >&2
    exit 1
fi

if [ "$(rpm -qp --queryformat '%{VERSION}' "$OUTPUT_PATH")" != "$RPM_VERSION" ]; then
    printf 'Generated RPM has an unexpected version\n' >&2
    exit 1
fi

if [ "$(rpm -qp --queryformat '%{ARCH}' "$OUTPUT_PATH")" != "$RPM_ARCH" ]; then
    printf 'Generated RPM has an unexpected architecture\n' >&2
    exit 1
fi

# Generate Matching SHA-256 Checksum
cd "$BUILD_DIR"
sha256sum "$OUTPUT_BASENAME" \
    > "$OUTPUT_BASENAME.sha256"

printf 'Created %s\n' "$OUTPUT_PATH"
printf 'Created %s\n' "$OUTPUT_PATH.sha256"
printf 'RPM version: %s\n' "$RPM_VERSION"
printf 'RPM release: %s\n' "$RPM_RELEASE"
printf 'RPM architecture: %s\n' "$RPM_ARCH"
