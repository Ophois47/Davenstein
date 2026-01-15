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
2. E1M10 the secret level
3. Enemies for episodes 2-6 (mutant, officer)
4. Game difficulty levels
5. High scores and associated menu screen
6. End of episode success logic and victory animation
7. Save and load game functionality
8. Options menu

**********************************************
BUGS:
**********************************************
- Time and Par digits need to be aligned with right column
