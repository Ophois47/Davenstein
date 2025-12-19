mod state;
mod hud;

pub use state::HudState;

pub struct UiPlugin;

impl bevy::prelude::Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<HudState>()
            .add_systems(bevy::prelude::Startup, hud::setup_hud)
            .add_systems(bevy::prelude::Update, hud::sync_hud_text);
    }
}
