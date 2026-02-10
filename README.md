*** Davenstein ***

This is an attempt to recreate Wolfenstein 3-D written entirely in Rust, using the Bevy engine.

NOTE! Left Control (LCtrl) releases the mouse from the window.


-- Build on Linux with: 
- $ cargo update && cargo build --release

-- To Cross Compile for Windows (GNU):
- $ cross build --release --target x86_64-pc-windows-gnu

-- To Cross Compile for Linux Armv7 (GNU):
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
- Currently spawning a PointLight with shadows_enabled: true in world::setup
- Enemy sprite materials use AlphaMode::Blend
