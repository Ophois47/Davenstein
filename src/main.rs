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
    tick_hard_stop_sfx,
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
    toggle_god_mode,
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

/// Gate Gameplay Systems Until World Resources Exist
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
	info!("##==> Davenstein Build: {}", env!("CARGO_PKG_VERSION"));
	let assets_path = extract_embedded_assets_to_temp();
	let high_scores = davelib::high_score::HighScores::load();

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
		.insert_resource(high_scores)
		.init_resource::<PlayerSettings>()
		.init_resource::<PlayerControlLock>()
		.init_resource::<PlayerDeathLatch>()
		.init_resource::<davelib::player::GodMode>()
		.init_resource::<davelib::skill::SkillLevel>()
		.init_resource::<ui::sync::DeathDelay>()
		.init_resource::<ui::sync::RestartRequested>()
		.init_resource::<ui::sync::NewGameRequested>()
		.init_resource::<ui::sync::AdvanceLevelRequested>()
		.init_resource::<PushwallOcc>()
		.init_resource::<PushwallState>()
		.init_resource::<PushwallClock>()
		.init_resource::<davelib::level::CurrentLevel>()
		.init_resource::<davelib::audio::MusicMode>()
		.init_resource::<level_complete::LevelComplete>()
		.init_resource::<davelib::level_score::LevelScore>()
		.init_resource::<level_complete::MissionSuccessTally>()
		.init_resource::<level_complete::ElevatorExitDelay>()
		.init_resource::<level_complete::PendingLevelExit>()
		.init_resource::<davelib::high_score::NameEntryState>()
		.add_message::<PlaySfx>()
		.add_message::<RebuildWalls>()
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
		.add_systems(
			Update,
			(
				toggle_god_mode,
				grab_mouse,
				mouse_look,
			)
				.chain()
				.run_if(|lock: Res<PlayerControlLock>, win: Res<level_complete::LevelComplete>| !lock.0 && !win.0),
		)
		.add_systems(
			Update,
			(
				level_complete::tick_elevator_exit_delay,
				level_complete::sync_mission_success_overlay_visibility,
				level_complete::start_mission_success_tally_on_win,
				level_complete::tick_mission_success_tally,
				level_complete::sync_mission_success_stats_text,
				level_complete::mission_success_input,
				level_complete::apply_mission_success_bonus_to_player_score_once,
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
		.add_systems(
			PostUpdate,
			(
				play_sfx_events,
				davelib::audio::tick_auto_stop_sfx,
				tick_hard_stop_sfx,
			)
				.chain(),
		)
		.add_systems(
			PostUpdate,
			(
				davelib::audio::sync_boot_music,
				davelib::audio::sync_level_music,
			)
				.chain(),
		)
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
		.add_systems(
			FixedUpdate,
			rebuild_wall_faces_on_request
				.run_if(world_ready)
				.run_if(|lock: Res<PlayerControlLock>| lock.0),
		)
		.add_systems(
			FixedUpdate,
			(
				davelib::level_score::tick_level_time,
				tick_pushwalls,
				rebuild_wall_faces_on_request,
				door_auto_close,
				door_animate,
				player_move,
				pickups::drop_guard_ammo,
				pickups::drop_mutant_ammo,
				pickups::drop_ss_loot,
				pickups::drop_officer_ammo,
				pickups::drop_hans_key,
				pickups::drop_gretel_key,
				pickups::collect_pickups,
			)
				.chain()
				.run_if(world_ready)
				.run_if(|lock: Res<PlayerControlLock>| !lock.0),
		)
		.run();
}

