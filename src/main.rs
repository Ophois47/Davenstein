/*
Davenstein - by David Petnick
*/
// IMPORTANT: Level rebuild scheduling + Bevy 0.18 ordering pitfalls (read before touching schedules)
//
// Context
// - This project builds the "world" (MapGrid, WolfPlane1, etc.) inside davelib::world::setup using Commands
// - During level transitions (RestartRequested / NewGameRequested / AdvanceLevelRequested) we despawn and rebuild the world at runtime
//
//
// 1) Resource validation panic (runtime)
//    - Symptom: panic in a system like pickups::spawn_pickups
//        "Parameter Res<MapGrid> failed validation: Resource does not exist"
//    - Cause: Bevy validates system params before running system code. If a system has strict Res/ResMut and the resource
//      is not currently present, the system panics before your code can early-return
//    - Why it happened here:
//      - davelib::world::setup inserts MapGrid / WolfPlane1 via Commands
//      - Commands are applied later (deferred). During rebuild frames, resources can be temporarily absent
//      - If any system that requires Res<MapGrid> runs during that gap, it crashes
//    - Fix strategy:
//      - Gate gameplay systems behind a run condition that checks resource existence using Option<Res<T>>
//      - Do NOT “just make every parameter Option<Res<T>>” inside systems; that doesn’t help if other params remain strict
//      - Use a single world_ready() predicate to guard all systems that assume world resources exist
//
// 2) Schedule initialization panic (startup, before gameplay)
//    - Symptom: panic while initializing schedule PostUpdate
//        "Tried to order against SystemTypeSet(... davelib::world::setup ...) in a schedule that has more than one instance"
//    - Cause: In Bevy 0.18, ordering against a SystemTypeSet becomes ambiguous if the same system type is registered
//      more than once in the same schedule. Bevy refuses to build the schedule and panics
//    - Why it happened here:
//      - We registered davelib::world::setup multiple times in PostUpdate (once per rebuild path)
//      - Then we tried to use `.after(setup)` to guarantee ordering for spawn_decorations / spawn_pickups
//      - With multiple setup instances in PostUpdate, `.after(setup)` is ambiguous, so Bevy panics at schedule build time
//    - Fix strategy (the correct pattern for this codebase):
//      - Register the rebuild pipeline systems (despawn -> setup -> decorations -> pickups) exactly ONCE per schedule
//      - Gate that pipeline with a single run_if predicate that detects ANY rebuild request:
//          level_rebuild_requested = RestartRequested || NewGameRequested || AdvanceLevelRequested
//      - Keep the per-request “finish” systems (restart_finish / new_game_finish / advance_level_finish) separate and gated individually
//      - This avoids duplicate SystemTypeSet instances and preserves deterministic ordering with `.after(...)`
//
// Additional landmines hit (lessons learned)
// - Bevy 0.18+ API surface changes: don’t assume helper functions exist (ex: apply_deferred import failed on Linux build)
// - Don’t assume crate-local modules exist (crate::map / crate::level) after refactors; this project’s source of truth
//   for MapGrid/WolfPlane1 is davelib (use davelib::map::MapGrid and davelib::level::WolfPlane1)
// - Avoid “just chain systems” advice; it can be incompatible with the project’s current Bevy version and can break wiring
// - Make changes surgical: do not remove or unhook existing systems; fix only the minimal scheduling/ordering needed
//
// Summary (what to do going forward)
// - World-building resources are created via Commands inside setup and may not exist during transitions
// - Guard gameplay systems with world_ready() so they never run without MapGrid/WolfPlane1
// - In PostUpdate, do NOT register multiple instances of setup/spawn systems and then order “after setup”
// - Instead: one rebuild pipeline gated by level_rebuild_requested(), plus per-request finish systems gated individually
mod combat;
mod episode_end;
mod level_complete;
mod pickups;
mod restart;
mod ui;

use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use include_dir::{include_dir, Dir};
use std::path::PathBuf;

use davelib::ai::EnemyAiPlugin;
use davelib::map::MapGrid;
use davelib::level::WolfPlane1;
use davelib::audio::{
    play_sfx_events,
    tick_hard_stop_sfx,
    setup_audio,
    start_music,
    PlaySfx,
};
use davelib::decorations::{
    billboard_decorations,
    spawn_decorations,
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
	map: Option<Res<MapGrid>>,
	plane1: Option<Res<WolfPlane1>>,
) -> bool {
	map.is_some() && plane1.is_some()
}

fn level_rebuild_requested(
	r: Res<ui::sync::RestartRequested>,
	n: Res<ui::sync::NewGameRequested>,
	a: Res<ui::sync::AdvanceLevelRequested>,
) -> bool {
	r.0 || n.0 || a.0
}

#[derive(Component)]
struct BootUiCamera;

fn spawn_boot_ui_camera(mut commands: Commands) {
	commands.spawn((
		BootUiCamera,
		Camera2d::default(),
		Camera {
			order: -10,
			..default()
		},
	));
}

fn disable_boot_ui_camera_when_player_camera_ready(
	mut q_boot: Query<&mut Camera, With<BootUiCamera>>,
	q_ui_cam: Query<(), (With<Camera>, With<bevy::ui::prelude::IsDefaultUiCamera>, Without<BootUiCamera>)>,
) {
	if q_ui_cam.iter().next().is_none() {
		return;
	}

	for mut cam in q_boot.iter_mut() {
		cam.is_active = false;
	}
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
		.add_plugins(episode_end::EpisodeEndPlugin)
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
		.init_resource::<davelib::level_score::EpisodeStats>()
		.init_resource::<level_complete::MissionSuccessTally>()
		.init_resource::<level_complete::ElevatorExitDelay>()
		.init_resource::<level_complete::PendingLevelExit>()
		.init_resource::<davelib::high_score::NameEntryState>()
		.add_message::<PlaySfx>()
		.add_message::<RebuildWalls>()
		.add_systems(Startup, setup_audio)
		.add_systems(Startup, start_music.after(setup_audio))
		.add_systems(Startup, spawn_boot_ui_camera)
		.add_systems(
			Update,
			toggle_god_mode.run_if(|lock: Res<PlayerControlLock>, win: Res<level_complete::LevelComplete>| !lock.0 && !win.0),
		)
		.add_systems(
			Update,
			grab_mouse.run_if(|lock: Res<PlayerControlLock>, win: Res<level_complete::LevelComplete>| !lock.0 && !win.0),
		)
		.add_systems(
			Update,
			mouse_look.run_if(|lock: Res<PlayerControlLock>, win: Res<level_complete::LevelComplete>| !lock.0 && !win.0),
		)
		.add_systems(Update, level_complete::tick_elevator_exit_delay)
		.add_systems(Update, level_complete::sync_mission_success_overlay_visibility)
		.add_systems(Update, level_complete::start_mission_success_tally_on_win)
		.add_systems(Update, level_complete::tick_mission_success_tally)
		.add_systems(Update, level_complete::sync_mission_success_stats_text)
		.add_systems(Update, level_complete::mission_success_input)
		.add_systems(Update, level_complete::apply_mission_success_bonus_to_player_score_once)
		.add_systems(Update, pickups::billboard_pickups.run_if(world_ready))
		.add_systems(Update, billboard_decorations.run_if(world_ready))
		.add_systems(Update, use_pushwalls.run_if(world_ready))
		.add_systems(Update, use_doors.run_if(world_ready))
		.add_systems(Update, level_complete::use_elevator_exit.run_if(world_ready))
		.add_systems(PostUpdate, play_sfx_events)
		.add_systems(PostUpdate, davelib::audio::tick_auto_stop_sfx)
		.add_systems(PostUpdate, tick_hard_stop_sfx)
		.add_systems(PostUpdate, davelib::audio::sync_boot_music)
		.add_systems(PostUpdate, davelib::audio::sync_level_music)
		.add_systems(
			PostUpdate,
			(
				restart::restart_despawn_level,
				setup,
				ApplyDeferred,
				disable_boot_ui_camera_when_player_camera_ready,
				spawn_decorations,
				pickups::spawn_pickups,
			)
				.chain()
				.run_if(level_rebuild_requested)
		)
		.add_systems(
			PostUpdate,
			restart::restart_finish
				.after(pickups::spawn_pickups)
				.run_if(|r: Res<ui::sync::RestartRequested>| r.0),
		)
		.add_systems(
			PostUpdate,
			restart::new_game_finish
				.after(pickups::spawn_pickups)
				.run_if(|r: Res<ui::sync::NewGameRequested>| r.0),
		)
		.add_systems(
			PostUpdate,
			restart::advance_level_finish
				.after(pickups::spawn_pickups)
				.run_if(|r: Res<ui::sync::AdvanceLevelRequested>| r.0),
		)
		.add_systems(
			FixedUpdate,
			rebuild_wall_faces_on_request
				.run_if(world_ready)
				.run_if(|lock: Res<PlayerControlLock>| lock.0),
		)
		.add_systems(FixedUpdate, davelib::level_score::tick_level_time.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, tick_pushwalls.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, rebuild_wall_faces_on_request.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, door_auto_close.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, door_animate.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, player_move.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_guard_ammo.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_mutant_ammo.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_ss_loot.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_officer_ammo.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_hans_key.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_gretel_key.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::collect_pickups.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.run();
}
