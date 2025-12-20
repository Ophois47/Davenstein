mod combat;
mod ui;

use bevy::prelude::*;
use davelib::audio::{
	play_sfx_events,
	setup_audio,
    start_music,
	PlaySfx,
};
use davelib::enemies::EnemiesPlugin;
use davelib::player::{
    door_animate,
	door_auto_close,
	grab_mouse, mouse_look,
	player_move,
	use_doors,
	PlayerSettings,
};
use davelib::world::setup;

fn main() {
    info!("##==> Davenstein Build: {}", env!("CARGO_PKG_VERSION"));

    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(ui::UiPlugin)
        .add_plugins(EnemiesPlugin)
        .add_plugins(combat::CombatPlugin)
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0))
        .init_resource::<PlayerSettings>()
        .add_message::<PlaySfx>()
        .add_systems(Startup, (setup_audio, start_music, setup).chain())
        .add_systems(Update, (grab_mouse, mouse_look, use_doors).chain())
        .add_systems(PostUpdate, play_sfx_events)
        .add_systems(FixedUpdate, (door_auto_close, door_animate, player_move).chain())
        .run();
}
