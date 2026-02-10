/*
Davenstein - by David Petnick
*/
// Episode End Flow Lives in BIN Crate (src/episode_end.rs) on Purpose
// - Needs Access to BIN Only Modules (crate::ui, HUD State + Splash Flow)
// - Davelib Crate is Reusable Gameplay Library, Should Not Depend on BIN UI Wiring
// - Keeping Only Shared Marker / Types in davelib::episode_end Avoids Circular
//	Dependencies + "Unreachable" Symbols While Letting Enemies Tag Bosses (DeathCamBoss)
//	from Inside Library
use bevy::prelude::*;

// Marker on Bosses Whose Death 
// Should Trigger Death Cam Replay
#[derive(Component)]
pub struct DeathCamBoss;

// Data for End of Episode Flow
#[derive(Resource, Clone, Copy)]
pub struct EpisodeEndResult {
	pub episode: u8,
	pub score: u32,
}
