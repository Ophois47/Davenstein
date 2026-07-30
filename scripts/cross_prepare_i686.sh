#!/usr/bin/env bash

set -euo pipefail

readonly EXPECTED_DEB_ARCH="i386"
readonly LINK_PROBE_SOURCE="/tmp/davenstein-i686-link-probe.c"
readonly LINK_PROBE="/tmp/davenstein-i686-link-probe"

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

readonly -a PKG_CONFIG_PACKAGES=(
    "wayland-client"
    "alsa"
    "libudev"
    "x11"
    "xcursor"
    "xi"
    "xkbcommon"
    "xrandr"
)

TARGET_COMPILER="$(command -v i686-linux-gnu-gcc || true)"
TARGET_READELF="$(command -v i686-linux-gnu-readelf || true)"

if [[ -z "${TARGET_COMPILER}" ]]
then
    printf '%s\n' "Missing Target Compiler: i686-linux-gnu-gcc" >&2
    exit 1
fi

if [[ -z "${TARGET_READELF}" ]]
then
    printf '%s\n' "Missing Target Readelf: i686-linux-gnu-readelf" >&2
    exit 1
fi

if ! dpkg --print-foreign-architectures |
    grep --fixed-strings --line-regexp --quiet "${EXPECTED_DEB_ARCH}"
then
    dpkg --add-architecture "${EXPECTED_DEB_ARCH}"
fi

apt-get update

DEBIAN_FRONTEND=noninteractive \
    apt-get \
        install \
        --assume-yes \
        --no-install-recommends \
        "${PACKAGES[@]}"

# Restrict pkg-config To The i386 Metadata So Host amd64 Libraries Cannot Be
# Selected By Native Build Scripts During The 32-Bit Cross Compilation.
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_LIBDIR="/usr/lib/i386-linux-gnu/pkgconfig:/usr/share/pkgconfig"
export PKG_CONFIG_PATH="/usr/lib/i386-linux-gnu/pkgconfig"

for PACKAGE in "${PKG_CONFIG_PACKAGES[@]}"
do
    printf 'Validating i686 pkg-config Package: %s\n' "${PACKAGE}"
    pkg-config --modversion "${PACKAGE}"
done

cat > "${LINK_PROBE_SOURCE}" <<'PROBE'
int main(void)
{
    return 0;
}
PROBE

# Link Every Native Bevy Platform Library Into One i686 Probe. Disabling
# As-Needed Keeps Each Dependency Visible For The Final ELF Inspection.
"${TARGET_COMPILER}" \
    "${LINK_PROBE_SOURCE}" \
    -o "${LINK_PROBE}" \
    -Wl,--no-as-needed \
    $(pkg-config \
        --cflags \
        --libs \
        wayland-client \
        alsa \
        libudev \
        x11 \
        xcursor \
        xi \
        xkbcommon \
        xrandr)

"${TARGET_READELF}" -h "${LINK_PROBE}" |
    grep --extended-regexp --quiet "Class:[[:space:]]+ELF32"

"${TARGET_READELF}" -h "${LINK_PROBE}" |
    grep --extended-regexp --quiet \
        "Data:[[:space:]]+2.s complement, little endian"

"${TARGET_READELF}" -h "${LINK_PROBE}" |
    grep --extended-regexp --quiet "Machine:[[:space:]]+Intel 80386"

"${TARGET_READELF}" -l "${LINK_PROBE}" |
    grep --fixed-strings --quiet \
        "Requesting program interpreter: /lib/ld-linux.so.2"

printf '%s\n' \
    "Validated The i686 Native Library Set, ELF32 Identity, And Dynamic Loader."
