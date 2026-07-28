#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Configures a Temporary macOS Keychain for Automated Developer ID Signing
# and Apple Notarization
#
# Temporary CI Keychain Configuration Process:
#     - Validate Required GitHub Actions Paths and Protected Credentials
#     - Capture the Existing User Keychain Search List
#     - Decode and Validate the Developer ID PKCS12 Archive
#     - Create and Unlock a Disposable macOS Keychain
#     - Prepend the Disposable Keychain to the User Search List
#     - Import the Developer ID Certificate and Private Key
#     - Grant Apple Signing Tools Access to the Imported Private Key
#     - Confirm the Expected Developer ID Signing Identity
#     - Store and Validate a notarytool Profile in the Disposable Keychain
#     - Export Temporary Paths and Profile Metadata for Later Workflow Steps
#
# This Script Does Not Build, Sign, Notarize, Archive, or Upload the Application
# The Matching Cleanup Script Must Run With if: always() After Configuration
#
# Release Automation Must Provide:
#     MACOS_CERTIFICATE_P12_BASE64     Base64 Encoded Developer ID PKCS12 Archive
#     MACOS_CERTIFICATE_PASSWORD       Password Protecting the PKCS12 Archive
#     APPLE_NOTARIZATION_ID            Apple Account Used for Notarization
#     APPLE_NOTARIZATION_PASSWORD      Apple App-Specific Password
#     APPLE_TEAM_ID                    Apple Developer Team Identifier
#     RUNNER_TEMP                      GitHub Actions Temporary Directory
#     GITHUB_ENV                       GitHub Actions Environment Output File
#
# Release Automation May Override:
#     EXPECTED_TEAM_ID                 Expected Apple Developer Team Identifier
#     SIGNING_IDENTITY                 Expected Developer ID Application Identity
#     NOTARYTOOL_PROFILE               Temporary notarytool Keychain Profile
#     MACOS_KEYCHAIN_FILE              Disposable macOS Keychain Path
#     MACOS_CERTIFICATE_FILE           Decoded PKCS12 Archive Path
#     MACOS_KEYCHAIN_LIST_FILE         Captured User Keychain Search List Path
#

# Required GitHub Actions Paths and Protected Credentials
RUNNER_TEMP=${RUNNER_TEMP:-}
GITHUB_ENV=${GITHUB_ENV:-}
MACOS_CERTIFICATE_P12_BASE64=${MACOS_CERTIFICATE_P12_BASE64:-}
MACOS_CERTIFICATE_PASSWORD=${MACOS_CERTIFICATE_PASSWORD:-}
APPLE_NOTARIZATION_ID=${APPLE_NOTARIZATION_ID:-}
APPLE_NOTARIZATION_PASSWORD=${APPLE_NOTARIZATION_PASSWORD:-}
APPLE_TEAM_ID=${APPLE_TEAM_ID:-}

# Expected Developer Identity and Temporary notarytool Profile
EXPECTED_TEAM_ID=${EXPECTED_TEAM_ID:-"923379G559"}
SIGNING_IDENTITY=${SIGNING_IDENTITY:-"Developer ID Application: CyberSoft Operating Corporation (923379G559)"}
NOTARYTOOL_PROFILE=${NOTARYTOOL_PROFILE:-"DavensteinCI"}

fail() {
    printf 'Temporary macOS Keychain configuration failed: %s\n' "$1" >&2
    exit 1
}

canonicalize_path() {
    path_value=$1
    path_directory=${path_value%/*}
    path_name=${path_value##*/}

    physical_directory=$(
        CDPATH= cd -- "$path_directory" 2>/dev/null &&
            pwd -P
    ) || return 1

    printf '%s/%s\n' "$physical_directory" "$path_name"
}

# Validate Every Required GitHub Actions Value Before Creating Credentials
test -n "$RUNNER_TEMP" ||
    fail "RUNNER_TEMP must not be empty"

test -n "$GITHUB_ENV" ||
    fail "GITHUB_ENV must not be empty"

test -n "$MACOS_CERTIFICATE_P12_BASE64" ||
    fail "MACOS_CERTIFICATE_P12_BASE64 must not be empty"

test -n "$MACOS_CERTIFICATE_PASSWORD" ||
    fail "MACOS_CERTIFICATE_PASSWORD must not be empty"

test -n "$APPLE_NOTARIZATION_ID" ||
    fail "APPLE_NOTARIZATION_ID must not be empty"

test -n "$APPLE_NOTARIZATION_PASSWORD" ||
    fail "APPLE_NOTARIZATION_PASSWORD must not be empty"

test -n "$APPLE_TEAM_ID" ||
    fail "APPLE_TEAM_ID must not be empty"

test -n "$EXPECTED_TEAM_ID" ||
    fail "EXPECTED_TEAM_ID must not be empty"

test -n "$SIGNING_IDENTITY" ||
    fail "SIGNING_IDENTITY must not be empty"

test -n "$NOTARYTOOL_PROFILE" ||
    fail "NOTARYTOOL_PROFILE must not be empty"

test "$APPLE_TEAM_ID" = "$EXPECTED_TEAM_ID" ||
    fail "APPLE_TEAM_ID does not match EXPECTED_TEAM_ID"

# Temporary Credential and Keychain State Paths
MACOS_KEYCHAIN_FILE=${MACOS_KEYCHAIN_FILE:-"$RUNNER_TEMP/davenstein-signing.keychain-db"}
MACOS_CERTIFICATE_FILE=${MACOS_CERTIFICATE_FILE:-"$RUNNER_TEMP/davenstein-signing.p12"}
MACOS_KEYCHAIN_LIST_FILE=${MACOS_KEYCHAIN_LIST_FILE:-"$RUNNER_TEMP/davenstein-original-keychains.txt"}

# Require Every Temporary Output to Remain Under RUNNER_TEMP
for temporary_path in \
    "$MACOS_KEYCHAIN_FILE" \
    "$MACOS_CERTIFICATE_FILE" \
    "$MACOS_KEYCHAIN_LIST_FILE"
do
    case "$temporary_path" in
        "$RUNNER_TEMP"/*)
            ;;
        *)
            fail "Temporary credential path is outside RUNNER_TEMP"
            ;;
    esac
done

# Resolve the Physical Disposable Keychain Path for Reliable Comparison
MACOS_KEYCHAIN_CANONICAL_PATH=$(
    canonicalize_path "$MACOS_KEYCHAIN_FILE"
) || fail "Could not canonicalize the temporary Keychain path"

# Validate Native macOS Credential and Notarization Utilities
for required_command in \
    base64 \
    chmod \
    grep \
    openssl \
    rm \
    security \
    sed \
    xcrun
do
    command -v "$required_command" >/dev/null 2>&1 ||
        fail "$required_command is required to configure the temporary Keychain"
done

test -w "$GITHUB_ENV" ||
    fail "GITHUB_ENV is not writable"

# Remove Any Stale Credential Material From an Earlier Partial Attempt
security delete-keychain "$MACOS_KEYCHAIN_FILE" >/dev/null 2>&1 ||
    true

rm -f \
    "$MACOS_KEYCHAIN_FILE" \
    "$MACOS_CERTIFICATE_FILE" \
    "$MACOS_KEYCHAIN_LIST_FILE"

umask 077

# Capture the Original User Keychain Search List Before Modifying It
security list-keychains \
    -d user \
    > "$MACOS_KEYCHAIN_LIST_FILE" ||
    fail "Could not capture the original user Keychain search list"

chmod 600 "$MACOS_KEYCHAIN_LIST_FILE"

# Decode and Validate the Protected Developer ID PKCS12 Archive
printf '%s' "$MACOS_CERTIFICATE_P12_BASE64" |
    base64 -D \
        > "$MACOS_CERTIFICATE_FILE" ||
    fail "Could not decode the Developer ID PKCS12 archive"

test -s "$MACOS_CERTIFICATE_FILE" ||
    fail "Decoded Developer ID PKCS12 archive is empty"

chmod 600 "$MACOS_CERTIFICATE_FILE"

openssl pkcs12 \
    -in "$MACOS_CERTIFICATE_FILE" \
    -passin env:MACOS_CERTIFICATE_PASSWORD \
    -noout ||
    fail "Developer ID PKCS12 archive validation failed"

# Generate a Unique Password for the Disposable Keychain
keychain_password=$(
    openssl rand -hex 32
) || fail "Could not generate the temporary Keychain password"

printf '::add-mask::%s\n' "$keychain_password"

# Create and Unlock the Disposable macOS Keychain
security create-keychain \
    -p "$keychain_password" \
    "$MACOS_KEYCHAIN_FILE" ||
    fail "Could not create the temporary macOS Keychain"

security set-keychain-settings \
    "$MACOS_KEYCHAIN_FILE" ||
    fail "Could not configure the temporary macOS Keychain"

security unlock-keychain \
    -p "$keychain_password" \
    "$MACOS_KEYCHAIN_FILE" ||
    fail "Could not unlock the temporary macOS Keychain"

# Reconstruct the Existing Search List and Prepend the Disposable Keychain
set --

while IFS= read -r keychain_entry
do
    keychain_path=$(
        printf '%s\n' "$keychain_entry" |
            sed -E \
                's/^[[:space:]]*"//; s/"[[:space:]]*$//'
    )

    if [ -n "$keychain_path" ]; then
        keychain_canonical_path=$(
            canonicalize_path "$keychain_path"
        ) || fail "Could not canonicalize an existing Keychain path"

        if [ "$keychain_canonical_path" != "$MACOS_KEYCHAIN_CANONICAL_PATH" ]; then
            set -- "$@" "$keychain_path"
        fi
    fi
done < "$MACOS_KEYCHAIN_LIST_FILE"

security list-keychains \
    -d user \
    -s "$MACOS_KEYCHAIN_FILE" "$@" ||
    fail "Could not update the user Keychain search list"

# Import the Developer ID Certificate and Private Key Without Prompts
security import "$MACOS_CERTIFICATE_FILE" \
    -k "$MACOS_KEYCHAIN_FILE" \
    -f pkcs12 \
    -P "$MACOS_CERTIFICATE_PASSWORD" \
    -T /usr/bin/codesign \
    -T /usr/bin/security ||
    fail "Could not import the Developer ID signing identity"

# Grant Apple Signing Tools Access to the Imported Private Key
security set-key-partition-list \
    -S apple-tool:,apple:,codesign: \
    -s \
    -k "$keychain_password" \
    "$MACOS_KEYCHAIN_FILE" ||
    fail "Could not configure Developer ID private key access"

# Confirm the Exact CyberSoft Developer ID Identity Was Imported
security find-identity \
    -v \
    -p codesigning \
    "$MACOS_KEYCHAIN_FILE" |
    grep -F -- "\"$SIGNING_IDENTITY\"" >/dev/null ||
    fail "Expected Developer ID signing identity was not imported"

# Store and Validate Notarization Credentials in the Disposable Keychain
xcrun notarytool store-credentials \
    "$NOTARYTOOL_PROFILE" \
    --apple-id "$APPLE_NOTARIZATION_ID" \
    --password "$APPLE_NOTARIZATION_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --validate \
    --keychain "$MACOS_KEYCHAIN_FILE" ||
    fail "Could not store and validate the temporary notarytool profile"

# Export Temporary Paths and Profile Metadata for Later Workflow Steps
{
    printf 'MACOS_KEYCHAIN_FILE=%s\n' "$MACOS_KEYCHAIN_FILE"
    printf 'MACOS_CERTIFICATE_FILE=%s\n' "$MACOS_CERTIFICATE_FILE"
    printf 'MACOS_KEYCHAIN_LIST_FILE=%s\n' "$MACOS_KEYCHAIN_LIST_FILE"
    printf 'NOTARYTOOL_PROFILE=%s\n' "$NOTARYTOOL_PROFILE"
    printf 'NOTARYTOOL_KEYCHAIN=%s\n' "$MACOS_KEYCHAIN_FILE"
} >> "$GITHUB_ENV"

printf 'Configured temporary macOS signing Keychain at %s\n' \
    "$MACOS_KEYCHAIN_FILE"
printf 'Imported signing identity: %s\n' "$SIGNING_IDENTITY"
printf 'Stored notarytool profile: %s\n' "$NOTARYTOOL_PROFILE"
