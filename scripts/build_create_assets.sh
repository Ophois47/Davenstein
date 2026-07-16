#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "##=>> Building Davenstein ..."
cargo build --release

echo
echo "##=>> Building assets.pak ..."
cargo run --release --bin pak_builder -- \
    --root assets \
    --out target/release/assets.pak

echo
echo "##=>> Davenstein Has Been Built Successfully!"
echo "##=> Run: target/release/Davenstein"
