*** Davenstein ***

This is an attempt to recreate my own version of Wolfenstein 3D written entirely in Rust, using the Bevy engine.

Build on Linux with: $ cargo update && cargo build && cargo build --release

To Cross Compile for Armv7: $ cross build --release --target armv7-unknown-linux-gnueabihf

Then Run: $ cargo run --bin Davenstein

Alternatively: $ ./target/release/Davenstein

**********************************************
CURRENT TODO LIST:
**********************************************
- Main menu and pause game mechanic
- End level score and stats tally
- Associated screens and interactivity for menus
- Episodes 2-6
- Enemies for episodes 2-6 (mutant, officer)
