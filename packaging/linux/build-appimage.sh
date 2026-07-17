#!/bin/sh
set -eu

# Resolve every project path from this script so it can be launched from any directory
ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/appimage"
APPDIR="$BUILD_DIR/Davenstein.AppDir"
TOOLS_DIR="$BUILD_DIR/tools"

# Keep downloaded third-party tooling under target so it never enters the repository
DEFAULT_LINUXDEPLOY="$TOOLS_DIR/linuxdeploy-x86_64.AppImage"
LINUXDEPLOY=${LINUXDEPLOY:-"$DEFAULT_LINUXDEPLOY"}

# Use a dated linuxdeploy release rather than the mutable continuous release
LINUXDEPLOY_RELEASE="1-alpha-20251107-1"
LINUXDEPLOY_URL="https://github.com/linuxdeploy/linuxdeploy/releases/download/$LINUXDEPLOY_RELEASE/linuxdeploy-x86_64.AppImage"
LINUXDEPLOY_CHECKSUM_FILE="$ROOT_DIR/packaging/linux/linuxdeploy-x86_64.sha256"

# Allow release automation to override VERSION with the complete Git tag version
RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}
OUTPUT_NAME="Davenstein-${RELEASE_VERSION}-x86_64.AppImage"

# Prevent the generic VERSION variable from changing appimagetool behavior
unset VERSION

download_linuxdeploy() {
    temporary_path="${DEFAULT_LINUXDEPLOY}.download"

    command -v curl >/dev/null 2>&1 || {
        printf 'curl is required to download linuxdeploy\n' >&2
        exit 1
    }

    mkdir -p "$TOOLS_DIR"
    rm -f "$temporary_path"

    printf 'Downloading linuxdeploy %s\n' "$LINUXDEPLOY_RELEASE"

    curl -fL \
        --retry 3 \
        --retry-delay 2 \
        -o "$temporary_path" \
        "$LINUXDEPLOY_URL"

    chmod +x "$temporary_path"
    mv "$temporary_path" "$DEFAULT_LINUXDEPLOY"
}

verify_default_linuxdeploy() {
    if [ ! -f "$LINUXDEPLOY_CHECKSUM_FILE" ]; then
        printf 'linuxdeploy checksum file was not found at %s\n' \
            "$LINUXDEPLOY_CHECKSUM_FILE" >&2
        return 1
    fi

    expected_checksum=$(awk 'NF { print $1; exit }' "$LINUXDEPLOY_CHECKSUM_FILE")
    actual_checksum=$(sha256sum "$DEFAULT_LINUXDEPLOY" | awk '{ print $1 }')

    if [ -z "$expected_checksum" ]; then
        printf 'linuxdeploy checksum file is empty\n' >&2
        return 1
    fi

    if [ "$actual_checksum" != "$expected_checksum" ]; then
        printf 'linuxdeploy checksum mismatch\n' >&2
        printf 'Expected: %s\n' "$expected_checksum" >&2
        printf 'Actual:   %s\n' "$actual_checksum" >&2
        return 1
    fi
}

# Automatically provision the pinned tool for normal local and CI builds
if [ "$LINUXDEPLOY" = "$DEFAULT_LINUXDEPLOY" ]; then
    if [ ! -x "$DEFAULT_LINUXDEPLOY" ]; then
        download_linuxdeploy
    fi

    if ! verify_default_linuxdeploy; then
        printf 'Replacing the invalid linuxdeploy download\n' >&2
        rm -f "$DEFAULT_LINUXDEPLOY"
        download_linuxdeploy

        if ! verify_default_linuxdeploy; then
            printf 'Downloaded linuxdeploy failed checksum verification\n' >&2
            exit 1
        fi
    fi
elif [ ! -x "$LINUXDEPLOY" ]; then
    printf 'Custom linuxdeploy executable was not found at %s\n' \
        "$LINUXDEPLOY" >&2
    exit 1
fi

if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein version\n' >&2
    exit 1
fi

cd "$ROOT_DIR"

# Rebuild the game so an AppImage can never reuse a stale local executable
cargo build --release --locked --bin Davenstein

mkdir -p target/release

# Rebuild the standalone PAK generator from the current repository source
rustc --edition=2024 -O \
    src/pak_builder.rs \
    -o target/release/pak_builder

# Rebuild the complete asset archive so changed assets cannot be omitted
./target/release/pak_builder \
    --root assets \
    --out target/release/assets.pak

# Construct every AppDir from scratch to prevent files from prior builds surviving
rm -rf "$APPDIR"
rm -f \
    "$BUILD_DIR"/Davenstein-x86_64.AppImage \
    "$BUILD_DIR"/Davenstein-*-x86_64.AppImage \
    "$BUILD_DIR"/Davenstein-*-x86_64.AppImage.sha256

mkdir -p "$APPDIR/usr/bin"

# Keep assets.pak beside the executable because the runtime resolves it there
install -m 755 \
    target/release/Davenstein \
    "$APPDIR/usr/bin/Davenstein"

install -m 644 \
    target/release/assets.pak \
    "$APPDIR/usr/bin/assets.pak"

cd "$BUILD_DIR"

# Exclude WSL-injected Windows paths while linuxdeploy searches for plugins
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
APPIMAGE_EXTRACT_AND_RUN=1 \
LINUXDEPLOY_OUTPUT_VERSION="$RELEASE_VERSION" \
"$LINUXDEPLOY" \
    --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/Davenstein" \
    --desktop-file "$ROOT_DIR/packaging/linux/davenstein.desktop" \
    --icon-file "$ROOT_DIR/packaging/linux/davenstein.png" \
    --output appimage

# Verify the versioned deliverable and generate its matching checksum
if [ ! -f "$OUTPUT_NAME" ]; then
    printf 'Expected AppImage was not created at %s\n' "$BUILD_DIR/$OUTPUT_NAME" >&2
    exit 1
fi

sha256sum "$OUTPUT_NAME" > "$OUTPUT_NAME.sha256"

printf 'Created %s\n' "$BUILD_DIR/$OUTPUT_NAME"
printf 'Created %s\n' "$BUILD_DIR/$OUTPUT_NAME.sha256"
