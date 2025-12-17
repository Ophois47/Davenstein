use bevy::prelude::*;
use davelib::audio::{play_sfx_events, setup_audio, PlaySfx};
use davelib::player::{grab_mouse, mouse_look, player_move, use_doors, PlayerSettings};
use davelib::world::setup;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0))
        .init_resource::<PlayerSettings>() // ensures it exists for mouse_look/player_move
        .add_message::<PlaySfx>()
        .add_systems(Startup, (setup, setup_audio))
        .add_systems(Update, (grab_mouse, mouse_look, use_doors, play_sfx_events))
        .add_systems(FixedUpdate, player_move)
        .run();
}
