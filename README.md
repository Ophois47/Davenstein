*** Davenstein ***

This is an attempt to recreate Wolfenstein 3-D written entirely in Rust, using the Bevy engine.


--Build on Linux with: 
- $ cargo update && cargo build && cargo build --release


-- To Cross Compile for Armv7:
- $ cross build --release --target armv7-unknown-linux-gnueabihf

-- Then Run:
- $ cargo run --bin Davenstein

-- OR --
- $ ./target/release/Davenstein

**********************************************
FEATURES TO BE DONE:
**********************************************
- Main menu and pause game mechanic
- Associated screens and interactivity for menus
- Episodes 2-6
- Enemies for episodes 2-6 (mutant, officer)

**********************************************
BUGS:
**********************************************
- Enemies sidestep in a strange way
- Cannot pickup chaingun for ammo if already owned
- Hans needs to do damage in line with Wolfenstein 3-D
- HUD face sprites need to be just a little bigger
