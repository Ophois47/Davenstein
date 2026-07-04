Write-Host "Building Davenstein ..."
cargo update
cargo build --release

Write-Host "Building Assets ..."
cargo run --bin pak_builder --release -- --root assets --out target/release/assets.pak

Write-Host "Davenstein Built! Run `target/release/Davenstein`"
