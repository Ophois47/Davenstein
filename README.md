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
1. End of episode success logic and victory animation
2. Save and load game functionality
3. Options menu

**********************************************
BUGS:
**********************************************
- Implement hit sounds and effects for rockets
- Chaingun pickup sound needs to take priority
