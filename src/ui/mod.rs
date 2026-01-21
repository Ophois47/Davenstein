/*
Davenstein - by David Petnick
*/
mod hud;
pub(crate) mod level_end_font;
mod splash;
mod state;
pub mod sync;

use bevy::prelude::*;

pub use state::DamageFlash;
pub use state::DeathOverlay;
pub use state::GameOver;
pub use state::HudState;
pub use state::PickupFlash;
pub use splash::SplashStep;

pub(crate) use hud::HudFaceOverride;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
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
            // Core State / Sync systems
            .add_systems(Update, sync::apply_enemy_fire_to_player_vitals)
            .add_systems(Update, sync::sync_player_hp_with_hud)
            .add_systems(Update, sync::handle_player_death_once)
            .add_systems(Update, sync::tick_death_delay_and_request_restart)
            .add_systems(Update, sync::game_over_input)
            // HUD + Viewmodel systems
            .add_systems(Update, hud::sync_viewmodel_size)
            .add_systems(Update, hud::weapon_fire_and_viewmodel)
            .add_systems(Update, hud::sync_hud_hp_digits)
            .add_systems(Update, hud::sync_hud_ammo_digits)
            .add_systems(Update, hud::sync_hud_score_digits)
            .add_systems(Update, hud::sync_hud_lives_digits)
            .add_systems(Update, hud::sync_hud_floor_digits)
            .add_systems(Update, hud::sync_hud_icons)
            .add_systems(Update, hud::tick_hud_face_timers)
            .add_systems(Update, hud::sync_hud_face)
            // Overlay systems
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
