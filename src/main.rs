use bevy::prelude::*;
use davelib::player::{player_move, mouse_look, grab_mouse};
use davelib::world::setup;

fn main() {
    App::new()
        // Nearest-neighbor sampling is great for a crunchy Wolf-like look
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        // Fixed timestep for simulation (movement/doors/AI)
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0))
        .add_systems(Startup, setup)
        .add_systems(Update, (grab_mouse, mouse_look))
        .add_systems(FixedUpdate, player_move)
        .run();
}
