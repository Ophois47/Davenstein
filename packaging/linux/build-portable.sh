#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BUILD_DIR="$ROOT_DIR/target/portable"
STAGE_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/davenstein-portable.XXXXXX")

RELEASE_VERSION=${VERSION:-$(sed -nE 's/^version = "([^"]+)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n 1)}
ARCHIVE_BASENAME="Davenstein-${RELEASE_VERSION}-linux-x86_64"
STAGE_DIR="$STAGE_ROOT/$ARCHIVE_BASENAME"
ARCHIVE_PATH="$BUILD_DIR/$ARCHIVE_BASENAME.tar.gz"

trap 'rm -rf "$STAGE_ROOT"' EXIT HUP INT TERM

if [ -z "$RELEASE_VERSION" ]; then
    printf 'Could not determine the Davenstein version\n' >&2
    exit 1
fi

for required_file in \
    "$ROOT_DIR/target/release/Davenstein" \
    "$ROOT_DIR/target/release/assets.pak" \
    "$ROOT_DIR/README.md" \
    "$ROOT_DIR/packaging/linux/davenstein.png"
do
    if [ ! -f "$required_file" ]; then
        printf 'Required portable-build input was not found at %s\n' \
            "$required_file" >&2
        exit 1
    fi
done

rm -f "$ARCHIVE_PATH" "$ARCHIVE_PATH.sha256"

install -d "$BUILD_DIR"
install -d -m 755 "$STAGE_DIR"

install -m 755 \
    "$ROOT_DIR/target/release/Davenstein" \
    "$STAGE_DIR/Davenstein"

install -m 644 \
    "$ROOT_DIR/target/release/assets.pak" \
    "$STAGE_DIR/assets.pak"

install -m 644 \
    "$ROOT_DIR/README.md" \
    "$STAGE_DIR/README.md"

install -m 644 \
    "$ROOT_DIR/packaging/linux/davenstein.png" \
    "$STAGE_DIR/davenstein.png"

: > "$STAGE_DIR/portable.flag"

cat > "$STAGE_DIR/run-davenstein.sh" <<'LAUNCHER'
#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

exec ./Davenstein "$@"
LAUNCHER

chmod 755 "$STAGE_DIR/run-davenstein.sh"
chmod 644 "$STAGE_DIR/portable.flag"

tar \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    -C "$STAGE_ROOT" \
    -czf "$ARCHIVE_PATH" \
    "$ARCHIVE_BASENAME"

cd "$BUILD_DIR"
sha256sum "$ARCHIVE_BASENAME.tar.gz" \
    > "$ARCHIVE_BASENAME.tar.gz.sha256"

printf 'Created %s\n' "$ARCHIVE_PATH"
printf 'Created %s\n' "$ARCHIVE_PATH.sha256"
