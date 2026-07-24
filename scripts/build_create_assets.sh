#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "##=>> Building Davenstein ..."
# --locked Forbids Cargo From Rewriting Cargo.lock During a Build, so Routine
# Builds Are Reproducible and Never Dirty the Lockfile (Which Is What Causes the
# Cross-Machine "cannot pull" Collisions). To Change Dependency Versions, Run
# `cargo update` Deliberately, Commit the New Cargo.lock, Then Pull Everywhere.
# This Matches the --locked Builds CI and the Flatpak Manifest Already Use
cargo build --release --locked

echo
echo "##=>> Building assets.pak ..."
cargo run --release --locked --bin pak_builder -- \
    --root assets \
    --out target/release/assets.pak

echo
echo "##=>> Davenstein Has Been Built Successfully!"
echo "##=> Run: target/release/Davenstein"
