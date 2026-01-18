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
1. Bosses for episodes 2, 3, 4 and 6
2. High scores and associated menu screen
3. End of episode success logic and victory animation
4. Save and load game functionality
5. Options menu

**********************************************
BUGS:
**********************************************
- Skill levels are not spawning enemies correctly
- Skill level screen should default to Bring it On
- Move face sprite on HUD up 1px
