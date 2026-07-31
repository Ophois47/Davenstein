/*
Davenstein - by David Petnick
*/

pub(crate) mod cheat_message;
mod hud;
pub(crate) mod level_end_font;
mod splash;
mod state;
mod touch_overlay;
pub mod sync;

use bevy::prelude::*;

pub use state::DamageFlash;
pub use state::DeathOverlay;
pub use state::GameOver;
pub use state::HudState;
pub use state::PickupFlash;

pub use splash::SplashStep;

// Re-Export Episode End UI Assets so Gameplay Modules can use
// Them Without Making Splash Module Public
pub(crate) use splash::EpisodeEndImages;

pub(crate) use hud::HudFaceOverride;

pub struct UiPlugin;

impl Plugin for UiPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<cheat_message::CheatMessageState>()
			.init_resource::<HudState>()
			.init_resource::<DamageFlash>()
			.init_resource::<PickupFlash>()
			.init_resource::<DeathOverlay>()
			.init_resource::<GameOver>()
			.init_resource::<sync::DeathDelay>()
			.init_resource::<sync::RestartRequested>()
			.init_resource::<sync::NewGameRequested>()
			.init_resource::<hud::HudFacePrevHp>()
			.init_resource::<hud::HudFaceLook>()
			.init_resource::<hud::WeaponState>()
			.add_plugins(splash::SplashPlugin)
			.add_systems(Startup, hud::setup_hud)
			.add_systems(Startup, splash::setup_splash)
			// Keep Window-Space UI (Menus, Splash, Intermission, Overlays) on the
			// Persistent Menu Camera so It Never Falls Into the Low-Res World Canvas.
			//
			// Runs in PostUpdate Before UI Layout - NOT Update - so a Root Spawned
			// This Frame Is Retargeted Before It Is Laid Out. In Update It Raced the
			// Menu-Spawn Systems: a Menu Spawned After This Ran Stayed Untargeted for
			// One Frame, Rendered on the Default (Canvas) Camera, Then Snapped to the
			// Menu Camera Next Frame - Seen as a Flicker When Opening Menus or
			// Rebuilding an Option Row. PostUpdate Sees All Update Spawns (Commands
			// Are Flushed at the Schedule Boundary) and '.before(UiSystems::Layout)'
			// Guarantees the Target Is Set Before Layout Reads It
			.add_systems(
				PostUpdate,
				hud::route_window_ui_to_menu_camera.before(bevy::ui::UiSystems::Layout),
			)
			// Core State / Sync Systems
			.add_systems(Update, sync::apply_enemy_fire_to_player_vitals)
			.add_systems(Update, sync::sync_player_hp_with_hud)
			.add_systems(Update, sync::handle_player_death_once)
			.add_systems(Update, sync::tick_death_delay_and_request_restart)
			// After InputGather so the Game Over Screen Reacts to This Frame's
			// Confirm (Gamepad South, a Touch Tap) Rather Than Last Frame's
			.add_systems(Update, sync::game_over_input.after(davelib::input::InputGather))
			// HUD + Viewmodel Systems
			.add_systems(Update, hud::sync_hud_layout_on_window_change)
			.add_systems(Update, hud::sync_mission_overlay_layout_on_window_change)
			.add_systems(Update, hud::sync_viewmodel_size)
			// Order After the Get-Psyched Loading Tick so the Frame It Clears the
			// Control Lock (Loading Finished) the Viewmodel Is Restored the *Same*
			// Frame the Teal Screen Is Removed - No One-Frame Gap Where the World Is
			// Visible but the Weapon Has Not Come Up Yet
			.add_systems(
				Update,
				hud::sync_viewmodel_visibility.after(splash::SplashUpdateSet::PsychedLoading),
			)
			// The Original's MIL Cheat Warning: a Modal Grey Box That Freezes Play
			// Until Any Input. No Ordering Constraint is Needed Against the Splash
			// Machine: its Gameplay Branch Bails While the Control Lock is Held, so
			// the Pause Menu Cannot Open Underneath This Box in Any System Order
			.add_systems(Update, cheat_message::trigger_cheat_message)
			.add_systems(Update, cheat_message::dismiss_cheat_message)
			// On-Screen Touch Controls. The Mode Sync Runs Before InputGather so
			// the Touch Source Tests This Frame's Fingers Against the Control Set
			// the Player Is Looking At; the Drawing Systems Run After It so the
			// Overlay Reflects This Frame's Assignments (Stick Under the Thumb,
			// Fire Lit While Held) Without a Frame of Lag
			.add_systems(
				Update,
				touch_overlay::sync_touch_ui_mode.before(davelib::input::InputGather),
			)
			.add_systems(
				Update,
				(
					touch_overlay::sync_touch_overlay_tree,
					touch_overlay::sync_touch_overlay_visibility,
					touch_overlay::sync_touch_button_feedback,
					touch_overlay::sync_touch_stick_visual,
				)
					.after(davelib::input::InputGather),
			)
			.add_systems(Update, hud::weapon_fire_and_viewmodel)
			.add_systems(Update, hud::sync_hud_hp_digits)
			.add_systems(Update, hud::sync_hud_ammo_digits)
			.add_systems(Update, hud::sync_hud_score_digits)
			.add_systems(Update, hud::sync_hud_lives_digits)
			.add_systems(Update, hud::sync_hud_floor_digits)
			.add_systems(Update, hud::sync_hud_icons)
			.add_systems(Update, hud::tick_hud_face_timers)
			.add_systems(Update, hud::sync_hud_face)
			// View Size Border (Classic Wolf3D Teal Border)
			.add_systems(Update, hud::sync_view_size_border)
			// Overlay Systems
			.add_systems(Update, hud::flash_on_hp_drop)
			.add_systems(Update, hud::ensure_pickup_flash_overlay)
			.add_systems(Update, hud::tick_pickup_flash)
			.add_systems(Update, hud::tick_damage_flash)
			.add_systems(Update, hud::tick_death_overlay)
			.add_systems(Update, hud::sync_game_over_overlay_visibility)
			.add_systems(Update, level_end_font::sync_level_end_bitmap_text)
			.add_systems(Update, hud::tick_mission_bj_card);
	}
}
