#!/bin/bash

echo 'Building Davenstein ...'
cargo update && cargo build --release
echo 'Building Assets ...'
cargo run --bin pak_builder --release -- --root assets --out target/release/assets.pak
echo 'Davenstein Built! Run `target/release/Davenstein`'
