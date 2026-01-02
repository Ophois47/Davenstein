/*
Davenstein - by David Petnick
*/
mod state;
mod hud;
pub mod sync;

use bevy::prelude::*;

pub use state::HudState;
pub use state::DamageFlash;
pub use state::DeathOverlay;
pub use state::GameOver;
pub(crate) use hud::HudFaceOverride;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // -----------------------------
        // Core UI State / Resources
        // -----------------------------
        app.init_resource::<HudState>();
        app.init_resource::<GameOver>();
        app.init_resource::<DeathOverlay>();

        app.init_resource::<DamageFlash>();
        app.init_resource::<hud::WeaponState>();

        // -----------------------------
        // Face System State
        // -----------------------------
        app.init_resource::<hud::HudFaceLook>();
        app.init_resource::<hud::HudFacePrevHp>();

        app.init_resource::<sync::DeathDelay>();
        app.init_resource::<sync::RestartRequested>();
        app.init_resource::<sync::NewGameRequested>();
        app.init_resource::<sync::AdvanceLevelRequested>();

        // -----------------------------
        // HUD spawn
        // -----------------------------
        app.add_systems(Startup, hud::setup_hud);

        // IMPORTANT: chain() makes ordering deterministic.
        app.add_systems(
            Update,
            (
                // -----------------------------
                // Gameplay -> HUD State
                // -----------------------------
                sync::apply_enemy_fire_to_player_vitals,
                sync::sync_player_hp_with_hud,

                // -----------------------------
                // Death / Game Over Flow
                // -----------------------------
                sync::handle_player_death_once,
                sync::tick_death_delay_and_request_restart,
                sync::game_over_input,

                // -----------------------------
                // Viewmodel
                // -----------------------------
                hud::sync_viewmodel_size,
                hud::weapon_fire_and_viewmodel,

                // -----------------------------
                // HUD Digits + Icons
                // -----------------------------
                hud::sync_hud_hp_digits,
                hud::sync_hud_ammo_digits,
                hud::sync_hud_score_digits,
                hud::sync_hud_lives_digits,
                hud::sync_hud_icons,

                // -----------------------------
                // HUD Face
                // -----------------------------
                hud::sync_hud_face,

                // -----------------------------
                // Overlays
                // -----------------------------
                hud::flash_on_hp_drop,
                hud::tick_damage_flash,
                hud::tick_death_overlay,
                hud::sync_game_over_overlay_visibility,
            )
                .chain(),
        );
    }
}
