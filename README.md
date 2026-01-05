*** Davenstein ***

This is an attempt to recreate my own version of Wolfenstein 3D written entirely in Rust, using the Bevy engine.

Build on Linux with: $ cargo update && cargo build && cargo build --release

To Cross Compile for Armv7: $ cross build --release --target armv7-unknown-linux-gnueabihf

Then Run: $ cargo run --bin Davenstein

Alternatively: $ ./target/release/Davenstein

**********************************************
CURRENT TODO LIST:
**********************************************
- Boss Hans for E1M9
- Doors should not close when corpses are in the way
- Guns we already have, we should still pick up and simply get ammo for
- Get enemy damage to player in line with original 1992 Wolfenstein
- Enemies shouldnt be able to fire when getting shot
- Enemies should sentry and patrol
- Pause game mechanic
- Proper main menu
- God mode mechanic (debug)
