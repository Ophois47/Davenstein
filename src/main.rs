/*
Davenstein - by David Petnick
*/
mod combat;
mod level_complete;
mod pickups;
mod restart;
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
use davelib::decorations::{
    billboard_decorations,
    spawn_wolf_e1m1_decorations,
};
use davelib::enemies::EnemiesPlugin;
use davelib::player::{
    door_animate,
    door_auto_close,
    grab_mouse, mouse_look,
    player_move,
    use_doors,
    PlayerSettings,
    PlayerControlLock,
    PlayerDeathLatch,
};
use davelib::pushwalls::{
    use_pushwalls,
    tick_pushwalls,
    PushwallOcc,
    PushwallState,
    PushwallClock,
};
use davelib::world::{
    setup,
    rebuild_wall_faces_on_request,
    RebuildWalls,
};

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

/// Gate gameplay systems until the world resources exist
// Introduced new transition path for level advance (AdvanceLevelRequested) and 
// rebuilding level during runtime Bevy validates system parameters before running
//  system code. So even Option<Res<MapGrid>> inside a system caused other Res<...>
//  params to panic. More generally, during transitions there can be frames where
//  world resources aren't present yet (because Commands apply deferred), and any
//  system using strict Res / ResMut will panic
fn world_ready(
    grid: Option<Res<davelib::map::MapGrid>>,
    solid: Option<Res<davelib::decorations::SolidStatics>>,
    markers: Option<Res<davelib::pushwalls::PushwallMarkers>>,
) -> bool {
    grid.is_some() && solid.is_some() && markers.is_some()
}

fn main() {
    let assets_path = extract_embedded_assets_to_temp();
    info!("##==> Davenstein Build: {}", env!("CARGO_PKG_VERSION"));

    App::new()
        // -----------------------------
        // Plugins (Engine + Game)
        // -----------------------------
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
        // -----------------------------
        // Core Resources / State
        // -----------------------------
        .insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0))
        .init_resource::<PlayerSettings>()
        .init_resource::<PlayerControlLock>()
        .init_resource::<PlayerDeathLatch>()
        .init_resource::<ui::sync::DeathDelay>()
        .init_resource::<ui::sync::RestartRequested>()
        .init_resource::<ui::sync::NewGameRequested>() // make explicit (even if UiPlugin also does it)
        .init_resource::<ui::sync::AdvanceLevelRequested>()
        .init_resource::<PushwallOcc>()
        .init_resource::<PushwallState>()
        .init_resource::<PushwallClock>()
        .init_resource::<level_complete::LevelComplete>()
        .init_resource::<davelib::level::CurrentLevel>()
        // -----------------------------
        // Messages / Events
        // -----------------------------
        .add_message::<PlaySfx>()
        .add_message::<RebuildWalls>()
        // -----------------------------
        // Startup:
        // Load Audio,
        // Build Initial Level,
        // Spawn Content
        // -----------------------------
        .add_systems(
            Startup,
            (
                setup_audio,
                start_music,
                setup,
                spawn_wolf_e1m1_decorations,
                pickups::spawn_pickups,
            )
                .chain(),
        )
        // -----------------------------
        // Update:
        // Input/UI + World Gameplay
        // -----------------------------
        .add_systems(
            Update,
            (
                grab_mouse,
                mouse_look,
                level_complete::mission_success_input,
                level_complete::sync_mission_success_overlay_visibility,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                pickups::billboard_pickups,
                billboard_decorations,
                use_pushwalls,
                use_doors,
                level_complete::use_elevator_exit,
            )
                .chain()
                .run_if(world_ready),
        )
        // -----------------------------
        // PostUpdate:
        // Audio,
        // Level Transitions
        // -----------------------------
        .add_systems(PostUpdate, play_sfx_events)
        .add_systems(PostUpdate, davelib::audio::sync_level_music)
        .add_systems(
            PostUpdate,
            (
                restart::restart_despawn_level,
                setup,
                spawn_wolf_e1m1_decorations,
                pickups::spawn_pickups,
                restart::restart_finish,
            )
                .chain()
                .run_if(|r: Res<ui::sync::RestartRequested>| r.0),
        )
        .add_systems(
            PostUpdate,
            (
                restart::restart_despawn_level,
                setup,
                spawn_wolf_e1m1_decorations,
                pickups::spawn_pickups,
                restart::new_game_finish,
            )
                .chain()
                .run_if(|r: Res<ui::sync::NewGameRequested>| r.0),
        )
        .add_systems(
            PostUpdate,
            (
                restart::restart_despawn_level,
                setup,
                spawn_wolf_e1m1_decorations,
                pickups::spawn_pickups,
                restart::advance_level_finish,
            )
                .chain()
                .run_if(|r: Res<ui::sync::AdvanceLevelRequested>| r.0),
        )
        // -----------------------------
        // FixedUpdate: Simulation
        // -----------------------------
        .add_systems(
            FixedUpdate,
            (
                tick_pushwalls,
                rebuild_wall_faces_on_request,
                door_auto_close,
                door_animate,
                player_move,
                pickups::drop_guard_ammo,
                pickups::collect_pickups,
            )
                .chain()
                .run_if(world_ready),
        )
        .run();
}
