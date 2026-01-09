*** Davenstein ***

This is an attempt to recreate Wolfenstein 3-D written entirely in Rust, using the Bevy engine.


--Build on Linux with: 
- $ cargo update && cargo build && cargo build --release


-- To Cross Compile for Armv7:
- $ cross build --release --target armv7-unknown-linux-gnueabihf

-- Then Run:
- $ cargo run --bin Davenstein

-- OR --
- $ ./target/release/Davenstein

**********************************************
FEATURES TO BE DONE:
**********************************************
- Main menus and pause game mechanic
- Associated screens and interactivity for menus
- E1M10 the secret level
- Episodes 2-6
- Enemies for episodes 2-6 (mutant, officer)
- BJ victory animation and logic at end of episodes

**********************************************
BUGS:
**********************************************
- End of level score screen requires scaling
- Enemies sometimes sidestep and walk backwards in a strange way
- Cannot pickup chaingun for ammo if it is already owned
- Hans needs to do damage in line with Wolfenstein 3-D
- HUD face sprites need to be just a little bigger
- HUD face sprite seems to look only in one direction
- Dog alert sound doesnt always play, something takes precedence
- Player can look around while 'Get Psyched' loading screen plays
