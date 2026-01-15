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
3. Episodes 2-6
4. Enemies for episodes 2-6 (mutant, officer)
5. Game difficulty levels
6. High scores and associated menu screen
7. End of episode success logic and victory animation
8. Save and load game functionality
9. Options menu

**********************************************
BUGS:
**********************************************
- Enemies sometimes sidestep and walk backwards in a strange way
