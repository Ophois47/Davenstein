/*
Davenstein - by David Petnick
*/

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use davelib::audio::{MusicMode, MusicModeKind, PlaySfx, SfxKind};
use davelib::level::{CurrentLevel, LevelId, WolfPlane1};
use davelib::map::MapGrid;
use davelib::episode_end::ScriptedCamera;
use davelib::player::{LookAngles, Player, PlayerControlLock, PlayerRenderInterp};

use crate::ui::HudState;
use crate::ui::SplashStep;
use crate::ui::EpisodeEndImages;
use crate::ui::level_end_font::LevelEndBitmapText;

pub struct EpisodeEndPlugin;

impl Plugin for EpisodeEndPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EpisodeEndFlow>()
			.add_systems(Update, start_bj_cutscene.run_if(world_ready))
			// FixedUpdate, NOT Update, and This Is the Entire Reason the Sequence
			// Broke After the Camera Interpolation Refactor
			//
			// 'apply_player_render_interp' Runs in Update and Unconditionally
			// Overwrites the Player Translation With a Lerp Between the Two Most
			// Recent Fixed-Tic Snapshots. A Camera Move Also Written in Update Has
			// No Ordering Contract With It, So Whichever System Bevy Happens to Run
			// Second Wins, and the Camera Oscillates Between the Scripted Target and
			// the Stale Snapshot at the Beat Frequency Between 70 Hz and the Display
			// Rate. That Oscillation Dragged the Near Plane Back and Forth Through
			// the Door Frame, Which Is What Read On Screen as Flickering Walls
			//
			// Running Inside the Fixed Loop Puts the Write After 'FixedFirst'
			// Restore and Before 'FixedLast' Capture, So the Snapshots Record the
			// Scripted Pose and the Interpolator SMOOTHS the Victory Camera Instead
			// of Undoing It. It Is Also What the Original Did: VictorySpin Was
			// Called Once per 70 Hz Tic From T_Player
			.add_systems(FixedUpdate, victory_spin)
			.add_systems(Update, tick_bj_cutscene)
			.add_systems(Update, tick_boss_death_replay_intro)
			.add_systems(Update, start_death_cam)
			.add_systems(Update, tick_death_cam)
			.add_systems(Update, episode_end_finish_to_ui);
	}
}

// ---------------------------------------------------------------------------
// VICTORY SEQUENCE CONSTANTS
//
// Every Number Below Is Taken From the Original Sources Rather Than Tuned by Feel:
// VictorySpin in WL_AGENT.C for the Camera, SpawnBJVictory / T_BJRun / T_BJJump and
// the s_bj* State Table in WL_ACT2.C for BJ. Wolfenstein Measured Distance in
// 1/65536ths of a Tile (TILEGLOBAL) and Time in 70 Hz Tics, so Each Rate Below Is
// the Original Per-Tic Value Converted to Tiles per Second at 70 Hz. Davenstein
// Already Runs Its Fixed Step at Exactly 70 Hz, So One Fixed Step Is One Tic and
// the Conversions Are Exact
//
// Axis Mapping: Wolf's Map y Grows SOUTH and Equals Davenstein's z (the Plane 1
// Index Is tz * width + tx), so Wolf North Is -Z and Wolf's 270 Degrees Is +Z
// ---------------------------------------------------------------------------

// How Far North the Camera Slides, in Tiles
//
// Original: desty = ((tiley - 5) << TILESHIFT) - 0x3000
// (tiley << TILESHIFT) Is the Tile's NORTH EDGE, Not Its Center. A Davenstein Tile
// tz Spans z in [tz - 0.5, tz + 0.5), so That Edge Is tz - 0.5. The 0x3000 Is
// 0x3000 / 0x10000 = 0.1875 Tiles Further North. Total: 0.5 + 5 + 0.1875
const VICTORY_CAM_SLIDE_TILES: f32 = 5.6875;

// Camera Slide Rate. Original tics * 4096 per Tic: 4096 / 65536 * 70
const VICTORY_CAM_SLIDE_SPEED: f32 = 4.375;

// Camera Turn Rate. Original tics * 3 Degrees per Tic
const VICTORY_CAM_SPIN_DEG_PER_SEC: f32 = 210.0;

// Absolute Yaw the Camera Turns To
//
// The Original Turns to a FIXED Compass Heading of 270 Degrees, Which Is Due South.
// It Is Not "Turn Around" and It Is Not "Face Wherever BJ Is". Bevy's Forward Is
// rotation * NEG_Z = (-sin yaw, 0, -cos yaw), so Facing +Z Requires -cos yaw = 1,
// Giving yaw = PI
const VICTORY_CAM_YAW: f32 = std::f32::consts::PI;

// BJ's Eye-Level Offset Off the Floor for His Billboard
const BJ_GROUND_Y: f32 = 0.40;

// BJ's Run Rate. Original BJRUNSPEED 2048 per Tic: 2048 / 65536 * 70
const BJ_RUN_SPEED: f32 = 2.1875;

// BJ's Rate During the Jump. Original BJJUMPSPEED 680 per Tic: 680 / 65536 * 70
// He Keeps Closing on the Camera Through the Jump, Just Far More Slowly
const BJ_JUMP_SPEED: f32 = 0.726_318_4;

// Tiles BJ Runs Before Jumping
//
// The Original Sets temp1 = 6, but the Spawn Snap Consumes One Count Without Moving
// Him (See the Comment at His Spawn Site), so Five Tiles Are Actually Travelled
//
// TUNED SHORTER THAN THE ORIGINAL, on Purpose. At the Faithful 5.0 He Finishes About
// 1.25 Tiles From the Lens, Which Framed Him Too Tightly Here to Read as a Figure --
// Wolfenstein Presented 320x200 on a 4:3 Screen With Non-Square Pixels and a Fixed
// Field of View, So the Same World Distance Does Not Frame the Same Way in Davenstein
//
// This Is the Right Knob to Turn Rather Than VICTORY_CAM_SLIDE_TILES: What the Shot
// Actually Depends On Is the GAP Between Camera and Actor, and Shortening BJ's Run
// Cannot Push the Camera Backwards Into Whatever Geometry Sits North of the Exit
// Alcove. Sliding the Camera Further Would Buy the Same Framing at the Cost of
// Needing More Guaranteed Clear Corridor Than the Level Author Provided
//
// The Arithmetic, if This Needs Dialling Again
//   jump starts at   tz + 1 - BJ_RUN_TILES
//   camera rests at  tz - VICTORY_CAM_SLIDE_TILES   (tz - 5.6875)
//   he closes a further BJ_JUMP_SPEED * 3 * BJ_JUMP_FRAME_SECS = 0.436 tiles
//   final gap = (1 - BJ_RUN_TILES) + 5.6875 - 0.436
// Every 0.5 Removed From BJ_RUN_TILES Buys 0.5 Tiles of Distance and Costs 0.23 s
// Off the Length of His Run
const BJ_RUN_TILES: f32 = 4.4;

// Run Cycle Frame Durations in Seconds, Read Off the Original State Table
// s_bjrun1 Holds SPR_BJ_W1 for 12 Tics and s_bjrun1s Holds the SAME Sprite for 3
// More, so W1 Is On Screen for 15 Tics; W2 for 8; W3 for 12 + 3 = 15; W4 for 8.
// One Full Cycle Is 46 Tics, About 0.657 Seconds
const BJ_RUN_FRAME_SECS: [f32; 4] = [15.0 / 70.0, 8.0 / 70.0, 15.0 / 70.0, 8.0 / 70.0];

// Jump Frames 1 Through 3 Each Hold 14 Tics (s_bjjump1 .. s_bjjump3)
const BJ_JUMP_FRAME_SECS: f32 = 14.0 / 70.0;

// s_bjjump4 Sits on the Final Pose for 300 Tics Before T_BJDone Ends the Sequence
const BJ_DONE_HOLD_SECS: f32 = 300.0 / 70.0;

// Marks the Player Entity as Owned by the Victory Camera
//
// Present for Exactly as Long as the Camera Is Scripted. 'victory_spin' Queries for
// It, so the System Is a No-Op With Zero Cost Whenever the Component Is Absent and
// Needs No Run Condition of Its Own
#[derive(Component, Clone, Copy)]
struct VictoryCam {
	// Absolute World Z the Camera Slides North To
	// Frozen at Trigger Time, Never Recomputed. See the Comment at the Insert Site
	target_z: f32,
}

#[derive(Component)]
struct DeathCamLabelUi;

#[allow(dead_code)]
enum EpisodeEndFlowPhase {
	Running(Option<EpisodeEndResult>),
	Finish(EpisodeEndResult),
}

impl Default for EpisodeEndFlowPhase {
	fn default() -> Self {
		Self::Running(None)
	}
}

fn world_ready(
	map: Option<Res<MapGrid>>,
	plane1: Option<Res<WolfPlane1>>,
) -> bool {
	map.is_some() && plane1.is_some()
}

#[derive(Resource, Default)]
pub struct EpisodeEndFlow {
	phase: EpisodeEndPhase,
}

#[derive(Default)]
enum EpisodeEndPhase {
	#[default]
	Inactive,
	BjCutscene(BjCutscene),
	DeathCam(DeathCam),
	Finish(EpisodeEndResult),
}

#[derive(Clone, Copy, Resource)]
#[allow(dead_code)]
pub struct EpisodeEndResult {
	pub episode: u8,
	pub score: u32,
}

#[derive(Component, Clone, Copy)]
struct BjBasePose {
	y: f32,
	scale: f32,
}

struct BjCutscene {
	stage: BjCutsceneStage,
	stage_timer: Timer,
	bj_entity: Entity,
	bj_material: Handle<StandardMaterial>,
	walk_frame: usize,
	jump_frame: usize,
	frame_timer: Timer,
	// World Z Where BJ Started Running, so Travelled Distance Can Be Measured
	// Against BJ_RUN_TILES. The Original Counts Tile Crossings Down From temp1;
	// Measuring Distance Is the Same Test Without Needing His Path Bookkeeping
	run_start_z: f32,
	played_yeah: bool,
	result: EpisodeEndResult,
}

// The Original Has No Turn-Then-Walk Split. victoryflag Goes Up, the Camera Starts
// Clamping Toward Its Targets, and BJ Starts Running, All on the Same Tic. The Spin
// Takes at Most 0.86 Seconds and the Slide About 1.3, so They Simply Finish When
// They Finish. Collapsing the Old Turning and Walking Stages Into One Removes the
// Timed Hand-Off That Had to Guess How Long the Camera Needed
#[derive(Clone, Copy, PartialEq, Eq)]
enum BjCutsceneStage {
	Running,
	Jumping,
	Done,
}

struct DeathCam {
	stage: DeathCamStage,
	boss_e: Entity,
	kind: DeathCamBossKind,
	replay_requested: bool,
	saw_dying: bool,
	elapsed: f32,
	duration: f32,
	end_yaw: f32,
	end_pitch: f32,
    kill_pos: Vec3,
    replay_pos_set: bool,
	result: EpisodeEndResult,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeathCamStage {
	WaitForCorpse,
	ShowingReplayIntro,
	Replaying,
	Holding,
}

#[derive(Clone, Copy)]
enum DeathCamBossKind {
	Hitler,
	Schabbs,
	Otto,
	General,
}

#[derive(Component)]
pub(crate) struct BossDeathReplayIntro;

#[derive(Resource)]
pub(crate) struct BossDeathReplayIntroTimer {
    pub timer: Timer,
}

impl Default for BossDeathReplayIntroTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(2.5, TimerMode::Once),
        }
    }
}

pub(crate) fn spawn_boss_death_replay_intro(
    commands: &mut Commands,
    win_w: f32,
    win_h: f32,
) {
    const BASE_HUD_H: f32 = 44.0;
    const HUD_W: f32 = 320.0;
    
    let hud_scale = (win_w / HUD_W).floor().max(1.0);
    let hud_h = (BASE_HUD_H * hud_scale).round();
    let view_h = (win_h - hud_h).max(0.0);

    commands
        .spawn((
            BossDeathReplayIntro,
            ZIndex(950),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(view_h),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.00, 0.25, 0.25)),
        ))
        .with_children(|root| {
            root.spawn((
                LevelEndBitmapText {
                    text: "LET'S SEE THAT AGAIN!".to_string(),
                    scale: 1.0,
                },
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
            ));
        });

    commands.insert_resource(BossDeathReplayIntroTimer::default());
}

pub(crate) fn tick_boss_death_replay_intro(
	mut commands: Commands,
	time: Res<Time>,
	timer: Option<ResMut<BossDeathReplayIntroTimer>>,
	mut flow: ResMut<EpisodeEndFlow>,
	q_intro: Query<Entity, With<BossDeathReplayIntro>>,
) {
	let Some(mut timer) = timer else { return; };
	
	timer.timer.tick(time.delta());

	if timer.timer.just_finished() {
		// Clean up the intro screen
		for entity in q_intro.iter() {
			commands.entity(entity).despawn();
		}
		commands.remove_resource::<BossDeathReplayIntroTimer>();

		// NOW transition to Replaying stage
		if let EpisodeEndPhase::DeathCam(cam) = &mut flow.phase {
			if cam.stage == DeathCamStage::ShowingReplayIntro {
				cam.stage = DeathCamStage::Replaying;
			}
		}
	}
}

fn deathcam_pos_ok(grid: &MapGrid, pos: Vec3) -> bool {
	let tx = (pos.x + 0.5).floor() as i32;
	let tz = (pos.z + 0.5).floor() as i32;

	if tx < 0 || tz < 0 || tx >= grid.width as i32 || tz >= grid.height as i32 {
		return false;
	}

	match grid.tile(tx as usize, tz as usize) {
		davelib::map::Tile::Wall => false,
		davelib::map::Tile::DoorClosed => false,
		_ => true,
	}
}

fn deathcam_pick_replay_pos(
	grid: &MapGrid,
	boss_pos: Vec3,
	kill_pos: Vec3,
	cam_y: f32,
	min_dist_tiles: f32,
	step_tiles: f32,
	max_dist_tiles: f32,
) -> Vec3 {
	let mut dir = boss_pos - kill_pos;
	dir.y = 0.0;

	let mut dir = dir.normalize_or_zero();
	if dir.length_squared() < 1e-6 {
		dir = Vec3::new(0.0, 0.0, 1.0);
	}

	let mut dist = min_dist_tiles;
	while dist <= max_dist_tiles {
		let mut p = boss_pos - dir * dist;
		p.y = cam_y;

		if deathcam_pos_ok(grid, p) {
			return p;
		}

		dist += step_tiles;
	}

	let mut p = boss_pos - dir * min_dist_tiles;
	p.y = cam_y;
	p
}

fn start_death_cam(
	mut flow: ResMut<EpisodeEndFlow>,
	current_level: Res<CurrentLevel>,
	hud: Res<HudState>,
	q_dead_boss: Query<
		(
			Entity,
			&Transform,
			Option<&davelib::enemies::Hitler>,
			Option<&davelib::enemies::Schabbs>,
			Option<&davelib::enemies::Otto>,
			Option<&davelib::enemies::General>,
		),
		(With<davelib::episode_end::DeathCamBoss>, Added<davelib::actors::Dead>),
	>,
	q_player: Query<(Entity, &Transform), With<Player>>,
	mut commands: Commands,
) {
	if !matches!(flow.phase, EpisodeEndPhase::Inactive) {
		return;
	}

	let Some((boss_e, _boss_tr, hitler, schabbs, otto, general)) = q_dead_boss.iter().next() else {
		return;
	};

	let on_floor_9 = current_level.0.floor_number() == 9;
	let is_hans_or_gretel = matches!(current_level.0, LevelId::E1M9 | LevelId::E5M9);
	if !on_floor_9 || is_hans_or_gretel {
		return;
	}

	let kind = if hitler.is_some() {
		DeathCamBossKind::Hitler
	} else if schabbs.is_some() {
		DeathCamBossKind::Schabbs
	} else if otto.is_some() {
		DeathCamBossKind::Otto
	} else if general.is_some() {
		DeathCamBossKind::General
	} else {
		return;
	};

	let Some((player_e, player_tr)) = q_player.iter().next() else {
		return;
	};

	// The Death Cam Aims DOWN at a Corpse, so It Needs the View to Itself. Without
	// This Marker 'level_pitch_without_mouselook' Snaps Its Tilt Back to the Horizon on
	// Every Frame for Anyone Playing With Mouselook Off, and the Replay Stares at a Wall
	commands.entity(player_e).insert(ScriptedCamera);

	let episode = current_level.0.episode() as u8;
	let result = EpisodeEndResult {
		episode,
		score: hud.score as u32,
	};

	let (yaw, pitch, _roll) = player_tr.rotation.to_euler(EulerRot::YXZ);

	flow.phase = EpisodeEndPhase::DeathCam(DeathCam {
		stage: DeathCamStage::WaitForCorpse,
		boss_e,
		kind,
		kill_pos: player_tr.translation,
		replay_pos_set: false,
		replay_requested: false,
		saw_dying: false,
		elapsed: 0.0,
		duration: 0.0,
		end_yaw: yaw,
		end_pitch: pitch,
		result,
	});
}

fn tick_death_cam(
	mut commands: Commands,
	mut flow: ResMut<EpisodeEndFlow>,
	time: Res<Time>,
	assets: Res<AssetServer>,
	grid: Option<Res<MapGrid>>,
	mut lock: ResMut<PlayerControlLock>,
	q_windows: Query<&Window, With<PrimaryWindow>>,
	mut q_player: Query<&mut Transform, With<Player>>,
	// Deliberately Separate Queries Rather Than One Wide Tuple. They Touch Different
	// Components on the Same Entity, Which Is Conflict-Free, and Keeping Them Apart
	// Leaves This Function's Existing Borrow Flow Exactly As It Was
	mut q_look: Query<&mut LookAngles, With<Player>>,
	mut q_interp: Query<&mut PlayerRenderInterp, With<Player>>,
	q_deathcam_label: Query<Entity, With<DeathCamLabelUi>>,
	q_hitler: Query<
		(Option<&davelib::enemies::HitlerCorpse>, Option<&davelib::enemies::HitlerDying>, &Transform),
		(With<davelib::enemies::Hitler>, Without<Player>),
	>,
	q_schabbs: Query<
		(Option<&davelib::enemies::SchabbsCorpse>, Option<&davelib::enemies::SchabbsDying>, &Transform),
		(With<davelib::enemies::Schabbs>, Without<Player>),
	>,
	q_otto: Query<
		(Option<&davelib::enemies::OttoCorpse>, Option<&davelib::enemies::OttoDying>, &Transform),
		(With<davelib::enemies::Otto>, Without<Player>),
	>,
	q_general: Query<
		(Option<&davelib::enemies::GeneralCorpse>, Option<&davelib::enemies::GeneralDying>, &Transform),
		(With<davelib::enemies::General>, Without<Player>),
	>,
) {
	let EpisodeEndPhase::DeathCam(cam) = &mut flow.phase else {
		return;
	};

	let Some(grid) = grid.as_ref() else {
		return;
	};

	const DEATH_CAM_MAX_PITCH: f32 = 0.35;
	const DEATH_CAM_PRE_REPLAY_SECS: f32 = 0.90;
	const DEATH_CAM_POST_REPLAY_SECS: f32 = 1.10;
	const REPLAY_MIN_DIST_TILES: f32 = 2.30;
	const REPLAY_STEP_TILES: f32 = 0.0625;
	const REPLAY_MAX_DIST_TILES: f32 = 8.0;
	const REPLAY_SLOW_MUL: u8 = 3;

	let ensure_label = |commands: &mut Commands| {
		if q_deathcam_label.iter().next().is_some() {
			return;
		}

		const LABEL_TEX_W: f32 = 546.0;
		const LABEL_TEX_H: f32 = 150.0;
		const LABEL_SCALE: f32 = 0.75;

		const LABEL_TOP_PX: f32 = 20.0;

		let w = LABEL_TEX_W * LABEL_SCALE;
		let h = LABEL_TEX_H * LABEL_SCALE;

		let tex: Handle<Image> = assets.load("textures/ui/episode_end/death_cam_text.png");

		commands
			.spawn((
				DeathCamLabelUi,
				ZIndex(1000),
				Node {
					width: Val::Percent(100.0),
					height: Val::Px(h),
					position_type: PositionType::Absolute,
					top: Val::Px(LABEL_TOP_PX),
					left: Val::Px(0.0),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					..default()
				},
			))
			.with_children(|c| {
				c.spawn((
					ImageNode::new(tex),
					Node {
						width: Val::Px(w),
						height: Val::Px(h),
						..default()
					},
				));
			});
	};

	let Some(mut player_tr) = q_player.iter_mut().next() else {
		let result = cam.result;
		flow.phase = EpisodeEndPhase::Finish(result);
		return;
	};

	let boss_state = |cam: &DeathCam| -> Option<(Vec3, bool, bool)> {
		match cam.kind {
			DeathCamBossKind::Hitler => {
				let (corpse, dying, tr) = q_hitler.get(cam.boss_e).ok()?;
				Some((tr.translation, corpse.is_some(), dying.is_some()))
			}
			DeathCamBossKind::Schabbs => {
				let (corpse, dying, tr) = q_schabbs.get(cam.boss_e).ok()?;
				Some((tr.translation, corpse.is_some(), dying.is_some()))
			}
			DeathCamBossKind::Otto => {
				let (corpse, dying, tr) = q_otto.get(cam.boss_e).ok()?;
				Some((tr.translation, corpse.is_some(), dying.is_some()))
			}
			DeathCamBossKind::General => {
				let (corpse, dying, tr) = q_general.get(cam.boss_e).ok()?;
				Some((tr.translation, corpse.is_some(), dying.is_some()))
			}
		}
	};

	let Some((boss_pos, boss_is_corpse, boss_is_dying)) = boss_state(cam) else {
		let result = cam.result;
		flow.phase = EpisodeEndPhase::Finish(result);
		return;
	};

	match cam.stage {
		DeathCamStage::WaitForCorpse => {
			if !boss_is_corpse {
				cam.elapsed = 0.0;
				return;
			}

			cam.elapsed += time.delta_secs();
			if cam.elapsed < DEATH_CAM_PRE_REPLAY_SECS {
				return;
			}

			cam.elapsed = 0.0;
			cam.replay_requested = false;
			cam.saw_dying = false;

			if !cam.replay_pos_set {
				let cam_y = player_tr.translation.y;

				let replay_pos = deathcam_pick_replay_pos(
					grid,
					boss_pos,
					cam.kill_pos,
					cam_y,
					REPLAY_MIN_DIST_TILES,
					REPLAY_STEP_TILES,
					REPLAY_MAX_DIST_TILES,
				);

				let to = boss_pos - replay_pos;
				let flat_len2 = to.x * to.x + to.z * to.z;

				let (end_yaw, end_pitch) = if flat_len2 <= 1e-6 {
					let (y, p, _r) = player_tr.rotation.to_euler(EulerRot::YXZ);
					(y, p.clamp(-DEATH_CAM_MAX_PITCH, DEATH_CAM_MAX_PITCH))
				} else {
					let dir = to.normalize();
					let yaw = (-dir.x).atan2(-dir.z);

					let pitch_raw = dir.y.atan2((dir.x * dir.x + dir.z * dir.z).sqrt());
					let pitch = pitch_raw.clamp(-DEATH_CAM_MAX_PITCH, DEATH_CAM_MAX_PITCH);

					(yaw, pitch)
				};

				player_tr.translation = replay_pos;
				player_tr.rotation = Quat::from_euler(EulerRot::YXZ, end_yaw, end_pitch, 0.0);

				// This Is a CUT, Not a Move, so the Interpolation Window Has to Come With
				// It. 'apply_player_render_interp' Blends prev Toward curr Every Rendered
				// Frame; Left Alone, prev Still Holds Wherever the Player Was Standing When
				// the Boss Died and the Camera Sweeps From There to the Replay Spot Over One
				// Fixed Step, Skidding Through Whatever Walls Lie Between
				if let Some(mut interp) = q_interp.iter_mut().next() {
					interp.snap_to(replay_pos);
				}

				// Keep LookAngles in Agreement With the Framing We Just Chose
				//
				// ScriptedCamera Already Stops Anything Rebuilding the Rotation Mid-Replay,
				// but LookAngles Is What 'apply_look' Resumes From Once Control Returns. If
				// It Still Held the Pre-Replay Angles, the First Mouse Movement After the
				// Sequence Would Snap the View Back to Where the Player Was Looking When the
				// Boss Fell Instead of Continuing From What They Are Actually Seeing
				if let Some(mut look) = q_look.iter_mut().next() {
					look.set_view(end_yaw, end_pitch);
				}

				cam.end_yaw = end_yaw;
				cam.end_pitch = end_pitch;

				cam.replay_pos_set = true;
			}

			// Get window dimensions
			let (win_w, win_h) = if let Some(win) = q_windows.iter().next() {
				(win.resolution.width(), win.resolution.height())
			} else {
				(320.0, 200.0) // fallback
			};

			spawn_boss_death_replay_intro(&mut commands, win_w, win_h);

			lock.0 = true;
			// Transition to new stage
			cam.stage = DeathCamStage::ShowingReplayIntro;
		}

		DeathCamStage::ShowingReplayIntro => {
			// Wait, tick_boss_death_replay_intro system will advance
			lock.0 = true;
		}

		DeathCamStage::Replaying => {
			ensure_label(&mut commands);
			lock.0 = true;
			player_tr.rotation = Quat::from_euler(EulerRot::YXZ, cam.end_yaw, cam.end_pitch, 0.0);

			if !cam.replay_requested {
				match cam.kind {
					DeathCamBossKind::Hitler => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::HitlerCorpse>();
						commands
							.entity(cam.boss_e)
							.insert(davelib::enemies::HitlerDying { frame: 0, tics: 0 });
					}
					DeathCamBossKind::Schabbs => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::SchabbsCorpse>();
						commands
							.entity(cam.boss_e)
							.insert(davelib::enemies::SchabbsDying { frame: 0, tics: 0 });
					}
					DeathCamBossKind::Otto => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::OttoCorpse>();
						commands
							.entity(cam.boss_e)
							.insert(davelib::enemies::OttoDying { frame: 0, tics: 0 });
					}
					DeathCamBossKind::General => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::GeneralCorpse>();
						commands
							.entity(cam.boss_e)
							.insert(davelib::enemies::GeneralDying { frame: 0, tics: 0 });
					}
				}

				commands
					.entity(cam.boss_e)
					.insert(davelib::enemies::DeathCamReplaySlow(REPLAY_SLOW_MUL));

				cam.replay_requested = true;
				return;
			}

			if !cam.saw_dying && boss_is_dying {
				cam.saw_dying = true;
			}

			if cam.saw_dying && boss_is_corpse && !boss_is_dying {
				cam.elapsed = 0.0;
				cam.duration = DEATH_CAM_POST_REPLAY_SECS;
				cam.stage = DeathCamStage::Holding;
			}
		}

		DeathCamStage::Holding => {
			ensure_label(&mut commands);

			player_tr.rotation = Quat::from_euler(EulerRot::YXZ, cam.end_yaw, cam.end_pitch, 0.0);

			cam.elapsed += time.delta_secs();
			if cam.elapsed >= cam.duration {
				let result = cam.result;
				flow.phase = EpisodeEndPhase::Finish(result);
			}
		}
	}
}

fn episode_end_finish_to_ui(
    mut commands: Commands,
    mut flow: ResMut<EpisodeEndFlow>,
    mut splash_step: ResMut<SplashStep>,
    mut music_mode: ResMut<MusicMode>,
    mut lock: ResMut<PlayerControlLock>,
    mut name_entry: ResMut<davelib::high_score::NameEntryState>,
    q_deathcam_label: Query<(Entity, Option<&Children>), With<DeathCamLabelUi>>,
    q_scripted_cam: Query<Entity, With<ScriptedCamera>>,
) {
    let EpisodeEndPhase::Finish(result) = flow.phase else {
        return;
    };

    // Hand the View Back Here Rather Than at Each Cutscene's Own Exit
    //
    // Both Sequences Reach Finish, and the Death Cam Reaches It From Several Places -
    // Normal Completion, a Missing Player, a Boss Entity That Vanished. Releasing at
    // the One Funnel They All Pass Through Means No Bail-Out Path Can Leave the Player
    // Permanently Unable to Aim Vertically
    for player in q_scripted_cam.iter() {
        commands.entity(player).remove::<ScriptedCamera>();
    }

    for (e, kids) in q_deathcam_label.iter() {
        if let Some(kids) = kids {
            for k in kids.iter() {
                commands.entity(k).despawn();
            }
        }
        commands.entity(e).despawn();
    }

    *splash_step = SplashStep::EpisodeVictory;
    lock.0 = true;
    music_mode.0 = MusicModeKind::Scores;

    name_entry.active = false;
    name_entry.name.clear();
    name_entry.cursor_pos = 0;
    name_entry.rank = 0;
    name_entry.score = 0;
    name_entry.episode = result.episode;

    flow.phase = EpisodeEndPhase::Inactive;
}

fn start_bj_cutscene(
	mut commands: Commands,
	mut flow: ResMut<EpisodeEndFlow>,
	mut lock: ResMut<PlayerControlLock>,
	current_level: Res<CurrentLevel>,
	plane1: Res<WolfPlane1>,
	grid: Res<MapGrid>,
	hud: Res<HudState>,
	images: Res<EpisodeEndImages>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	// Read Only. This System Used to Snap the Camera to the Tile Center and Seed a
	// Dolly; the Original Never Writes player->x at All, So There Is Nothing to Move
	// Here Any More and the Victory Camera Owns Every Later Write
	q_player: Query<(Entity, &Transform), With<Player>>,
) {
	if !matches!(flow.phase, EpisodeEndPhase::Inactive) {
		return;
	}

	if lock.0 {
		return;
	}

	let is_hans_or_gretel = matches!(current_level.0, LevelId::E1M9 | LevelId::E5M9);
	if !is_hans_or_gretel {
		return;
	}

	let Some((player_e, player_tr)) = q_player.iter().next() else {
		return;
	};

	let Some((tx, tz)) = world_to_tile(player_tr.translation) else {
		return;
	};

	let idx = (tz as usize) * (grid.width as usize) + (tx as usize);
	let Some(code) = plane1.0.get(idx).copied() else {
		return;
	};

	if code != 99 {
		return;
	}

	lock.0 = true;

	// Freeze the Trigger Tile
	// The Original Depends on player->tilex / tiley NOT Being Updated While
	// victoryflag Is Set, Because T_Player Returns Before the Tile Update Runs. That
	// Is What Makes desty a CONSTANT. Capturing tz Once Here Reproduces That
	// Deliberately: Recomputing the Target From the Live Camera Position Every Frame
	// Would Make the Target Retreat as Fast as the Camera Chasing It, and the Slide
	// Would Never End
	let target_z = tz as f32 - VICTORY_CAM_SLIDE_TILES;

	// Note What Is NOT Here Any More: There Is No Door Scan, No Free-Run Measurement,
	// and No Chosen Retreat Direction. The Original Hardcodes Due North for the Slide
	// and an Absolute 270 Degrees for the Facing, Because the Level Author Guaranteed
	// a Clear Northward Run From Every Exit Tile. Davenstein Ships the Original Map
	// Data, so That Guarantee Still Holds, and Detecting the Geometry at Runtime Only
	// Ever Added Ways to Get It Wrong
	// ScriptedCamera Locks Out Every System That Would Second-Guess the View for the
	// Rest of the Sequence. VictoryCam Carries the Data; the Marker Carries the Authority
	commands
		.entity(player_e)
		.insert((VictoryCam { target_z }, ScriptedCamera));

	// BJ Materializes One Tile SOUTH of the Trigger Tile and Runs North to Meet the
	// Retreating Camera
	//
	// The Original Spawns Him With Tile Index (tilex, tiley + 1) but Fine Coordinates
	// Copied Straight From the Player, an Intentional Mismatch. On His First Think the
	// Path Bookkeeping Snaps His Fine Position to That Tile's Center, and Because the
	// Snap Runs Through the Same Loop Body as a Real Tile Crossing It Consumes One of
	// His Six Tile Counts Without Advancing Him. Spawning Him Directly at the Center
	// Reaches the Same State One Frame Earlier, Which Is Exactly Why He Runs
	// BJ_RUN_TILES (5) Below and Not 6. Change One Without the Other and He Overshoots
	// the Camera by a Tile
	let bj_start_z = tz as f32 + 1.0;
	let bj_pos = Vec3::new(tx as f32, BJ_GROUND_Y, bj_start_z);

	let bj_mesh = meshes.add(Rectangle::new(0.95, 1.30));
	let bj_mat = materials.add(StandardMaterial {
		base_color_texture: Some(images.bj_victory_walk[0].clone()),
		alpha_mode: AlphaMode::Mask(0.5),
		unlit: true,
		double_sided: true,
		..default()
	});

	const BJ_SCALE: f32 = 0.65;

	let bj_entity = commands
		.spawn((
			Name::new("BJ Victory"),
			Mesh3d(bj_mesh),
			MeshMaterial3d(bj_mat.clone()),
			Transform::from_translation(bj_pos).with_scale(Vec3::splat(BJ_SCALE)),
			BjBasePose { y: bj_pos.y, scale: BJ_SCALE },
			Visibility::Visible,
		))
		.id();

	let episode = current_level.0.episode() as u8;

	let result = EpisodeEndResult {
		episode,
		score: hud.score as u32,
	};

	flow.phase = EpisodeEndPhase::BjCutscene(BjCutscene {
		stage: BjCutsceneStage::Running,
		// Only Used for the Final Held Pose, and Re-Armed on Entry to Done. Seeded
		// With the Real Duration Rather Than Zero so No Code Path Can Ever Meet a
		// Zero-Length Timer
		stage_timer: Timer::from_seconds(BJ_DONE_HOLD_SECS, TimerMode::Once),
		bj_entity,
		bj_material: bj_mat,
		walk_frame: 0,
		jump_frame: 0,
		frame_timer: Timer::from_seconds(BJ_RUN_FRAME_SECS[0], TimerMode::Once),
		run_start_z: bj_start_z,
		played_yeah: false,
		result,
	});
}

fn tick_bj_cutscene(
	mut commands: Commands,
	time: Res<Time>,
	images: Res<EpisodeEndImages>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut sfx: MessageWriter<PlaySfx>,
	mut flow: ResMut<EpisodeEndFlow>,
	// Read Only for the Camera. This System Reads the Camera Position to Aim BJ's
	// Billboard and Nothing More; the Entity Is Kept Solely to Remove VictoryCam When
	// the Sequence Ends
	q_player: Query<(Entity, &Transform), With<Player>>,
	mut q_bj: Query<(&mut Transform, &BjBasePose), Without<Player>>,
) {
	let Some((player_e, player_tr)) = q_player.iter().next() else {
		return;
	};

	// Notice There Is No Camera Write Anywhere in This System. The Victory Camera Is
	// Owned Entirely by 'victory_spin' in FixedUpdate. Two Systems Writing the Player
	// Transform From Two Different Schedules Is the Bug That Broke This Sequence, so
	// Anything Camera-Shaped Belongs There, Not Here
	let (bj_entity, stage, jump_frame, run_start_z, finish_result) = {
		let EpisodeEndPhase::BjCutscene(cut) = &mut flow.phase else {
			return;
		};

		let mut finish_result: Option<EpisodeEndResult> = None;

		match cut.stage {
			BjCutsceneStage::Running => {
				// Per-Frame Durations, Because the Original Run Cycle Is Not Uniform:
				// Two of the Four Sprites Are Held Roughly Twice as Long as the Others
				cut.frame_timer.tick(time.delta());
				if cut.frame_timer.just_finished() {
					cut.walk_frame = (cut.walk_frame + 1) % 4;
					cut.frame_timer = Timer::from_seconds(
						BJ_RUN_FRAME_SECS[cut.walk_frame],
						TimerMode::Once,
					);

					if let Some(mut mat) = materials.get_mut(&cut.bj_material) {
						mat.base_color_texture = Some(images.bj_victory_walk[cut.walk_frame].clone());
					}
				}
			}

			BjCutsceneStage::Jumping => {
				cut.frame_timer.tick(time.delta());
				if cut.frame_timer.just_finished() {
					cut.jump_frame += 1;

					if cut.jump_frame >= 3 {
						// Frame 4 (Index 3) Is the Held Pose. s_bjjump4 Sits on It for
						// 300 Tics With No Think, so BJ Also Stops Moving Here
						if let Some(mut mat) = materials.get_mut(&cut.bj_material) {
							mat.base_color_texture = Some(images.bj_victory_jump[3].clone());
						}

						cut.jump_frame = 3;
						cut.stage = BjCutsceneStage::Done;
						cut.stage_timer = Timer::from_seconds(BJ_DONE_HOLD_SECS, TimerMode::Once);
					} else {
						cut.frame_timer = Timer::from_seconds(BJ_JUMP_FRAME_SECS, TimerMode::Once);

						if let Some(mut mat) = materials.get_mut(&cut.bj_material) {
							mat.base_color_texture = Some(images.bj_victory_jump[cut.jump_frame].clone());
						}

						// The Yell Belongs to the SECOND Jump Frame, Not the First:
						// T_BJYell Is the Action on s_bjjump2, and an Action Fires as Its
						// State Is Entered. Playing It on Frame 1 Lands It a Fifth of a
						// Second Early and Reads as Out of Sync With the Leap
						if cut.jump_frame == 1 && !cut.played_yeah {
							cut.played_yeah = true;
							sfx.write(PlaySfx {
								kind: SfxKind::EpisodeVictoryYea,
								pos: Vec3::ZERO,
							});
						}
					}
				}
			}

			BjCutsceneStage::Done => {
				cut.stage_timer.tick(time.delta());
				if cut.stage_timer.just_finished() {
					finish_result = Some(cut.result);
				}
			}
		}

		(
			cut.bj_entity,
			cut.stage,
			cut.jump_frame,
			cut.run_start_z,
			finish_result,
		)
	};

	if let Some(result) = finish_result {
		commands.entity(bj_entity).despawn();
		// Hand the Camera Back. Once VictoryCam Is Gone 'victory_spin' Stops Matching
		// and the Normal Interpolation Path Owns the Transform Again With No Gap
		commands.entity(player_e).remove::<VictoryCam>();
		flow.phase = EpisodeEndPhase::Finish(result);
		return;
	}

	let cam_pos = player_tr.translation;

	if let Ok((mut bj_tr, base_pose)) = q_bj.get_mut(bj_entity) {
		// BJ Runs Due NORTH, Always. He Is Never Steered Toward the Camera
		//
		// The Original Sets dir = north Once in SpawnBJVictory and Never Re-Aims Him.
		// Homing on the Camera Looks Equivalent and Is Not: the Camera Is Itself
		// Retreating North, so a Homing Vector Curves His Path and Walks Him Off the
		// One Axis the Level Geometry Guarantees Is Clear, Straight Into a Wall on Any
		// Map Where the Exit Alcove Is Not Perfectly Centred
		let speed = match stage {
			BjCutsceneStage::Running => BJ_RUN_SPEED,
			BjCutsceneStage::Jumping => BJ_JUMP_SPEED,
			// s_bjjump4 Has No Think Function, so the Held Pose Does Not Advance
			BjCutsceneStage::Done => 0.0,
		};

		bj_tr.translation.z -= speed * time.delta_secs();

		// Billboard Facing
		//
		// The Original's BJ States All Carry rotate = false, meaning He Is a Single
		// Non-Directional Sprite Presented Flat to the View, so Turning the Quad to
		// Face the Camera Is the Faithful Behavior Rather Than a Shortcut
		let mut to_cam = cam_pos - bj_tr.translation;
		to_cam.y = 0.0;
		let to_cam = to_cam.normalize_or_zero();

		if to_cam != Vec3::ZERO {
			// A Bevy Rectangle Mesh Faces +Z, so Recovering Yaw From a Desired Facing
			// Is atan2(x, z) Here. That Is NOT the Camera's Inverse, Which Is
			// atan2(-x, -z) Because a Camera Looks Down NEG_Z. Two Genuinely Different
			// Conventions in One File: Mixing Them Is What Left the Old Victory Camera
			// Facing 180 Degrees Away From BJ Whenever the Sequence Ran Along the X Axis
			bj_tr.rotation = Quat::from_rotation_y(to_cam.x.atan2(to_cam.z));
		}

		// Jump Arc, Scaled With the Sprite so It Reads the Same at Any Billboard Size
		let jf = jump_frame.min(3);

		const BJ_JUMP_Y_OFFSETS: [f32; 4] = [0.00, 0.03, 0.05, 0.03];

		let raw_off = match stage {
			BjCutsceneStage::Jumping => BJ_JUMP_Y_OFFSETS[jf],
			BjCutsceneStage::Done => BJ_JUMP_Y_OFFSETS[3],
			_ => 0.0,
		};

		bj_tr.translation.y = base_pose.y + raw_off * base_pose.scale;

		// Distance Test Replaces the Original's Tile-Crossing Countdown
		//
		// Re-Borrowing the Phase Here Rather Than Inside the Match Above Is Deliberate:
		// the Decision Depends on BJ's Position, Which Is Only Known After He Has Moved
		if matches!(stage, BjCutsceneStage::Running)
			&& (run_start_z - bj_tr.translation.z) >= BJ_RUN_TILES
		{
			if let EpisodeEndPhase::BjCutscene(cut) = &mut flow.phase {
				cut.stage = BjCutsceneStage::Jumping;
				cut.jump_frame = 0;
				cut.frame_timer = Timer::from_seconds(BJ_JUMP_FRAME_SECS, TimerMode::Once);

				if let Some(mut mat) = materials.get_mut(&cut.bj_material) {
					mat.base_color_texture = Some(images.bj_victory_jump[0].clone());
				}
			}
		}
	}
}

fn world_to_tile(pos: Vec3) -> Option<(u32, u32)> {
	let tx = (pos.x + 0.5).floor() as i32;
	let tz = (pos.z + 0.5).floor() as i32;

	if tx < 0 || tz < 0 {
		return None;
	}

	Some((tx as u32, tz as u32))
}

// Step an Angle Toward a Target by at Most max_step, Along the Shorter Arc
//
// This Replaces the Old Fixed-Duration Angle Lerp. The Original Does Not Interpolate
// Over a Duration at All: It Adds or Subtracts a Constant 3 Degrees per Tic and
// Clamps on Arrival, So the Turn Takes However Long It Takes and the Camera Slide
// Runs Independently Alongside It. Snapping to the Target Once Inside One Step Is
// the Same Clamp the Original Performs
//
// DEVIATION, Deliberate: the Original Compares Raw Angle Numbers (if angle > 270
// Decrease, Else Increase), Which on Wolf's 0..359 Counterclockwise Scale Can Take
// the LONG Way Round -- Approaching the Exit Facing Slightly North of East Produces
// a 260 Degree Whirl Rather Than a 100 Degree Turn. Shorter-Arc Turning Is Used Here
// Because It Is What Players Expect and What This Sequence Already Did Before. Swap
// the Two Branches Below for a Literal Numeric Match if the Whirl Is Wanted Back
fn step_angle_toward(from: f32, to: f32, max_step: f32) -> f32 {
	let tau = std::f32::consts::TAU;
	let mut delta = (to - from) % tau;

	if delta > std::f32::consts::PI {
		delta -= tau;
	} else if delta < -std::f32::consts::PI {
		delta += tau;
	}

	if delta.abs() <= max_step {
		to
	} else {
		from + delta.signum() * max_step
	}
}

// Drive the Episode-End Victory Camera. This Is VictorySpin From WL_AGENT.C
//
// Runs in FixedUpdate so One Call Is One 70 Hz Tic, Matching the Original Exactly
// and Keeping the Write Inside the Snapshot Window That
// 'apply_player_render_interp' Reads. See the Registration Comment in the Plugin for
// Why Anything Else Breaks
//
// Both Motions Are Independent Rate-Limited Clamps Toward Absolute Targets, Not a
// Timed Animation. The Turn Finishes Within 0.86 Seconds Worst Case and the Slide
// Takes About 1.3, and Neither Waits on the Other
fn victory_spin(
	fixed_time: Res<Time<Fixed>>,
	mut q_player: Query<(&mut Transform, &mut LookAngles, &VictoryCam), With<Player>>,
) {
	let dt = fixed_time.delta_secs();

	for (mut transform, mut look, cam) in &mut q_player {
		// Facing Is Written Through LookAngles and Only THEN Composed Into the
		// Rotation. Writing Transform.rotation Alone Leaves LookAngles Holding the
		// Player's Pre-Cutscene Yaw, and 'level_pitch_without_mouselook' -- Which Is
		// Deliberately Not Lock-Gated -- Rebuilds the Rotation From That Stale Value
		// Every Frame, Silently Undoing the Turn Whenever Mouselook Is Off
		let max_step = VICTORY_CAM_SPIN_DEG_PER_SEC.to_radians() * dt;
		let yaw = step_angle_toward(look.yaw(), VICTORY_CAM_YAW, max_step);

		// Pitch Is Forced Level. The Original Renderer Could Not Express a Pitched
		// View at All, and Pitch Here Rides on the Player Transform, so Any Leftover
		// Tilt Would Aim the Victory Shot at the Ceiling
		look.set_view(yaw, 0.0);
		transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);

		// Slide North Only, Never South, and Never Sideways. X Is Left Untouched
		// Exactly as the Original Leaves player->x Untouched
		if transform.translation.z > cam.target_z {
			transform.translation.z -= VICTORY_CAM_SLIDE_SPEED * dt;

			if transform.translation.z < cam.target_z {
				transform.translation.z = cam.target_z;
			}
		}
	}
}
