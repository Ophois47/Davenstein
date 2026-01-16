*** Davenstein ***

This is an attempt to recreate Wolfenstein 3-D written entirely in Rust, using the Bevy engine.

NOTE! Left Control (LCtrl) releases the mouse from the window.


-- Build on Linux with: 
- $ cargo update && cargo build && cargo build --release


-- To Cross Compile for Armv7:
- $ cross build --release --target armv7-unknown-linux-gnueabihf

-- Then Run:
- $ cargo run --bin Davenstein

-- OR --
- $ ./target/release/Davenstein

**********************************************
FEATURES TO BE COMPLETED:
**********************************************
1. E1M10 and the secret levels
2. Game difficulty levels
3. Ceilings need to be respective colors per level
4. Bosses for episodes 2-6
5. High scores and associated menu screen
6. End of episode success logic and victory animation
7. Save and load game functionality
8. Options menu

**********************************************
BUGS:
**********************************************
- Enemies can walk through blocking decorations
- Officers and mutants do not drop ammo
