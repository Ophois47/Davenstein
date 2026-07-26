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

// Marker on the PLAYER While a Scripted End-of-Episode Camera Owns the View
//
// Both End Sequences - the Boss Death Cam and the BJ Victory Camera - Drive the
// Player's Transform and LookAngles Themselves. Any System That Would Otherwise
// Reassert the Player's Own View Preferences Has to Stand Down While This Is
// Present, so a Cutscene Frames the Shot It Was Authored to Frame Regardless of How
// the Player Has Their Controls Configured. Concretely It Stops
// 'level_pitch_without_mouselook' From Flattening the Death Cam's Deliberate
// Downward Tilt to the Horizon on Every Frame Whenever Mouselook Is Switched Off
//
// It Lives in davelib Rather Than Beside the Cutscenes Because the Systems That Must
// Respect It Live Here While the Cutscenes That Set It Live in the Binary Crate,
// Which Is Exactly the Split This Module Exists to Serve
#[derive(Component)]
pub struct ScriptedCamera;

// Data for End of Episode Flow
#[derive(Resource, Clone, Copy)]
pub struct EpisodeEndResult {
	pub episode: u8,
	pub score: u32,
}
