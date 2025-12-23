/*
Davenstein - by David Petnick
*/
mod state;
mod hud;

use bevy::prelude::*;

pub use state::HudState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<hud::WeaponState>()
            .add_systems(Startup, hud::setup_hud)
            .add_systems(Update, hud::weapon_fire_and_viewmodel)
            .add_systems(Update, hud::sync_hud_text.after(hud::weapon_fire_and_viewmodel));
    }
}
