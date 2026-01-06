*** Davenstein ***

This is an attempt to recreate my own version of Wolfenstein 3D written entirely in Rust, using the Bevy engine.

Build on Linux with: $ cargo update && cargo build && cargo build --release

To Cross Compile for Armv7: $ cross build --release --target armv7-unknown-linux-gnueabihf

Then Run: $ cargo run --bin Davenstein

Alternatively: $ ./target/release/Davenstein

**********************************************
CURRENT TODO LIST:
**********************************************
- Pause game mechanic
- Boss Hans for E1M9 (In Progress with Other Model)
- Get enemy damage to player in line with original 1992 Wolfenstein
- Enemies should sentry and patrol
