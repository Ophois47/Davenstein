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

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<DamageFlash>()
            .init_resource::<DeathOverlay>()
            .init_resource::<GameOver>()
            .init_resource::<sync::NewGameRequested>()
            .init_resource::<hud::WeaponState>()
            .add_systems(Startup, hud::setup_hud)
            // 1) Resolve enemy shots into PlayerVitals (gameplay truth)
            // 2) Copy PlayerVitals -> HudState.hp (UI truth)
            // 3) Then do HUD text + flash logic
            .add_systems(
                Update,
                (
                    sync::apply_enemy_fire_to_player_vitals,
                    sync::sync_player_hp_with_hud,
                    sync::handle_player_death_once,
                    sync::tick_death_delay_and_request_restart,
                    sync::game_over_input,
                    hud::sync_viewmodel_size,
                    hud::weapon_fire_and_viewmodel,
                    hud::sync_hud_hp_digits,
                    hud::sync_hud_ammo_digits,
                    hud::sync_hud_score_digits,
                    hud::sync_hud_lives_digits,
                    hud::sync_hud_icons,
                    hud::flash_on_hp_drop,
                    hud::tick_damage_flash,
                    hud::tick_death_overlay,
                    hud::sync_game_over_overlay_visibility,
                )
                    .chain(),
            );
    }
}
