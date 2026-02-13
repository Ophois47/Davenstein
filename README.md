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

-- To Build or Rebuild the Assets Pak
- $ cargo run --bin pak_builder --release -- --root assets --out dist/assets.pak

-- To Build or Rebuild Assets Pak in Release Dir
- $ cargo run --bin pak_builder --release -- --root assets --out target/release/assets.pak

**********************************************
FEATURES TO BE COMPLETED:
**********************************************
1. Save and load game functionality

**********************************************
BUGS:
**********************************************
- Change window size does not respect menu UI
- Cursor will sometimes become offset when switching display modes
- Options get set upon going into and then leaving options menu
- Other window resolutions do not show up outside of native resolution 
