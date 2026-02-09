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
1. Save and load game functionality
2. Options menu

**********************************************
BUGS:
**********************************************
- Fix scaling from windowed to full screen on all resolutions
- Enemies sometimes walk backwards while still facing player
- Some wall textures appear flipped backwards
- Should not be able to hit ESC during death cam or episode end
