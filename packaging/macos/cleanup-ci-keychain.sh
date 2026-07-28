#!/bin/sh
set -eu

#
# Davenstein - by David Petnick
#
# Removes Temporary macOS Signing Credentials and Restores the User Keychain
# Search List After Automated Developer ID Signing and Apple Notarization
#
# Temporary CI Keychain Cleanup Process:
#     - Validate GitHub Actions Temporary Paths
#     - Restore the Captured User Keychain Search List
#     - Remove the Disposable Keychain From the Active Search List
#     - Delete the Disposable macOS Keychain
#     - Delete the Decoded Developer ID PKCS12 Archive
#     - Delete Temporary Keychain State Files
#     - Report Any Cleanup Failure After Attempting Every Operation
#
# This Script Is Safe to Run After Partial Keychain Configuration Failure
# Release Automation Must Run This Script With if: always()
#
# Release Automation Must Provide:
#     RUNNER_TEMP                      GitHub Actions Temporary Directory
#
# Release Automation May Override:
#     MACOS_KEYCHAIN_FILE              Disposable macOS Keychain Path
#     MACOS_CERTIFICATE_FILE           Decoded PKCS12 Archive Path
#     MACOS_KEYCHAIN_LIST_FILE         Captured User Keychain Search List Path
#

# Required GitHub Actions Temporary Directory
RUNNER_TEMP=${RUNNER_TEMP:-}

fail() {
    printf 'Temporary macOS Keychain cleanup failed: %s\n' "$1" >&2
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

# Validate the Required GitHub Actions Temporary Directory
test -n "$RUNNER_TEMP" ||
    fail "RUNNER_TEMP must not be empty"

test -d "$RUNNER_TEMP" ||
    fail "RUNNER_TEMP does not exist"

test -w "$RUNNER_TEMP" ||
    fail "RUNNER_TEMP is not writable"

# Temporary Credential and Keychain State Paths
MACOS_KEYCHAIN_FILE=${MACOS_KEYCHAIN_FILE:-"$RUNNER_TEMP/davenstein-signing.keychain-db"}
MACOS_CERTIFICATE_FILE=${MACOS_CERTIFICATE_FILE:-"$RUNNER_TEMP/davenstein-signing.p12"}
MACOS_KEYCHAIN_LIST_FILE=${MACOS_KEYCHAIN_LIST_FILE:-"$RUNNER_TEMP/davenstein-original-keychains.txt"}
CURRENT_KEYCHAIN_LIST_FILE="$RUNNER_TEMP/davenstein-current-keychains.txt"

# Require Every Temporary Path to Remain Under RUNNER_TEMP
for temporary_path in \
    "$MACOS_KEYCHAIN_FILE" \
    "$MACOS_CERTIFICATE_FILE" \
    "$MACOS_KEYCHAIN_LIST_FILE" \
    "$CURRENT_KEYCHAIN_LIST_FILE"
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

# Validate Native macOS Keychain and File Utilities
for required_command in \
    rm \
    security \
    sed
do
    command -v "$required_command" >/dev/null 2>&1 ||
        fail "$required_command is required to clean up the temporary Keychain"
done

cleanup_failed=0

record_failure() {
    printf 'Temporary macOS Keychain cleanup warning: %s\n' "$1" >&2
    cleanup_failed=1
}

# Use the Captured Search List or Fall Back to the Current Search List
if [ -f "$MACOS_KEYCHAIN_LIST_FILE" ]; then
    restore_source="$MACOS_KEYCHAIN_LIST_FILE"
else
    if security list-keychains \
        -d user \
        > "$CURRENT_KEYCHAIN_LIST_FILE"
    then
        restore_source="$CURRENT_KEYCHAIN_LIST_FILE"
    else
        restore_source=
        record_failure "Could not capture the current user Keychain search list"
    fi
fi

# Restore the Search List Without the Disposable Keychain
if [ -n "$restore_source" ]; then
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
            ) || {
                record_failure "Could not canonicalize an existing Keychain path"
                keychain_canonical_path=
            }

            if [ -n "$keychain_canonical_path" ] &&
                [ "$keychain_canonical_path" != "$MACOS_KEYCHAIN_CANONICAL_PATH" ]
            then
                set -- "$@" "$keychain_path"
            fi
        fi
    done < "$restore_source"

    if [ "$#" -gt 0 ]; then
        security list-keychains \
            -d user \
            -s "$@" ||
            record_failure "Could not restore the user Keychain search list"
    else
        record_failure "No user Keychains were available to restore"
    fi
fi

# Confirm the Disposable Keychain Is No Longer in the Search List
if security list-keychains \
    -d user \
    > "$CURRENT_KEYCHAIN_LIST_FILE"
then
    disposable_keychain_present=0

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
            ) || {
                record_failure "Could not canonicalize a restored Keychain path"
                keychain_canonical_path=
            }

            if [ "$keychain_canonical_path" = "$MACOS_KEYCHAIN_CANONICAL_PATH" ]; then
                disposable_keychain_present=1
            fi
        fi
    done < "$CURRENT_KEYCHAIN_LIST_FILE"

    if [ "$disposable_keychain_present" -ne 0 ]; then
        record_failure "Disposable Keychain remains in the user search list"
    fi
else
    record_failure "Could not verify the restored user Keychain search list"
fi

# Delete the Disposable Keychain After Removing It From the Search List
if [ -e "$MACOS_KEYCHAIN_FILE" ]; then
    security delete-keychain \
        "$MACOS_KEYCHAIN_FILE" ||
        record_failure "Could not delete the disposable macOS Keychain"
fi

# Remove All Remaining Temporary Credential and State Files
rm -f \
    "$MACOS_KEYCHAIN_FILE" \
    "$MACOS_CERTIFICATE_FILE" \
    "$MACOS_KEYCHAIN_LIST_FILE" \
    "$CURRENT_KEYCHAIN_LIST_FILE" ||
    record_failure "Could not remove temporary credential files"

if [ "$cleanup_failed" -ne 0 ]; then
    fail "One or more cleanup operations failed"
fi

printf '%s\n' 'Removed temporary macOS signing credentials'
printf '%s\n' 'Restored the user Keychain search list'
