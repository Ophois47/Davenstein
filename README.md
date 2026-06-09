*** Davenstein ***

This is an attempt to recreate Wolfenstein 3-D written entirely in Rust, using the Bevy engine.

NOTE! Left Control (LCtrl) releases the mouse from the window.

-- Build on Linux with: 
- $ cargo update && cargo build --release

-- Cross Compiling requires a container engine (Podman or Docker) and the `cross` tool:
- $ cargo install cross --git https://github.com/cross-rs/cross

-- On Fedora (or any Podman host), tell cross to use Podman:
- $ export CROSS_CONTAINER_ENGINE=podman

-- To Cross Compile for Windows (GNU):
- $ cross build --release --target x86_64-pc-windows-gnu --target-dir target/win

-- To Cross Compile for Linux Armv7 (GNU):
- $ cross build --release --target armv7-unknown-linux-gnueabihf --target-dir target/arm

-- To Build or Rebuild the Assets Pak
- $ cargo run --bin pak_builder --release -- --root assets --out dist/assets.pak
-- To Build or Rebuild Assets Pak in Release Dir
- $ cargo run --bin pak_builder --release -- --root assets --out target/release/assets.pak


**********************************************
FEATURES TO BE COMPLETED:
**********************************************

**********************************************
BUGS:
**********************************************
- Change view size does not respect menu UI