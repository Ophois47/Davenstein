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
1. Implement the last enemy boss, General Fettgesicht
2. End of episode success logic and victory animation
3. Save and load game functionality
4. Options menu

**********************************************
BUGS:
**********************************************
- Implement chaingun attack for General Fettgesicht
- Implement hit sounds and effects for rockets
- Implement smoke trail effects for rockets
- Several rocket directions seem mixed up
- Bosses need to scale thier health values per skill level
- The width of Otto's corpse is a little squished
- When two of the same enemies fire their weapons at the same time
		there is a conflict, we get:

	"FetchError>>>::with_entity::{{closure}}: Entity despawned: The entity with ID 1378v14 is invalid; its index now has generation 15. Note that interacting with a despawned entity is the most common cause of this error but there are others. If you were attempting to apply a command to this entity, and want to handle this error gracefully, consider using `EntityCommands::queue_handled` or `queue_silenced`."
