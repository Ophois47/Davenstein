/*
Davenstein - by David Petnick

IMPORTANT: Level rebuild scheduling + Bevy ordering pitfalls (READ BEFORE TOUCHING SCHEDULES)

- This project builds the "world" (MapGrid, WolfPlane1, etc.) inside davelib::world::setup using Commands
- During level transitions (RestartRequested / NewGameRequested / AdvanceLevelRequested) we despawn and rebuild the world at runtime

1) Resource validation panic (runtime)
   - Symptom: panic in a system like pickups::spawn_pickups
       "Parameter Res<MapGrid> failed validation: Resource does not exist"
   - Cause: Bevy validates system params before running system code. If a system has strict Res/ResMut and the resource
     is not currently present, the system panics before your code can early-return
   - Why it happened here:
     - davelib::world::setup inserts MapGrid / WolfPlane1 via Commands
     - Commands are applied later (deferred). During rebuild frames, resources can be temporarily absent
     - If any system that requires Res<MapGrid> runs during that gap, it crashes
   - Fix strategy:
     - Gate gameplay systems behind a run condition that checks resource existence using Option<Res<T>>
     - Do NOT “just make every parameter Option<Res<T>>” inside systems; that doesn’t help if other params remain strict
     - Use a single world_ready() predicate to guard all systems that assume world resources exist

2) Schedule initialization panic (startup, before gameplay)
   - Symptom: panic while initializing schedule PostUpdate
       "Tried to order against SystemTypeSet(... davelib::world::setup ...) in a schedule that has more than one instance"
   - Cause: In Bevy 0.18, ordering against a SystemTypeSet becomes ambiguous if the same system type is registered
     more than once in the same schedule. Bevy refuses to build the schedule and panics
   - Why it happened here:
     - We registered davelib::world::setup multiple times in PostUpdate (once per rebuild path)
     - Then we tried to use `.after(setup)` to guarantee ordering for spawn_decorations / spawn_pickups
     - With multiple setup instances in PostUpdate, `.after(setup)` is ambiguous, so Bevy panics at schedule build time
   - Fix strategy (the correct pattern for this codebase):
     - Register the rebuild pipeline systems (despawn -> setup -> decorations -> pickups) exactly ONCE per schedule
     - Gate that pipeline with a single run_if predicate that detects ANY rebuild request:
         level_rebuild_requested = RestartRequested || NewGameRequested || AdvanceLevelRequested
     - Keep the per-request “finish” systems (restart_finish / new_game_finish / advance_level_finish) separate and gated individually
     - This avoids duplicate SystemTypeSet instances and preserves deterministic ordering with `.after(...)`

- Bevy 0.18+ API surface changes: don’t assume helper functions exist (ex: apply_deferred import failed on Linux build)
- Don’t assume crate-local modules exist (crate::map / crate::level) after refactors; this project's source of truth
  for MapGrid/WolfPlane1 is davelib (use davelib::map::MapGrid and davelib::level::WolfPlane1)
- Avoid "just chain systems" advice; it can be incompatible with the project's current Bevy version and can break wiring
- World-building resources are created via Commands inside setup and may not exist during transitions
- Guard gameplay systems with world_ready() so they never run without MapGrid/WolfPlane1
- In PostUpdate, do NOT register multiple instances of setup/spawn systems and then order “after setup”
- Instead: one rebuild pipeline gated by level_rebuild_requested(), plus per-request finish systems gated individually
*/

use crate::{
    combat,
    episode_end_app as episode_end,
    level_complete,
    pak_assets,
    pickups,
    restart,
    save,
    settings,
    ui,
};

use bevy::prelude::*;
use bevy::asset::{
	AssetMetaCheck,
	AssetPlugin,
};
use bevy::camera::ClearColorConfig;
use bevy::camera::visibility::RenderLayers;
use bevy::window::{PresentMode, PrimaryWindow, ScreenEdge, WindowPlugin};
use bevy::light::cluster::GlobalClusterSettings;
use davelib::options::{MenuUiCamera, MenuUiCameraRef};

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
    apply_look,
    level_pitch_without_mouselook,
    door_animate,
    door_auto_close,
    init_player_render_interp,
    player_interp_capture_after_tic,
    player_interp_restore_before_tic,
    apply_player_render_interp,
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
};
use davelib::world::{
    setup,
    rebuild_wall_faces_on_request,
    RebuildWalls,
};

/// Gate Gameplay Systems Until World Resources Exist
// Introduced new transition path for level advance (AdvanceLevelRequested) and
// rebuilding level during runtime Bevy validates system parameters before running
//  system code. So even Option<Res<MapGrid>> inside a system caused other Res<...>
//  params to panic. More generally, during transitions there can be frames where
//  world resources aren't present yet (because Commands apply deferred), and any
//  system using strict Res / ResMut will panic
pub(crate) fn world_ready(
	map: Option<Res<MapGrid>>,
	plane1: Option<Res<WolfPlane1>>,
) -> bool {
	map.is_some() && plane1.is_some()
}

fn level_rebuild_requested(
    r: Res<ui::sync::RestartRequested>,
    n: Res<ui::sync::NewGameRequested>,
    a: Res<ui::sync::AdvanceLevelRequested>,
    l: Res<save::LoadGameRequested>,
) -> bool {
    r.0 || n.0 || a.0 || l.0.is_some()
}

/// Force CPU Light Clustering on the Software Render Path Only
// The llvmpipe Renderer Clusters Lights on the CPU Regardless so the GPU Compute
// Clustering Dispatch Is Pure Overhead There. Desktop Builds Keep the GPU Path
fn disable_gpu_clustering(_settings: Option<ResMut<GlobalClusterSettings>>) {
	#[cfg(feature = "software_render")]
	if let Some(mut settings) = _settings {
		settings.gpu_clustering = None;
	}
}

/// Spawn the Persistent Window-Space UI Camera Once at Startup and Record Its
/// Entity in 'MenuUiCameraRef'. This Camera Draws Every Window-Laid-Out UI Root
/// (Menus, Splash, the Intermission Tally, Debug Overlays) at the Window's Own
/// Logical Size, Independent of the Low-Res World Canvas
///
/// It Sits at a High Render 'order' (Above the World Present Camera at order 1)
/// so Menus Composite On Top of the Presented Game, and Uses
/// 'ClearColorConfig::None' so It Never Wipes the Game Behind It: When No Menu Is
/// Open It Simply Draws Nothing and the Presented World Shows Through, and When a
/// Full-Screen Menu Is Open That Menu's Own Opaque Background Covers the Game.
/// It Deliberately Does NOT Carry 'IsDefaultUiCamera' - That Stays on the World
/// Canvas Camera so the In-Game HUD Keeps Rendering Into the Canvas and Scaling
/// With render_scale. It Is Never Despawned by the Level Rebuild Path, so Its
/// Entity Stays Valid for the Whole Run
fn spawn_menu_ui_camera(mut commands: Commands) {
	let entity = commands
		.spawn((
			MenuUiCamera,
			Camera2d::default(),
			Camera {
				// Above the World Present Camera (order 1) so Menus Draw on Top
				order: 10,
				// Composite Over the Game Instead of Clearing It (See Doc Above)
				clear_color: ClearColorConfig::None,
				..default()
			},
			// MSAA Must Be Off Here. This Camera Composites Directly Onto the
			// Existing 1-Sample Window Surface ('ClearColorConfig::None'), so a
			// Multisampled (4x) Depth Attachment Would Not Match the 1-Sample
			// Color Target and wgpu Rejects the Render Pass. A Pure 2-D UI Overlay
			// Gains Nothing From MSAA Anyway
			Msaa::Off,
		))
		.id();

	commands.insert_resource(MenuUiCameraRef(entity));
}

/// Persistent Background Clear Camera. Fixes the Magenta/Purple Flash Seen Before
/// the First Splash Screen on Mobile.
///
/// Until the Player Starts a Game, the World Cameras (3-D, HUD, Present) Do Not
/// Exist - They Are Spawned by 'world::setup', Which Only Runs on a Level Rebuild.
/// The Only Camera Alive at Startup Is the MenuUiCamera, and It Uses
/// 'ClearColorConfig::None' so It Can Composite Menus Over Live Gameplay. With No
/// Camera Clearing the Window, the Freshly Acquired Swapchain Drawable Is
/// Uninitialized; iOS / Metal Presents That as Magenta Until the Splash Image
/// Loads and Covers It (Most Visible When the First Asset Load Is Slow). Desktop
/// GPUs Usually Hand Back Zeroed Drawables, so the Flash Is a Mobile Symptom.
///
/// This Camera Sits at the Lowest Render Order and Clears the Window to Black
/// Every Frame, Guaranteeing a Defined Background Before Anything Composites. It
/// Renders Nothing Itself - It Is Pinned to an Otherwise Unused Render Layer so It
/// Only Clears - Never Owns 'IsDefaultUiCamera', Runs at One Sample to Match the
/// Window Surface, and Is Never Despawned. Once the Present Camera Exists It Draws
/// the World Straight Over This Black, so the Camera Is Harmless During Play
fn spawn_background_clear_camera(mut commands: Commands) {
	commands.spawn((
		Camera2d::default(),
		Camera {
			// Below Every Other Camera (World 0/1/2, Menu 10) so It Clears First
			order: -1000,
			clear_color: ClearColorConfig::Custom(Color::BLACK),
			..default()
		},
		// One Sample, Matching the Window Surface and the Menu Camera - a
		// Multisampled Clear-Only Camera Would Mismatch the 1-Sample Target
		Msaa::Off,
		// An Unused Render Layer: Nothing Is Assigned to Layer 8, so This Camera
		// Clears the Window and Draws No Sprite or UI Node, Avoiding Any Chance of
		// It Redrawing Content or Forming a Canvas Feedback Loop
		RenderLayers::layer(8),
	));
}

// Reveal the Primary Window a Few Frames After Launch on Mobile, Where It Started
// Hidden (See the WindowPlugin 'visible' Field). By the Time This Fires the
// Background Clear Camera Has Painted Black for Several Frames, so Unhiding Shows a
// Clean Black Screen Rather Than the Uninitialized Metal Drawable. A Small Frame
// Count, Not Zero: One Frame Is Not Always Enough for the Cleared Drawable to Have
// Reached the Surface. The System Removes Its Own Effect by Latching 'done' and Is
// a No-Op on Desktop, Where the Window Was Never Hidden
fn reveal_window_after_first_frames(
	mut frames: Local<u32>,
	mut done: Local<bool>,
	mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
	if *done || !davelib::options::MOBILE_PLATFORM {
		return;
	}
	*frames += 1;
	// Three Frames of Confirmed Black Before Revealing
	if *frames >= 3 {
		if let Ok(mut window) = windows.single_mut() {
			window.visible = true;
		}
		*done = true;
	}
}

pub fn run() {
	davelib::input::install_platform_gamepad_mappings();

	info!("##==> Davenstein Build: {}", env!("CARGO_PKG_VERSION"));
	let asset_file_path = if cfg!(debug_assertions) {
		"assets".to_string()
	} else {
		".".to_string()
	};
	let high_scores = davelib::high_score::HighScores::load();

	let default_plugins = DefaultPlugins
		.set(AssetPlugin {
			file_path: asset_file_path,
			meta_check: AssetMetaCheck::Never,
			watch_for_changes_override: Some(cfg!(debug_assertions)),
			..default()
		})
		.set(ImagePlugin::default_nearest())
		.set(WindowPlugin {
			primary_window: Some(Window {
				present_mode: PresentMode::Fifo,
				// iOS Home Indicator + System Edge Gestures. Both Fields Are Inert
				// on Desktop (bevy_window Documents Them as iOS-Only), so There Is
				// No cfg Needed. 'prefers_home_indicator_hidden' Removes the White
				// Bar That Otherwise Sits Over the Fire / OK Thumb Arc. Deferring
				// System Gestures on ALL Edges Is the Important One: Without It the
				// First Swipe Near the Bottom Edge Opens Control Center or the App
				// Switcher Instead of Reaching Our Controls, Which the Player
				// Experiences as the Bottom-Row Buttons (Confirm, Back, Weapons)
				// Suddenly Not Responding. Deferral Makes iOS Require a SECOND Swipe
				// for Its Own Gesture, Handing the First Touch to the Game
				prefers_home_indicator_hidden: true,
				preferred_screen_edges_deferring_system_gestures: ScreenEdge::All,
				// Start Hidden on Mobile, Revealed by 'reveal_window_after_first_frames'
				// Once a Black Frame Has Actually Been Drawn. This Closes the Residual,
				// Intermittent Magenta Flash the Background Clear Camera Alone Could Not:
				// on iOS the Surface's Very First Drawable Can Be Presented Uninitialized
				// AROUND Surface Configuration, Before Bevy's First Cleared Frame Reaches
				// the Screen - Below Any Camera's Reach. A Hidden Window Composites
				// Nothing, so That Garbage Drawable Is Never Visible; by the Time We
				// Unhide, the Clear Camera Has Painted Black. Desktop Stays Visible From
				// the Start (No Such Uninitialized-Present Behaviour, and Hiding Would
				// Only Add a Startup Flicker There)
				visible: !davelib::options::MOBILE_PLATFORM,
				..default()
			}),
			..default()
		});

	// Software Rendering: Select llvmpipe and Drop the Heavy GPU-Only Passes
	// Zeroing Storage Textures Disables Atmosphere SSAO and Env Map Generation
	// Binding Array Limits Stay Native so Bindless StandardMaterial Still Builds
	// Compute and Storage Buffers Stay Native so CPU Preprocessing Runs Safely
	// The WGPU_ADAPTER_NAME Environment Variable Still Overrides the Baked Adapter
	#[cfg(feature = "software_render")]
	let default_plugins = {
		let mut limits = bevy::render::settings::WgpuLimits::default();
		limits.max_storage_textures_per_shader_stage = 0;
		limits.max_binding_array_elements_per_shader_stage = u32::MAX;
		limits.max_binding_array_sampler_elements_per_shader_stage = u32::MAX;
		default_plugins.set(bevy::render::RenderPlugin {
			render_creation: bevy::render::settings::WgpuSettings {
				adapter_name: Some("llvmpipe".into()),
				constrained_limits: Some(limits),
				..default()
			}
			.into(),
			..default()
		})
	};

	App::new()
		.add_plugins(pak_assets::PakAssetsPlugin)
		.add_plugins(default_plugins)
		.add_plugins(davelib::options::OptionsPlugin)
		.add_plugins(davelib::input::InputPlugin)
		.add_plugins(davelib::perf_overlay::PerfOverlayPlugin)
		.add_plugins(ui::UiPlugin)
		.add_plugins(save::SavePlugin)
		.add_plugins(settings::SettingsPlugin)
		.add_plugins(EnemiesPlugin)
		.add_plugins(EnemyAiPlugin)
		.add_plugins(combat::CombatPlugin)
		.add_plugins(episode_end::EpisodeEndPlugin)
		// Wolf3D Ran its Whole Simulation at 70 Hz. Matching That Makes One
		// FixedUpdate Equal Exactly One AI Tic (tics == 1), so the Ported
		// Formulas Behave as They Did at Full Speed and the Manual Sub-Tick
		// Accumulators (AiTicker, Pushwall Clock) Are no Longer Needed
		.insert_resource(Time::<Fixed>::from_hz(70.0))
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
		.init_resource::<davelib::pushwalls::CompletedPushwalls>()
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
		.add_systems(Startup, spawn_menu_ui_camera)
		.add_systems(Startup, spawn_background_clear_camera)
		.add_systems(Update, reveal_window_after_first_frames)
		.add_systems(Startup, disable_gpu_clustering)
		.add_systems(
			Update,
			toggle_god_mode.run_if(|lock: Res<PlayerControlLock>, win: Res<level_complete::LevelComplete>| !lock.0 && !win.0),
		)
		.add_systems(
			Update,
			apply_look
				.after(davelib::input::InputGather)
				.run_if(|lock: Res<PlayerControlLock>, win: Res<level_complete::LevelComplete>| !lock.0 && !win.0),
		)
		// Deliberately NOT Gated by the Control Lock, so It Also Runs While the
		// Options Menu Is Open (Where Control Is Locked). This Levels the Pitch the
		// Instant Mouselook Is Turned Off, so Returning to Play Always Looks Ahead.
		// Ordered After 'apply_look' so It Has the Final Word on Pitch That Frame
		.add_systems(
			Update,
			level_pitch_without_mouselook.after(apply_look),
		)
		// The Intermission Is One State Machine, Not Seven Independent Systems.
		// Keep Every Mutation And Read In a Fixed Order so a New Schedule Edge
		// Elsewhere Cannot Render Pre-Initialization Values, Tick Before the
		// Targets Exist, or Advance the Level Before Its Bonus Is Applied
		.add_systems(Update, level_complete::tick_elevator_exit_delay)
		.add_systems(
			Update,
			level_complete::start_mission_success_tally_on_win
				.after(level_complete::tick_elevator_exit_delay),
		)
		.add_systems(
			Update,
			level_complete::tick_mission_success_tally
				.after(level_complete::start_mission_success_tally_on_win),
		)
		.add_systems(
			Update,
			level_complete::apply_mission_success_bonus_to_player_score_once
				.after(level_complete::tick_mission_success_tally),
		)
		// After InputGather so the Tally Reacts to This Frame's Confirm
		// (Gamepad South, a Touch Tap) Rather Than Last Frame's. Also Run After
		// Bonus Application so Level Advancement Cannot Reassign CurrentLevel
		// Before the Completed Floor's Score and Episode Statistics Are Final
		.add_systems(
			Update,
			level_complete::mission_success_input
				.after(davelib::input::InputGather)
				.after(level_complete::apply_mission_success_bonus_to_player_score_once),
		)
		.add_systems(
			Update,
			level_complete::sync_mission_success_stats_text
				.after(level_complete::mission_success_input),
		)
		.add_systems(
			Update,
			level_complete::sync_mission_success_overlay_visibility
				.after(level_complete::start_mission_success_tally_on_win),
		)
		.add_systems(Update, pickups::billboard_pickups.run_if(world_ready))
		.add_systems(Update, billboard_decorations.run_if(world_ready))
		.add_systems(Update, use_pushwalls.run_if(world_ready).after(davelib::input::InputGather))
		.add_systems(Update, use_doors.run_if(world_ready).after(davelib::input::InputGather))
		.add_systems(Update, level_complete::use_elevator_exit.run_if(world_ready).after(davelib::input::InputGather))
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
		    PostUpdate,
		    restart::load_game_finish
		        .after(pickups::spawn_pickups)
		        .after(restart::restart_finish)
		        .after(restart::new_game_finish)
		        .after(restart::advance_level_finish)
		        .run_if(|r: Res<save::LoadGameRequested>| r.0.is_some()),
		)
		.add_systems(
			FixedUpdate,
			rebuild_wall_faces_on_request
				.run_if(world_ready)
				.run_if(|lock: Res<PlayerControlLock>| lock.0),
		)
		.add_systems(
			FixedUpdate,
			davelib::level_score::tick_level_time
				.run_if(world_ready)
				.run_if(|lock: Res<PlayerControlLock>| !lock.0),
		)
		.add_systems(FixedUpdate, tick_pushwalls.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, rebuild_wall_faces_on_request.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, door_auto_close.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, door_animate.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, player_move.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		// Camera Render Interpolation: Seed the Snapshots When the Player Spawns,
		// Bracket the Fixed Tic to Record Tic-Aligned Positions, and Every Frame
		// Draw the Camera Interpolated Between the Two Most Recent Tics. This Keeps
		// the 70 Hz Simulation Faithful While Presenting Smoothly at Any Refresh Rate
		.add_systems(PreUpdate, init_player_render_interp.run_if(world_ready))
		.add_systems(FixedFirst, player_interp_restore_before_tic.run_if(world_ready))
		.add_systems(FixedLast, player_interp_capture_after_tic.run_if(world_ready))
		.add_systems(Update, apply_player_render_interp.run_if(world_ready))
		.add_systems(FixedUpdate, pickups::drop_guard_ammo.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_mutant_ammo.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_ss_loot.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_officer_ammo.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_hans_key.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::drop_gretel_key.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.add_systems(FixedUpdate, pickups::collect_pickups.run_if(world_ready).run_if(|lock: Res<PlayerControlLock>| !lock.0))
		.run();
}
