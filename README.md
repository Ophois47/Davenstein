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
2. Ceilings need to be respective colors per level
3. Bosses for episodes 2-6
4. High scores and associated menu screen
5. End of episode success logic and victory animation
6. Save and load game functionality
7. Options menu

**********************************************
BUGS:
**********************************************
- Officers + dogs sometimes do not play alert sound when reacting to player
- Enemy LOS regarding alert sound and walls
