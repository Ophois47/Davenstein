#!/bin/sh
set -eu

# Build a native FreeBSD package from a previously cross-compiled executable.
# This script must run under FreeBSD because pkg create and package ABI
# detection come from the native FreeBSD package-management environment.

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

BUILD_DIR="$ROOT_DIR/target/freebsd"
STAGE_ROOT="$BUILD_DIR/package-root"
METADATA_DIR="$BUILD_DIR/metadata"
DEPENDENCY_ROWS="$BUILD_DIR/dependencies.rows"
DEPENDENCY_UCL="$BUILD_DIR/dependencies.ucl"

RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}
ARCH=${ARCH:-x86_64}
BINARY_PATH=${BINARY_PATH:-"$ROOT_DIR/target/x86_64-unknown-freebsd/release/Davenstein"}
ASSETS_PATH=${ASSETS_PATH:-"$ROOT_DIR/target/release/assets.pak"}
ICON_PATH=${ICON_PATH:-"$ROOT_DIR/packaging/linux/davenstein.png"}
DESKTOP_PATH=${DESKTOP_PATH:-"$SCRIPT_DIR/davenstein.desktop"}
PLIST_PATH=${PLIST_PATH:-"$SCRIPT_DIR/pkg-plist"}

PACKAGE_BASENAME="Davenstein-${RELEASE_VERSION}-freebsd-${ARCH}"
PACKAGE_PATH="$BUILD_DIR/$PACKAGE_BASENAME.pkg"
CHECKSUM_PATH="$PACKAGE_PATH.sha256"

if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein version\n' >&2
    exit 1
fi

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

# Package ABI uses FreeBSD terminology such as FreeBSD:14:amd64 even though
# the public release artifact uses the more portable architecture name x86_64.
PACKAGE_ABI=${PKG_ABI:-$(pkg config ABI)}

case "$PACKAGE_ABI" in
    FreeBSD:*:amd64)
        ;;
    *)
        printf 'Unsupported FreeBSD package ABI: %s\n' "$PACKAGE_ABI" >&2
        exit 1
        ;;
esac

rm -rf "$STAGE_ROOT" "$METADATA_DIR"
rm -f \
    "$DEPENDENCY_ROWS" \
    "$DEPENDENCY_UCL" \
    "$PACKAGE_PATH" \
    "$CHECKSUM_PATH" \
    "$BUILD_DIR"/davenstein-"$RELEASE_VERSION"*.pkg

install -d "$BUILD_DIR"
install -d "$METADATA_DIR"

install -d -m 755 \
    "$STAGE_ROOT/usr/local/bin" \
    "$STAGE_ROOT/usr/local/libexec/davenstein" \
    "$STAGE_ROOT/usr/local/share/applications" \
    "$STAGE_ROOT/usr/local/share/doc/davenstein" \
    "$STAGE_ROOT/usr/local/share/icons/hicolor/256x256/apps"

install -m 755 \
    "$BINARY_PATH" \
    "$STAGE_ROOT/usr/local/libexec/davenstein/Davenstein"

install -m 644 \
    "$ASSETS_PATH" \
    "$STAGE_ROOT/usr/local/libexec/davenstein/assets.pak"

install -m 644 \
    "$DESKTOP_PATH" \
    "$STAGE_ROOT/usr/local/share/applications/davenstein.desktop"

install -m 644 \
    "$ICON_PATH" \
    "$STAGE_ROOT/usr/local/share/icons/hicolor/256x256/apps/davenstein.png"

install -m 644 \
    "$ROOT_DIR/README.md" \
    "$STAGE_ROOT/usr/local/share/doc/davenstein/README.md"

cat > "$STAGE_ROOT/usr/local/bin/Davenstein" <<'LAUNCHER'
#!/bin/sh
set -eu

# Locate the installation prefix from this launcher rather than assuming that
# the caller started Davenstein from a particular working directory.
PREFIX=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
GAME_DIR="$PREFIX/libexec/davenstein"

# The release executable resolves assets.pak beside its own executable.
cd "$GAME_DIR"

exec "$GAME_DIR/Davenstein" "$@"
LAUNCHER

chmod 755 "$STAGE_ROOT/usr/local/bin/Davenstein"

: > "$DEPENDENCY_ROWS"

while IFS='|' read -r package_name package_origin
do
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

    if [ -z "$dependency" ]; then
        printf 'Could not resolve FreeBSD dependency %s from %s\n' \
            "$package_name" \
            "$package_origin" >&2
        exit 1
    fi

    package_version=${dependency##*|}

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

# Add commas between dependency entries while leaving the final entry clean.
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

pkg create \
    --verbose \
    --root-dir "$STAGE_ROOT" \
    --metadata "$METADATA_DIR" \
    --plist "$PLIST_PATH" \
    --out-dir "$BUILD_DIR"

CREATED_PACKAGE=

for candidate in "$BUILD_DIR"/davenstein-"$RELEASE_VERSION"*.pkg
do
    if [ -f "$candidate" ]; then
        CREATED_PACKAGE=$candidate
        break
    fi
done

if [ -z "$CREATED_PACKAGE" ]; then
    printf 'pkg create did not produce the expected package\n' >&2
    exit 1
fi

mv "$CREATED_PACKAGE" "$PACKAGE_PATH"

package_hash=$(sha256 -q "$PACKAGE_PATH")

printf '%s  %s\n' \
    "$package_hash" \
    "$(basename "$PACKAGE_PATH")" \
    > "$CHECKSUM_PATH"

printf 'Created %s\n' "$PACKAGE_PATH"
printf 'Created %s\n' "$CHECKSUM_PATH"
