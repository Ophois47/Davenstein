use bevy::prelude::*;

mod state;
mod hud;

pub use state::HudState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<hud::WeaponState>()
            .add_systems(Startup, hud::setup_hud)
            // run weapon first, then update HUD text
            .add_systems(Update, hud::weapon_fire_and_viewmodel)
            .add_systems(Update, hud::sync_hud_text.after(hud::weapon_fire_and_viewmodel));
    }
}
