#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Extracts and Independently Verifies a Final macOS Release Archive
#
# macOS Release Archive Verification Process:
#     - Validate Release Metadata and Required Archive Files
#     - Verify the Published SHA-256 Checksum
#     - Extract the Archive Into a Unique Temporary Directory
#     - Validate the Extracted Application Bundle Layout and Metadata
#     - Strictly Verify the Developer ID Signature
#     - Confirm the Expected Bundle Identifier, Signing Identity, Team ID,
#       Timestamp, and Hardened Runtime
#     - Validate the Stapled Apple Notarization Ticket
#     - Require Gatekeeper to Accept the Extracted Application
#     - Confirm the Expected Executable Architectures
#
# Temporary Verification Files Are Removed After Complete Success
# Temporary Verification Files Are Retained for Failure Diagnostics
#
# This Script Does Not Build, Sign, Notarize, Staple, or Modify the Application
#
# Release Automation May Override:
#     VERSION                  Complete Release Version or Git Tag
#     ARCH                     Public Release Architecture Name
#     ZIP_PATH                 Existing macOS Release Archive
#     CHECKSUM_PATH            Existing SHA-256 Checksum
#     SIGNING_IDENTITY         Expected Developer ID Application Identity
#     EXPECTED_TEAM_ID         Expected Apple Developer Team Identifier
#     EXPECTED_BUNDLE_ID       Expected macOS Application Bundle Identifier
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

# Cargo Package Version Used as the Default Release Version
CARGO_VERSION=$(
    sed -nE \
        's/^version = "([^"]+)"/\1/p' \
        "$ROOT_DIR/Cargo.toml" |
        head -n 1
)

# Release Metadata and Expected Developer Identity
RELEASE_VERSION=${VERSION:-"$CARGO_VERSION"}
ARCH=${ARCH:-aarch64}
SIGNING_IDENTITY=${SIGNING_IDENTITY:-"Developer ID Application: CyberSoft Operating Corporation (923379G559)"}
EXPECTED_TEAM_ID=${EXPECTED_TEAM_ID:-"923379G559"}
EXPECTED_BUNDLE_ID=${EXPECTED_BUNDLE_ID:-"com.davidpetnick.davenstein"}

# Existing Versioned macOS Archive and Checksum
OUTPUT_BASENAME="Davenstein-${RELEASE_VERSION}-macos-${ARCH}"
ZIP_PATH=${ZIP_PATH:-"$ROOT_DIR/target/macos/$OUTPUT_BASENAME.zip"}
CHECKSUM_PATH=${CHECKSUM_PATH:-"$ZIP_PATH.sha256"}

# Temporary Work Directory Is Assigned After Initial Validation
WORK_DIR=

fail() {
    printf 'macOS release archive verification failed: %s\n' "$1" >&2

    if [ -n "$WORK_DIR" ]; then
        printf 'Retained verification work directory: %s\n' \
            "$WORK_DIR" >&2
    fi

    exit 1
}

# Validate Required Release and Developer Identity Values
test -n "$RELEASE_VERSION" ||
    fail "RELEASE_VERSION could not be determined"

test -n "$ARCH" ||
    fail "ARCH must not be empty"

test -n "$ZIP_PATH" ||
    fail "ZIP_PATH must not be empty"

test -n "$CHECKSUM_PATH" ||
    fail "CHECKSUM_PATH must not be empty"

test -n "$SIGNING_IDENTITY" ||
    fail "SIGNING_IDENTITY must not be empty"

test -n "$EXPECTED_TEAM_ID" ||
    fail "EXPECTED_TEAM_ID must not be empty"

test -n "$EXPECTED_BUNDLE_ID" ||
    fail "EXPECTED_BUNDLE_ID must not be empty"

# Select the Required Executable Architectures
case "$ARCH" in
    aarch64)
        EXPECTED_ARCHITECTURES="arm64"
        ;;
    universal)
        EXPECTED_ARCHITECTURES="arm64 x86_64"
        ;;
    *)
        fail "Unsupported macOS release architecture: $ARCH"
        ;;
esac

# Validate the Existing Archive and Checksum
test -f "$ZIP_PATH" ||
    fail "Release archive was not found at $ZIP_PATH"

test -s "$ZIP_PATH" ||
    fail "Release archive is empty at $ZIP_PATH"

test -f "$CHECKSUM_PATH" ||
    fail "Release checksum was not found at $CHECKSUM_PATH"

test -s "$CHECKSUM_PATH" ||
    fail "Release checksum is empty at $CHECKSUM_PATH"

# Validate Native macOS Archive and Security Utilities
for required_command in \
    awk \
    codesign \
    ditto \
    grep \
    lipo \
    mktemp \
    plutil \
    sed \
    shasum \
    spctl \
    xcrun
do
    command -v "$required_command" >/dev/null 2>&1 ||
        fail "$required_command is required to verify the macOS release archive"
done

# Verify the Published Archive Checksum From Its Containing Directory
ZIP_DIRECTORY=$(CDPATH= cd -- "$(dirname -- "$ZIP_PATH")" && pwd)
ZIP_NAME=${ZIP_PATH##*/}
CHECKSUM_NAME=${CHECKSUM_PATH##*/}

test "$CHECKSUM_NAME" = "$ZIP_NAME.sha256" ||
    fail "Checksum filename does not match the release archive"

(
    cd "$ZIP_DIRECTORY" ||
        exit 1

    shasum \
        -a 256 \
        -c "$CHECKSUM_NAME"
) || fail "Release archive checksum verification failed"

# Create a Unique Work Directory for Extraction and Diagnostics
WORK_DIR=$(
    mktemp \
        -d \
        "${TMPDIR:-/tmp}/davenstein-archive-verification.XXXXXX"
) || fail "Could not create the verification work directory"

trap 'fail "Archive verification was interrupted"' HUP INT TERM

# Extract the Final Distribution Archive Without Modifying It
ditto \
    -x \
    -k \
    "$ZIP_PATH" \
    "$WORK_DIR" ||
    fail "Could not extract the macOS release archive"

APP_BUNDLE="$WORK_DIR/Davenstein.app"
INFO_PLIST="$APP_BUNDLE/Contents/Info.plist"
EXECUTABLE_PATH="$APP_BUNDLE/Contents/MacOS/Davenstein"
ASSETS_PATH="$APP_BUNDLE/Contents/Resources/assets.pak"
LEGACY_ASSETS_PATH="$APP_BUNDLE/Contents/MacOS/assets.pak"
SIGNATURE_DETAILS="$WORK_DIR/signature-details.txt"

# Validate the Extracted Application Bundle Contents
test -d "$APP_BUNDLE" ||
    fail "Extracted Davenstein.app bundle was not found"

for required_file in \
    "$INFO_PLIST" \
    "$EXECUTABLE_PATH" \
    "$ASSETS_PATH"
do
    test -f "$required_file" ||
        fail "Required extracted application file was not found at $required_file"
done

test -x "$EXECUTABLE_PATH" ||
    fail "Extracted application executable is not executable"

test ! -e "$LEGACY_ASSETS_PATH" ||
    fail "assets.pak must not remain beside the extracted executable"

# Validate the Extracted Application Bundle Identifier
bundle_id=$(
    plutil \
        -extract CFBundleIdentifier \
        raw \
        -o - \
        "$INFO_PLIST"
) || fail "Could not read the extracted application bundle identifier"

test "$bundle_id" = "$EXPECTED_BUNDLE_ID" ||
    fail "Extracted application bundle identifier does not match"

# Strictly Verify the Extracted Developer ID Signed Executable and Bundle
codesign \
    --verify \
    --strict \
    --verbose=2 \
    "$EXECUTABLE_PATH" ||
    fail "Extracted application executable signature verification failed"

codesign \
    --verify \
    --deep \
    --strict \
    --verbose=2 \
    "$APP_BUNDLE" ||
    fail "Extracted application bundle signature verification failed"

# Capture the Extracted Signature Metadata for Explicit Policy Validation
codesign \
    -d \
    --verbose=4 \
    "$APP_BUNDLE" \
    > "$SIGNATURE_DETAILS" 2>&1 ||
    fail "Could not inspect the extracted application signature"

grep -Fx \
    "Identifier=$EXPECTED_BUNDLE_ID" \
    "$SIGNATURE_DETAILS" >/dev/null ||
    fail "Extracted signature bundle identifier does not match"

grep -Fx \
    "Authority=$SIGNING_IDENTITY" \
    "$SIGNATURE_DETAILS" >/dev/null ||
    fail "Extracted Developer ID signing identity does not match"

grep -Fx \
    "TeamIdentifier=$EXPECTED_TEAM_ID" \
    "$SIGNATURE_DETAILS" >/dev/null ||
    fail "Extracted signature Team ID does not match"

grep -E \
    '^Timestamp=' \
    "$SIGNATURE_DETAILS" >/dev/null ||
    fail "Extracted signature does not contain a secure timestamp"

grep -E \
    '^Runtime Version=' \
    "$SIGNATURE_DETAILS" >/dev/null ||
    fail "Extracted signature does not enable Hardened Runtime"

# Validate the Stapled Apple Notarization Ticket
xcrun stapler \
    validate \
    "$APP_BUNDLE" ||
    fail "Extracted application does not contain a valid stapled ticket"

# Require Gatekeeper to Accept the Extracted Application
spctl \
    --assess \
    --type execute \
    --verbose=4 \
    "$APP_BUNDLE" ||
    fail "Gatekeeper rejected the extracted application"

# Confirm the Exact Executable Architectures
actual_architectures=$(
    lipo \
        -archs \
        "$EXECUTABLE_PATH"
) || fail "Could not inspect the extracted executable architectures"

for expected_architecture in $EXPECTED_ARCHITECTURES
do
    printf '%s\n' "$actual_architectures" |
        grep -E "(^|[[:space:]])$expected_architecture([[:space:]]|$)" \
            >/dev/null ||
        fail "Extracted executable is missing $expected_architecture"
done

actual_architecture_count=$(
    printf '%s\n' "$actual_architectures" |
        sed -E \
            's/^[[:space:]]+//; s/[[:space:]]+$//' |
        awk '{ print NF }'
) || fail "Could not count the extracted executable architectures"

expected_architecture_count=$(
    printf '%s\n' "$EXPECTED_ARCHITECTURES" |
        awk '{ print NF }'
) || fail "Could not count the expected executable architectures"

test "$actual_architecture_count" -eq "$expected_architecture_count" ||
    fail "Extracted executable contains unexpected architectures"

# Remove Extracted Verification Material Only After Complete Success
rm -rf "$WORK_DIR"
WORK_DIR=

trap - HUP INT TERM

printf 'Verified macOS release archive: %s\n' "$ZIP_PATH"
printf 'Verified executable architectures: %s\n' "$actual_architectures"
printf 'Verified signing identity: %s\n' "$SIGNING_IDENTITY"
printf 'Verified Apple Team ID: %s\n' "$EXPECTED_TEAM_ID"
printf 'Verified bundle identifier: %s\n' "$EXPECTED_BUNDLE_ID"
