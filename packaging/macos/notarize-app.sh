#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Notarizes and Staples an Existing Developer ID Signed macOS Application
#
# Application Notarization Process:
#     - Validate the Application Bundle Layout and Metadata
#     - Strictly Verify the Existing Developer ID Signature
#     - Confirm the Expected Bundle Identifier, Signing Identity, Team ID,
#       Timestamp, and Hardened Runtime
#     - Confirm the notarytool Keychain Profile Can Authenticate
#     - Refuse an Application That Already Has a Valid Stapled Ticket
#     - Create a Temporary ZIP Archive for Apple Notarization
#     - Submit the Archive and Wait for Apple to Complete Processing
#     - Require an Accepted Notarization Result
#     - Retrieve the Apple Notarization Log
#     - Staple and Validate the Notarization Ticket
#     - Reverify the Code Signature and Run Gatekeeper Assessment
#
# Temporary Notarization Files Are Removed After Complete Success
# Temporary Notarization Files Are Retained for Failure Diagnostics
#
# This Script Does Not Construct, Sign, or Create the Final Release Archive
#
# Release Automation May Override:
#     APP_BUNDLE              Existing Developer ID Signed Davenstein.app Bundle
#     SIGNING_IDENTITY        Expected Developer ID Application Identity
#     EXPECTED_TEAM_ID        Apple Developer Team Identifier
#     EXPECTED_BUNDLE_ID      Expected macOS Application Bundle Identifier
#     NOTARYTOOL_PROFILE      Stored notarytool Keychain Profile
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
NOTARYTOOL_PROFILE=${NOTARYTOOL_PROFILE:-"DavensteinNotary"}

# Required Application Bundle Paths
INFO_PLIST="$APP_BUNDLE/Contents/Info.plist"
EXECUTABLE_PATH="$APP_BUNDLE/Contents/MacOS/Davenstein"
ASSETS_PATH="$APP_BUNDLE/Contents/Resources/assets.pak"
LEGACY_ASSETS_PATH="$APP_BUNDLE/Contents/MacOS/assets.pak"

# Temporary Work Directory Is Assigned After Initial Validation
WORK_DIR=

fail() {
    printf 'Notarization failed: %s\n' "$1" >&2

    if [ -n "$WORK_DIR" ]; then
        printf 'Retained notarization work directory: %s\n' \
            "$WORK_DIR" >&2
    fi

    exit 1
}

# Validate Required Notarization Values Without Dynamic Shell Evaluation
test -n "$APP_BUNDLE" ||
    fail "APP_BUNDLE must not be empty"

test -n "$SIGNING_IDENTITY" ||
    fail "SIGNING_IDENTITY must not be empty"

test -n "$EXPECTED_TEAM_ID" ||
    fail "EXPECTED_TEAM_ID must not be empty"

test -n "$EXPECTED_BUNDLE_ID" ||
    fail "EXPECTED_BUNDLE_ID must not be empty"

test -n "$NOTARYTOOL_PROFILE" ||
    fail "NOTARYTOOL_PROFILE must not be empty"

# Validate Required Application Bundle Contents
test -d "$APP_BUNDLE" || {
    printf 'Application bundle was not found at %s\n' \
        "$APP_BUNDLE" >&2
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

# Validate Native macOS Notarization and Verification Utilities
for required_command in \
    cat \
    codesign \
    ditto \
    grep \
    mktemp \
    plutil \
    rm \
    spctl \
    xcrun
do
    command -v "$required_command" >/dev/null 2>&1 || {
        printf '%s is required to notarize the macOS application\n' \
            "$required_command" >&2
        exit 1
    }
done

# Validate the Application Bundle Identifier Before Any Submission
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

# Create a Unique Work Directory for Submission and Diagnostics
WORK_DIR=$(
    mktemp -d \
        "${TMPDIR:-/tmp}/davenstein-notarization.XXXXXX"
) || fail "Could not create the temporary notarization work directory"

SIGNATURE_DETAILS="$WORK_DIR/signature-details.txt"
PRE_STAPLE_LOG="$WORK_DIR/pre-staple-validation.txt"
PROFILE_HISTORY="$WORK_DIR/notarytool-history.json"
SUBMISSION_ZIP="$WORK_DIR/Davenstein-notarization-submission.zip"
SUBMISSION_RESULT="$WORK_DIR/notarytool-submit.json"
SUBMISSION_ERROR="$WORK_DIR/notarytool-submit-stderr.txt"
NOTARIZATION_LOG="$WORK_DIR/notarization-log.json"
STAPLE_LOG="$WORK_DIR/stapler-staple.txt"
STAPLE_VALIDATION_LOG="$WORK_DIR/stapler-validate.txt"
GATEKEEPER_LOG="$WORK_DIR/gatekeeper-assessment.txt"

trap 'fail "Notarization was interrupted"' HUP INT TERM

# Strictly Verify the Existing Developer ID Signed Executable and Bundle
codesign \
    --verify \
    --strict \
    --verbose=4 \
    "$EXECUTABLE_PATH" ||
    fail "Main executable code signature verification failed"

codesign \
    --verify \
    --deep \
    --strict \
    --verbose=4 \
    "$APP_BUNDLE" ||
    fail "Application bundle code signature verification failed"

# Capture Existing Signature Metadata for Explicit Policy Validation
codesign \
    --display \
    --verbose=4 \
    "$APP_BUNDLE" \
    >/dev/null \
    2>"$SIGNATURE_DETAILS" ||
    fail "Could not read the application signature metadata"

grep -Fqx \
    "Identifier=$EXPECTED_BUNDLE_ID" \
    "$SIGNATURE_DETAILS" ||
    fail "Signed bundle identifier validation failed"

grep -Fqx \
    "Authority=$SIGNING_IDENTITY" \
    "$SIGNATURE_DETAILS" ||
    fail "Developer ID signing identity validation failed"

grep -Fqx \
    "TeamIdentifier=$EXPECTED_TEAM_ID" \
    "$SIGNATURE_DETAILS" ||
    fail "Apple Developer Team ID validation failed"

grep -Eq \
    '^Timestamp=' \
    "$SIGNATURE_DETAILS" ||
    fail "Secure signing timestamp was not found"

grep -Eq \
    '^Runtime Version=' \
    "$SIGNATURE_DETAILS" ||
    fail "Hardened Runtime metadata was not found"

# Refuse an Application That Already Has a Valid Stapled Ticket
if xcrun stapler validate \
    -v \
    "$APP_BUNDLE" \
    >"$PRE_STAPLE_LOG" \
    2>&1
then
    fail "Application already has a valid stapled notarization ticket"
fi

# Confirm the Stored notarytool Profile Can Authenticate Before Submission
xcrun notarytool history \
    --keychain-profile "$NOTARYTOOL_PROFILE" \
    --output-format json \
    >"$PROFILE_HISTORY" ||
    fail "The stored notarytool profile could not authenticate"

# Create an Ephemeral Submission Archive Without Modifying the Application
ditto \
    -c \
    -k \
    --sequesterRsrc \
    --keepParent \
    "$APP_BUNDLE" \
    "$SUBMISSION_ZIP" ||
    fail "Could not create the temporary notarization archive"

test -s "$SUBMISSION_ZIP" ||
    fail "Temporary notarization archive is empty"

printf 'Submitting %s for Apple notarization\n' "$APP_BUNDLE"

# Submit Privately to Apple and Wait for a Terminal Result
xcrun notarytool submit \
    "$SUBMISSION_ZIP" \
    --keychain-profile "$NOTARYTOOL_PROFILE" \
    --wait \
    --no-progress \
    --output-format json \
    >"$SUBMISSION_RESULT" \
    2>"$SUBMISSION_ERROR" ||
    fail "notarytool submission failed"

submission_id=$(
    plutil \
        -extract id \
        raw \
        -o - \
        "$SUBMISSION_RESULT" \
        2>/dev/null
) || fail "Could not read the notarization submission ID"

submission_status=$(
    plutil \
        -extract status \
        raw \
        -o - \
        "$SUBMISSION_RESULT" \
        2>/dev/null
) || fail "Could not read the notarization submission status"

test -n "$submission_id" ||
    fail "Notarization submission ID was empty"

test -n "$submission_status" ||
    fail "Notarization submission status was empty"

printf 'Notarization submission ID: %s\n' "$submission_id"
printf 'Notarization status: %s\n' "$submission_status"

# Retrieve the Apple Log for Accepted and Rejected Submissions
if [ "$submission_status" = "Accepted" ]; then
    xcrun notarytool log \
        "$submission_id" \
        --keychain-profile "$NOTARYTOOL_PROFILE" \
        "$NOTARIZATION_LOG" ||
        fail "Could not retrieve the accepted Apple notarization log"
else
    xcrun notarytool log \
        "$submission_id" \
        --keychain-profile "$NOTARYTOOL_PROFILE" \
        "$NOTARIZATION_LOG" ||
        true

    fail "Apple did not accept the notarization submission"
fi

# Staple the Accepted Ticket to the Original Application Bundle
xcrun stapler staple \
    -v \
    "$APP_BUNDLE" \
    >"$STAPLE_LOG" \
    2>&1 ||
    fail "Could not staple the notarization ticket"

# Validate the Ticket Attached to the Original Application Bundle
xcrun stapler validate \
    -v \
    "$APP_BUNDLE" \
    >"$STAPLE_VALIDATION_LOG" \
    2>&1 ||
    fail "Stapled notarization ticket validation failed"

# Reverify the Signature After Stapling
codesign \
    --verify \
    --deep \
    --strict \
    --verbose=4 \
    "$APP_BUNDLE" ||
    fail "Application signature verification failed after stapling"

# Require Gatekeeper to Accept the Signed and Stapled Application
spctl \
    --assess \
    --type execute \
    --verbose=4 \
    "$APP_BUNDLE" \
    >"$GATEKEEPER_LOG" \
    2>&1 ||
    fail "Gatekeeper rejected the signed and stapled application"

printf '%s\n' 'Stapled ticket validation:'
cat "$STAPLE_VALIDATION_LOG"

printf '%s\n' 'Gatekeeper assessment:'
cat "$GATEKEEPER_LOG"

# Remove Ephemeral Submission Material Only After Complete Success
rm -rf "$WORK_DIR"
WORK_DIR=

printf 'Notarized and stapled %s\n' "$APP_BUNDLE"
printf 'Notarization submission ID: %s\n' "$submission_id"
printf 'Notarytool profile: %s\n' "$NOTARYTOOL_PROFILE"
printf '%s\n' 'Stapled ticket validation passed'
printf '%s\n' 'Gatekeeper assessment passed'
