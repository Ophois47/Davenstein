/*
Davenstein - by David Petnick
*/
// Episode-end flow lives in the BIN crate (src/episode_end.rs) on purpose
// - It needs access to bin-only modules like crate::ui (HUD state + splash flow)
// - The davelib crate is a reusable gameplay library and should not depend on the bin UI wiring
// - Keeping only shared marker/types in davelib::episode_end avoids circular dependencies and "unreachable" symbols
//   while still letting enemies tag bosses (DeathCamBoss) from inside the library
use bevy::prelude::*;

// Marker on bosses whose death should trigger the Death Cam replay
#[derive(Component)]
pub struct DeathCamBoss;

// Data for end of episode flow
#[derive(Resource, Clone, Copy)]
pub struct EpisodeEndResult {
	pub episode: u8,
	pub score: u32,
}
