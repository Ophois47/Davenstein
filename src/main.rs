use bevy::prelude::*;
use davelib::player::{grab_mouse, mouse_look, player_move, PlayerSettings};
use davelib::world::setup;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0))
        .init_resource::<PlayerSettings>() // ✅ ensures it exists for mouse_look/player_move
        .add_systems(Startup, setup)
        .add_systems(Update, (grab_mouse, mouse_look))
        .add_systems(FixedUpdate, player_move)
        .run();
}
