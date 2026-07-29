#!/usr/bin/env bash

set -euo pipefail

readonly EXPECTED_TARGET="powerpc64le-unknown-linux-gnu"
readonly EXPECTED_DEB_ARCH="ppc64el"
readonly TARGET="${1:?Cross Did Not Pass The Target Triple}"

if [[ "$TARGET" != "$EXPECTED_TARGET" ]]
then
    printf "Unexpected Cross Target: %s\n" "$TARGET" >&2
    exit 1
fi

if [[ "${CROSS_DEB_ARCH:-}" != "$EXPECTED_DEB_ARCH" ]]
then
    printf "Unexpected Debian Architecture: %s\n" "${CROSS_DEB_ARCH:-unset}" >&2
    exit 1
fi

readonly SOURCE_FILE="/etc/apt/sources.list.d/ubuntu-ppc64el.sources"
readonly PACKAGE_ROOT="/tmp/davenstein-powerpc64le-package-root"
readonly TARGET_LIBRARY_DIRECTORY="/usr/lib/powerpc64le-linux-gnu"
readonly TARGET_PKG_CONFIG_DIRECTORY="${TARGET_LIBRARY_DIRECTORY}/pkgconfig"
readonly LINK_PROBE="/tmp/davenstein-powerpc64le-link-probe"

readonly -a PACKAGES=(
    "libwayland-dev:${EXPECTED_DEB_ARCH}"
    "libasound2-dev:${EXPECTED_DEB_ARCH}"
    "libudev-dev:${EXPECTED_DEB_ARCH}"
    "libx11-dev:${EXPECTED_DEB_ARCH}"
    "libxcursor-dev:${EXPECTED_DEB_ARCH}"
    "libxi-dev:${EXPECTED_DEB_ARCH}"
    "libxkbcommon-dev:${EXPECTED_DEB_ARCH}"
    "libxrandr-dev:${EXPECTED_DEB_ARCH}"
)

# The Cross Image Already Provides The PowerPC64LE Compiler And glibc Sysroot.
# Download The Bevy Platform Packages Without Installing Their Foreign glibc
# Into The Image's Native amd64 Package Database.
dpkg --add-architecture "$EXPECTED_DEB_ARCH"

printf "%s\n" \
    "Types: deb" \
    "URIs: http://ports.ubuntu.com/ubuntu-ports" \
    "Suites: noble noble-updates noble-backports noble-security" \
    "Architectures: ${EXPECTED_DEB_ARCH}" \
    "Components: main restricted universe multiverse" \
    "Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg" \
    > "$SOURCE_FILE"

apt-get update

rm -f /var/cache/apt/archives/*.deb

DEBIAN_FRONTEND=noninteractive \
    apt-get \
        -o Dpkg::Use-Pty=0 \
        install \
        --download-only \
        --assume-yes \
        --no-install-recommends \
        "${PACKAGES[@]}"

mkdir -p \
    /usr/include \
    "$TARGET_LIBRARY_DIRECTORY" \
    "$TARGET_PKG_CONFIG_DIRECTORY"

shopt -s nullglob
DEB_PACKAGES=(/var/cache/apt/archives/*.deb)

if (( ${#DEB_PACKAGES[@]} == 0 ))
then
    printf "No Debian Packages Were Downloaded\n" >&2
    exit 1
fi

for DEB in "${DEB_PACKAGES[@]}"
do
    ARCHITECTURE="$(dpkg-deb -f "$DEB" Architecture)"
    PACKAGE="$(dpkg-deb -f "$DEB" Package)"

    case "${ARCHITECTURE}:${PACKAGE}" in
        ppc64el:libc6)
            printf "Skipping %-32s %s\n" "$PACKAGE" "$ARCHITECTURE"
            continue
            ;;
        ppc64el:*|all:*)
            printf "Extracting %-30s %s\n" "$PACKAGE" "$ARCHITECTURE"
            ;;
        *)
            printf "Ignoring %-32s %s\n" "$PACKAGE" "$ARCHITECTURE"
            continue
            ;;
    esac

    rm -rf "$PACKAGE_ROOT"
    mkdir -p "$PACKAGE_ROOT"

    dpkg-deb -x "$DEB" "$PACKAGE_ROOT"

    if [[ -d "$PACKAGE_ROOT/usr/include" ]]
    then
        cp -a \
            "$PACKAGE_ROOT/usr/include/." \
            /usr/include/
    fi

    if [[ -d "$PACKAGE_ROOT/usr/lib/powerpc64le-linux-gnu" ]]
    then
        cp -a \
            "$PACKAGE_ROOT/usr/lib/powerpc64le-linux-gnu/." \
            "$TARGET_LIBRARY_DIRECTORY/"
    fi

    if [[ -d "$PACKAGE_ROOT/lib/powerpc64le-linux-gnu" ]]
    then
        cp -a \
            "$PACKAGE_ROOT/lib/powerpc64le-linux-gnu/." \
            "$TARGET_LIBRARY_DIRECTORY/"
    fi

    # Architecture-Independent X11 Protocol Metadata Must Reside Beside The
    # Target Metadata Because Cross Uses The Target pkg-config Directory.
    if [[ -d "$PACKAGE_ROOT/usr/share/pkgconfig" ]]
    then
        cp -a \
            "$PACKAGE_ROOT/usr/share/pkgconfig/." \
            "$TARGET_PKG_CONFIG_DIRECTORY/"
    fi
done

# Validate Every Native Library Family Before Building The Rust Application.
for PACKAGE in \
    wayland-client \
    alsa \
    libudev \
    x11 \
    xcursor \
    xi \
    xkbcommon \
    xrandr
do
    printf "%-18s " "$PACKAGE"

    PKG_CONFIG_LIBDIR="$TARGET_PKG_CONFIG_DIRECTORY" \
        pkg-config --modversion "$PACKAGE"
done

printf "int main(void) { return 0; }\n" |
    powerpc64le-linux-gnu-gcc \
        -x c \
        - \
        -o "$LINK_PROBE" \
        $(
            PKG_CONFIG_LIBDIR="$TARGET_PKG_CONFIG_DIRECTORY" \
                pkg-config \
                    --libs \
                    wayland-client \
                    alsa \
                    libudev \
                    x11 \
                    xcursor \
                    xi \
                    xkbcommon \
                    xrandr
        )

powerpc64le-linux-gnu-readelf -h "$LINK_PROBE" |
    grep -F "Machine:                           PowerPC64"

powerpc64le-linux-gnu-readelf -l "$LINK_PROBE" |
    grep -F "[Requesting program interpreter: /lib64/ld64.so.2]"

rm -rf \
    "$PACKAGE_ROOT" \
    "$LINK_PROBE" \
    "$SOURCE_FILE" \
    /var/lib/apt/lists/* \
    /var/cache/apt/archives/*.deb
