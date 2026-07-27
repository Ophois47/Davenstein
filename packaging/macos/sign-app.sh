#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Signs an Existing macOS Application Bundle for Developer ID Distribution
#
# Application Signing Process:
#     - Validate the Application Bundle Layout and Metadata
#     - Remove Extended Attributes From the Application Bundle
#     - Sign the Main Executable With Hardened Runtime and Secure Timestamping
#     - Sign the Outer Application Bundle With Hardened Runtime and Secure Timestamping
#     - Strictly Verify the Main Executable and Complete Application Bundle
#     - Confirm the Expected Bundle Identifier, Signing Identity, Team ID, Timestamp, and Runtime
#
# This Script Does Not Construct, Notarize, Staple, or Archive the Application
#
# Release Automation May Override:
#     APP_BUNDLE              Existing Davenstein.app Bundle
#     SIGNING_IDENTITY        Developer ID Application Signing Identity
#     EXPECTED_TEAM_ID        Apple Developer Team Identifier
#     EXPECTED_BUNDLE_ID      Expected macOS Application Bundle Identifier
#

# Resolve Repository Paths Relative to this Script
# Script May be Launched From Any Working Directory
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

# Existing Application Bundle and Expected Developer Identity
APP_BUNDLE=${APP_BUNDLE:-"$ROOT_DIR/target/macos/Davenstein.app"}
SIGNING_IDENTITY=${SIGNING_IDENTITY:-"Developer ID Application: CyberSoft Operating Corporation (923379G559)"}
EXPECTED_TEAM_ID=${EXPECTED_TEAM_ID:-"923379G559"}
EXPECTED_BUNDLE_ID=${EXPECTED_BUNDLE_ID:-"com.davidpetnick.davenstein"}

# Required Application Bundle Paths
INFO_PLIST="$APP_BUNDLE/Contents/Info.plist"
EXECUTABLE_PATH="$APP_BUNDLE/Contents/MacOS/Davenstein"
ASSETS_PATH="$APP_BUNDLE/Contents/Resources/assets.pak"
LEGACY_ASSETS_PATH="$APP_BUNDLE/Contents/MacOS/assets.pak"

# Validate Required Signing Values Without Dynamic Shell Evaluation
test -n "$APP_BUNDLE" || {
    printf '%s\n' 'APP_BUNDLE must not be empty' >&2
    exit 1
}

test -n "$SIGNING_IDENTITY" || {
    printf '%s\n' 'SIGNING_IDENTITY must not be empty' >&2
    exit 1
}

test -n "$EXPECTED_TEAM_ID" || {
    printf '%s\n' 'EXPECTED_TEAM_ID must not be empty' >&2
    exit 1
}

test -n "$EXPECTED_BUNDLE_ID" || {
    printf '%s\n' 'EXPECTED_BUNDLE_ID must not be empty' >&2
    exit 1
}

# Validate Required Application Bundle Contents
test -d "$APP_BUNDLE" || {
    printf 'Application bundle was not found at %s\n' "$APP_BUNDLE" >&2
    exit 1
}

for required_file in \
    "$INFO_PLIST" \
    "$EXECUTABLE_PATH" \
    "$ASSETS_PATH"
do
    test -f "$required_file" || {
        printf 'Required application bundle file was not found at %s\n' \
            "$required_file" >&2
        exit 1
    }
done

test -x "$EXECUTABLE_PATH" || {
    printf 'Application executable is not executable at %s\n' \
        "$EXECUTABLE_PATH" >&2
    exit 1
}

test ! -e "$LEGACY_ASSETS_PATH" || {
    printf 'assets.pak must not remain beside the executable at %s\n' \
        "$LEGACY_ASSETS_PATH" >&2
    exit 1
}

# Validate Native macOS Signing and Metadata Utilities
for required_command in \
    codesign \
    grep \
    mktemp \
    plutil \
    security \
    xattr
do
    command -v "$required_command" >/dev/null 2>&1 || {
        printf '%s is required to sign the macOS application\n' \
            "$required_command" >&2
        exit 1
    }
done

# Validate the Application Bundle Identifier Before Signing
bundle_id=$(
    plutil \
        -extract CFBundleIdentifier \
        raw \
        -o - \
        "$INFO_PLIST"
)

if [ "$bundle_id" != "$EXPECTED_BUNDLE_ID" ]; then
    printf 'Unexpected application bundle identifier: %s\n' \
        "$bundle_id" >&2
    exit 1
fi

# Confirm the Requested Developer ID Identity Before Modifying the Bundle
security find-identity -v -p codesigning |
    grep -F -- "\"$SIGNING_IDENTITY\"" >/dev/null || {
        printf 'Signing identity was not found: %s\n' \
            "$SIGNING_IDENTITY" >&2
        exit 1
    }

# Inspect Any Existing Signature Before Modifying the Application Bundle
if existing_signature_details=$(
    codesign \
        --display \
        --verbose=4 \
        "$APP_BUNDLE" \
        2>&1
); then
    # Permit Only the Expected Linker-Generated Ad Hoc Signature
    if ! printf '%s\n' "$existing_signature_details" |
        grep -Fqx 'Signature=adhoc' ||
        ! printf '%s\n' "$existing_signature_details" |
        grep -Fqx 'TeamIdentifier=not set' ||
        printf '%s\n' "$existing_signature_details" |
        grep -Eq '^Authority='
    then
        printf 'Application bundle already has a non-ad-hoc signature: %s\n' \
            "$APP_BUNDLE" >&2
        exit 1
    fi
fi

# Remove Extended Attributes Before Creating the Final Code Signature
xattr -cr "$APP_BUNDLE"

# Sign the Main Executable Before Signing the Containing Application Bundle
codesign \
    --force \
    --sign "$SIGNING_IDENTITY" \
    --options runtime \
    --timestamp \
    --verbose=2 \
    "$EXECUTABLE_PATH"

# Sign the Complete Application Bundle After All Nested Code is Signed
codesign \
    --force \
    --sign "$SIGNING_IDENTITY" \
    --options runtime \
    --timestamp \
    --verbose=2 \
    "$APP_BUNDLE"

# Strictly Verify the Signed Main Executable and Complete Bundle
codesign \
    --verify \
    --strict \
    --verbose=4 \
    "$EXECUTABLE_PATH"

codesign \
    --verify \
    --deep \
    --strict \
    --verbose=4 \
    "$APP_BUNDLE"

# Capture the Final Signature Metadata for Explicit Policy Validation
signature_details=$(mktemp)
trap 'rm -f "$signature_details"' EXIT HUP INT TERM

codesign \
    --display \
    --verbose=4 \
    "$APP_BUNDLE" \
    >/dev/null \
    2>"$signature_details"

grep -Fqx \
    "Identifier=$EXPECTED_BUNDLE_ID" \
    "$signature_details" || {
        printf '%s\n' 'Signed bundle identifier validation failed' >&2
        exit 1
    }

grep -Fqx \
    "Authority=$SIGNING_IDENTITY" \
    "$signature_details" || {
        printf '%s\n' 'Developer ID signing identity validation failed' >&2
        exit 1
    }

grep -Fqx \
    "TeamIdentifier=$EXPECTED_TEAM_ID" \
    "$signature_details" || {
        printf '%s\n' 'Apple Developer Team ID validation failed' >&2
        exit 1
    }

grep -Eq \
    '^Timestamp=' \
    "$signature_details" || {
        printf '%s\n' 'Secure signing timestamp was not found' >&2
        exit 1
    }

grep -Eq \
    '^Runtime Version=' \
    "$signature_details" || {
        printf '%s\n' 'Hardened Runtime metadata was not found' >&2
        exit 1
    }

printf 'Signed %s\n' "$APP_BUNDLE"
printf 'Signing identity: %s\n' "$SIGNING_IDENTITY"
printf 'Team identifier: %s\n' "$EXPECTED_TEAM_ID"

printf '%s\n' 'Verified signature metadata:'
grep -E \
    '^(Identifier|Format|Authority|Timestamp|TeamIdentifier|Runtime Version)=' \
    "$signature_details"
