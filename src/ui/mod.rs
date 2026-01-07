/*
Davenstein - by David Petnick
*/
mod state;
mod hud;
mod splash;
pub mod sync;

use bevy::prelude::*;

pub use state::HudState;
pub use state::DamageFlash;
pub use state::PickupFlash;
pub use state::DeathOverlay;
pub use state::GameOver;

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
            .add_systems(Startup, (hud::setup_hud, splash::setup_splash).chain())
            .add_systems(
                Update,
                (
                    // Core state/sync (keep strict ordering)
                    (
                        sync::apply_enemy_fire_to_player_vitals,
                        sync::sync_player_hp_with_hud,
                        sync::handle_player_death_once,
                        sync::tick_death_delay_and_request_restart,
                        sync::game_over_input,
                    )
                        .chain(),
                    // HUD + viewmodel (strict ordering)
                    (
                        hud::sync_viewmodel_size,
                        hud::weapon_fire_and_viewmodel,
                        hud::sync_hud_hp_digits,
                        hud::sync_hud_ammo_digits,
                        hud::sync_hud_score_digits,
                        hud::sync_hud_lives_digits,
                        hud::sync_hud_floor_digits,
                        hud::sync_hud_icons,
                        hud::tick_hud_face_timers,
                        hud::sync_hud_face,
                    )
                        .chain(),
                    // Overlays
                    (
                        hud::flash_on_hp_drop,
                        hud::ensure_pickup_flash_overlay,
                        hud::tick_pickup_flash,
                        hud::tick_damage_flash,
                        hud::tick_death_overlay,
                        hud::sync_game_over_overlay_visibility,
                        hud::tick_mission_bj_card,
                    )
                        .chain(),
                )
                    .chain(),
            );
    }
}
