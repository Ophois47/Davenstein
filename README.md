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
1. Ceilings need to be respective colors per level
2. Bosses for episodes 2-6
3. High scores and associated menu screen
4. End of episode success logic and victory animation
5. Save and load game functionality
6. Options menu

**********************************************
BUGS:
**********************************************
