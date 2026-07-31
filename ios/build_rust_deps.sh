#!/usr/bin/env bash

set -euo pipefail

# Xcode Prepends its Toolchain Directory and Can Cause Rust to Select the Wrong Linker
# Restore the Normal System Paths While Keeping Cargo and Homebrew Available
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

script_directory="$(
    cd "$(dirname "${BASH_SOURCE[0]}")"
    pwd
)"

repository_root="${DAVENSTEIN_REPO_ROOT:-$script_directory/..}"

test -f "$repository_root/Cargo.toml" || {
    echo "Davenstein repository root was not found: $repository_root" >&2
    exit 1
}

cd "$repository_root"

profile="debug"
cargo_profile=()

if [[ "$CONFIGURATION" != "Debug" ]]; then
    profile="release"
    cargo_profile=(--release)
fi

export CARGO_TARGET_DIR="$DERIVED_FILE_DIR/cargo"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

executables=()

for arch in $ARCHS; do
    case "$arch" in
        arm64)
            if [[ "${LLVM_TARGET_TRIPLE_SUFFIX-}" = "-simulator" ]]; then
                rust_target="aarch64-apple-ios-sim"
            else
                rust_target="aarch64-apple-ios"
            fi
            ;;
        x86_64)
            if [[ "${LLVM_TARGET_TRIPLE_SUFFIX-}" != "-simulator" ]]; then
                echo "x86_64 is only supported for an iOS Simulator build" >&2
                exit 1
            fi

            rust_target="x86_64-apple-ios"
            ;;
        *)
            echo "Unsupported Xcode architecture: $arch" >&2
            exit 1
            ;;
    esac

    cargo build \
        "${cargo_profile[@]}" \
        --target "$rust_target" \
        --bin Davenstein

    executables+=(
        "$CARGO_TARGET_DIR/$rust_target/$profile/Davenstein"
    )
done

mkdir -p "$(dirname "$TARGET_BUILD_DIR/$EXECUTABLE_PATH")"

lipo \
    -create \
    -output "$TARGET_BUILD_DIR/$EXECUTABLE_PATH" \
    "${executables[@]}"
