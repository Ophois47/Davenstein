/*
Davenstein - by David Petnick
*/
mod combat;
mod pickups;
mod ui;

use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use include_dir::{include_dir, Dir};
use std::path::PathBuf;

use davelib::ai::EnemyAiPlugin;
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

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

fn extract_embedded_assets_to_temp() -> String {
    // Location of Extracted Assets
    let out_dir: PathBuf = std::env::temp_dir().join(format!(
        "{}_assets_{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    ));

    // Dir::extract() Fails if Files Already Exist. Clear First
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("create temp assets dir");

    ASSETS
        .extract(&out_dir)
        .expect("extract embedded assets");

    out_dir.to_string_lossy().to_string()
}

fn debug_self_damage(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<&mut davelib::player::PlayerVitals, With<davelib::player::Player>>,
) {
    if !keys.just_pressed(KeyCode::KeyH) {
        return;
    }

    let Some(mut vitals) = q_player.iter_mut().next() else {
        return;
    };

    vitals.hp = (vitals.hp - 10).max(0);
    info!("DEBUG: self-damage (-10) -> vitals.hp={}", vitals.hp);
}

fn main() {
    let assets_path = extract_embedded_assets_to_temp();
    info!("##==> Davenstein Build: {}", env!("CARGO_PKG_VERSION"));

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: assets_path,
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(ui::UiPlugin)
        .add_plugins(EnemiesPlugin)
        .add_plugins(EnemyAiPlugin)
        .add_plugins(combat::CombatPlugin)
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0))
        .init_resource::<PlayerSettings>()
        .add_message::<PlaySfx>()
        .add_systems(
            Startup,
            (
                setup_audio,
                start_music,
                setup,
                pickups::spawn_test_weapon_pickup,
            ).chain(),
        )
        .add_systems(
            Update,
            (
                grab_mouse,
                mouse_look,
                debug_self_damage,
                ui::sync::sync_player_hp_with_hud,
                pickups::billboard_pickups,
                use_doors,
            ).chain(),
        )
        .add_systems(PostUpdate, play_sfx_events)
        .add_systems(
            FixedUpdate,
            (
                door_auto_close,
                door_animate,
                player_move,
                pickups::drop_guard_ammo,
                pickups::collect_pickups,
            ).chain(),
        )
        .run();
}
