/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

use davelib::audio::{MusicMode, MusicModeKind, PlaySfx, SfxKind};
use davelib::level::{CurrentLevel, LevelId, WolfPlane1};
use davelib::map::MapGrid;
use davelib::player::{Player, PlayerControlLock};

use crate::ui::HudState;
use crate::ui::SplashStep;
use crate::ui::EpisodeEndImages;

pub struct EpisodeEndPlugin;

impl Plugin for EpisodeEndPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<EpisodeEndFlow>()
			.add_systems(Update, start_bj_cutscene.run_if(world_ready))
			.add_systems(Update, tick_bj_cutscene)
			.add_systems(Update, start_death_cam)
			.add_systems(Update, tick_death_cam)
			.add_systems(Update, episode_end_finish_to_ui);
	}
}

#[derive(Component, Clone, Copy)]
struct BjDolly {
	start: Vec3,
	end: Vec3,
}

fn world_ready(
	map: Option<Res<MapGrid>>,
	plane1: Option<Res<WolfPlane1>>,
) -> bool {
	map.is_some() && plane1.is_some()
}

#[derive(Resource, Default)]
struct EpisodeEndFlow {
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
	walk_loops_left: u8,
	frame_timer: Timer,
	played_yeah: bool,
	result: EpisodeEndResult,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BjCutsceneStage {
	Turning,
	Walking,
	Jumping,
	Done,
}

struct DeathCam {
	elapsed: f32,
	duration: f32,
	start_yaw: f32,
	start_pitch: f32,
	end_yaw: f32,
	end_pitch: f32,
	result: EpisodeEndResult,
}

fn start_death_cam(
	mut flow: ResMut<EpisodeEndFlow>,
	mut lock: ResMut<PlayerControlLock>,
	current_level: Res<CurrentLevel>,
	hud: Res<HudState>,
	q_dead_boss: Query<&Transform, (With<davelib::episode_end::DeathCamBoss>, Added<davelib::actors::Dead>)>,
	q_player: Query<&Transform, With<Player>>,
) {
	if !matches!(flow.phase, EpisodeEndPhase::Inactive) {
		return;
	}

	let Some(boss_tr) = q_dead_boss.iter().next() else {
		return;
	};

	let on_floor_9 = current_level.0.floor_number() == 9;
	let is_hans_or_gretel = matches!(current_level.0, LevelId::E1M9 | LevelId::E5M9);
	if !on_floor_9 || is_hans_or_gretel {
		return;
	}

	let Some(player_tr) = q_player.iter().next() else {
		return;
	};

	lock.0 = true;

	let episode = current_level.0.episode() as u8;

	let result = EpisodeEndResult {
		episode,
		score: hud.score as u32,
	};

	let (start_yaw, start_pitch, _roll) = player_tr.rotation.to_euler(EulerRot::YXZ);

	let target_pos = boss_tr.translation;
	let dir = (target_pos - player_tr.translation).normalize_or_zero();

	let end_yaw = dir.x.atan2(-dir.z);
	let end_pitch = dir.y.atan2((dir.x * dir.x + dir.z * dir.z).sqrt());

	flow.phase = EpisodeEndPhase::DeathCam(DeathCam {
		elapsed: 0.0,
		duration: 1.25,
		start_yaw,
		start_pitch,
		end_yaw,
		end_pitch,
		result,
	});
}

fn tick_death_cam(
	mut flow: ResMut<EpisodeEndFlow>,
	time: Res<Time>,
	mut q_player: Query<&mut Transform, With<Player>>,
) {
	let EpisodeEndPhase::DeathCam(cam) = &mut flow.phase else {
		return;
	};

	cam.elapsed += time.delta_secs();

	let mut t = cam.elapsed / cam.duration;
	if t > 1.0 {
		t = 1.0;
	}

	// Smoothstep so it feels like a snap-pan without being instant
	let t = t * t * (3.0 - 2.0 * t);

	let yaw = lerp_angle(cam.start_yaw, cam.end_yaw, t);
	let pitch = cam.start_pitch + (cam.end_pitch - cam.start_pitch) * t;

	let Some(mut tr) = q_player.iter_mut().next() else {
		let result = cam.result;
		flow.phase = EpisodeEndPhase::Finish(result);
		return;
	};

	tr.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

	if cam.elapsed >= cam.duration {
		let result = cam.result;
		flow.phase = EpisodeEndPhase::Finish(result);
	}
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
	mut q_player: Query<(Entity, &mut Transform), With<Player>>,
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

	let Some((player_e, mut player_tr)) = q_player.iter_mut().next() else {
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

	let tx_i = tx as i32;
	let tz_i = tz as i32;

	let is_door = |t: davelib::map::Tile| matches!(
		t,
		davelib::map::Tile::DoorClosed | davelib::map::Tile::DoorOpen
	);

	let free_run = |step_x: i32, step_z: i32| -> i32 {
		let mut cx = tx_i;
		let mut cz = tz_i;
		let mut run = 0;

		for _ in 0..32 {
			let nx = cx + step_x;
			let nz = cz + step_z;

			if nx < 0 || nz < 0 || nx >= grid.width as i32 || nz >= grid.height as i32 {
				break;
			}

			if matches!(grid.tile(nx as usize, nz as usize), davelib::map::Tile::Wall) {
				break;
			}

			run += 1;
			cx = nx;
			cz = nz;
		}

		run
	};

	const DOOR_SCAN_MAX: i32 = 16;
	let scan_dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];

	let mut best_door: Option<(i32, i32, i32, i32, i32, i32)> = None;
	// (door_x, door_z, dist_to_door, run_away, away_step_x, away_step_z)

	for (sx, sz) in scan_dirs {
		for dist in 1..=DOOR_SCAN_MAX {
			let nx = tx_i + sx * dist;
			let nz = tz_i + sz * dist;

			if nx < 0 || nz < 0 || nx >= grid.width as i32 || nz >= grid.height as i32 {
				break;
			}

			let t = grid.tile(nx as usize, nz as usize);

			if matches!(t, davelib::map::Tile::Wall) {
				break;
			}

			if is_door(t) {
				let away_step_x = -sx;
				let away_step_z = -sz;
				let run_away = free_run(away_step_x, away_step_z);

				let cand = (nx, nz, dist, run_away, away_step_x, away_step_z);

				match best_door {
					None => best_door = Some(cand),
					Some((_, _, best_dist, best_run, _, _)) => {
						if dist < best_dist || (dist == best_dist && run_away > best_run) {
							best_door = Some(cand);
						}
					}
				}

				break;
			}
		}
	}

	let cam_y = player_tr.translation.y;

	const DOLLY_PAD_TILES: f32 = 0.90;
	const DOLLY_MAX: f32 = 4.35;

	let (away_dir, door_center, dolly_dist) = if let Some((door_x, door_z, _dist, run_away, away_step_x, away_step_z)) = best_door {
		let away = Vec3::new(away_step_x as f32, 0.0, away_step_z as f32).normalize_or_zero();
		let door_center = Vec3::new(door_x as f32, cam_y, door_z as f32);
		let dist = ((run_away as f32) - DOLLY_PAD_TILES).clamp(0.0, DOLLY_MAX);

		(away, door_center, dist)
	} else {
		let mut best = (0, 0, 0);
		// (step_x, step_z, run)

		for (sx, sz) in scan_dirs {
			let run = free_run(sx, sz);
			if run > best.2 {
				best = (sx, sz, run);
			}
		}

		let away = Vec3::new(best.0 as f32, 0.0, best.1 as f32).normalize_or_zero();
		let door_center = Vec3::new((tx_i - best.0) as f32, cam_y, (tz_i - best.1) as f32);
		let dist = ((best.2 as f32) - DOLLY_PAD_TILES).clamp(0.0, DOLLY_MAX);

		(away, door_center, dist)
	};

	let cam_start = Vec3::new(tx_i as f32, cam_y, tz_i as f32);
	player_tr.translation = cam_start;

	let forward_to_door = -away_dir;
	let yaw_after = forward_to_door.x.atan2(-forward_to_door.z);
	player_tr.rotation = Quat::from_euler(EulerRot::YXZ, yaw_after, 0.0, 0.0);

	let cam_end = cam_start + away_dir * dolly_dist;

	commands.entity(player_e).insert(BjDolly {
		start: cam_start,
		end: cam_end,
	});

	let bj_mesh = meshes.add(Rectangle::new(0.95, 1.30));
	let bj_mat = materials.add(StandardMaterial {
		base_color_texture: Some(images.bj_victory_walk[0].clone()),
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		double_sided: true,
		..default()
	});

	let mut bj_pos = door_center + away_dir * 1.10;
	bj_pos.y = 0.40;

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
		stage: BjCutsceneStage::Turning,
		stage_timer: Timer::from_seconds(1.70, TimerMode::Once),
		bj_entity,
		bj_material: bj_mat,
		walk_frame: 0,
		jump_frame: 0,
		walk_loops_left: 3,
		frame_timer: Timer::from_seconds(0.10, TimerMode::Once),
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
	mut q_player: Query<(Entity, &mut Transform, Option<&BjDolly>), With<Player>>,
	mut q_bj: Query<(&mut Transform, &BjBasePose), Without<Player>>,
) {
	let Some((player_e, mut player_tr, dolly)) = q_player.iter_mut().next() else {
		return;
	};

	let mut bj_entity: Option<Entity> = None;
	let mut stage = BjCutsceneStage::Done;
	let mut turning_elapsed = 0.0f32;
	let mut jump_frame = 0usize;

	let mut finish_result: Option<EpisodeEndResult> = None;

	{
		let EpisodeEndPhase::BjCutscene(cut) = &mut flow.phase else {
			return;
		};

		bj_entity = Some(cut.bj_entity);

		match cut.stage {
			BjCutsceneStage::Turning => {
				cut.stage_timer.tick(time.delta());

				if let Some(dolly) = dolly {
					let dur = cut.stage_timer.duration().as_secs_f32().max(0.0001);
					let t = (cut.stage_timer.elapsed_secs() / dur).clamp(0.0, 1.0);
					let t = t * t * (3.0 - 2.0 * t);

					player_tr.translation = dolly.start + (dolly.end - dolly.start) * t;
				}

				cut.frame_timer.tick(time.delta());
				if cut.frame_timer.just_finished() {
					cut.frame_timer.reset();

					cut.walk_frame = (cut.walk_frame + 1) % 4;

					if let Some(mat) = materials.get_mut(&cut.bj_material) {
						mat.base_color_texture = Some(images.bj_victory_walk[cut.walk_frame].clone());
					}
				}

				if cut.stage_timer.just_finished() {
					commands.entity(player_e).remove::<BjDolly>();
					cut.stage = BjCutsceneStage::Walking;
				}
			}
			BjCutsceneStage::Walking => {
				cut.frame_timer.tick(time.delta());
				if cut.frame_timer.just_finished() {
					cut.frame_timer.reset();

					cut.walk_frame = (cut.walk_frame + 1) % 4;

					if let Some(mat) = materials.get_mut(&cut.bj_material) {
						mat.base_color_texture = Some(images.bj_victory_walk[cut.walk_frame].clone());
					}

					if cut.walk_frame == 0 && cut.walk_loops_left > 0 {
						cut.walk_loops_left -= 1;
						if cut.walk_loops_left == 0 {
							cut.stage = BjCutsceneStage::Jumping;
							cut.jump_frame = 0;
							cut.frame_timer = Timer::from_seconds(0.12, TimerMode::Once);
						}
					}
				}
			}
			BjCutsceneStage::Jumping => {
				if !cut.played_yeah {
					cut.played_yeah = true;

					sfx.write(PlaySfx {
						kind: SfxKind::EpisodeVictoryYea,
						pos: Vec3::ZERO,
					});
				}

				cut.frame_timer.tick(time.delta());
				if cut.frame_timer.just_finished() {
					cut.frame_timer.reset();

					cut.jump_frame += 1;

					if cut.jump_frame >= 4 {
						const BJ_DONE_HOLD_SECS: f32 = 5.00;

						cut.stage = BjCutsceneStage::Done;
						cut.stage_timer = Timer::from_seconds(BJ_DONE_HOLD_SECS, TimerMode::Once);
					} else if let Some(mat) = materials.get_mut(&cut.bj_material) {
						mat.base_color_texture = Some(images.bj_victory_jump[cut.jump_frame].clone());
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

		stage = cut.stage;
		jump_frame = cut.jump_frame;

		if stage == BjCutsceneStage::Turning {
			turning_elapsed = cut.stage_timer.elapsed_secs();
		}
	}

	let Some(bj_entity) = bj_entity else {
		return;
	};

	if let Some(result) = finish_result {
		commands.entity(bj_entity).despawn();
		flow.phase = EpisodeEndPhase::Finish(result);
		return;
	}

	let cam_pos = player_tr.translation;

	if let Ok((mut bj_tr, base_pose)) = q_bj.get_mut(bj_entity) {
		let mut dir = cam_pos - bj_tr.translation;
		dir.y = 0.0;

		let dist = dir.length();
		let dir = dir.normalize_or_zero();

		let yaw = dir.x.atan2(dir.z);
		bj_tr.rotation = Quat::from_rotation_y(yaw);

		const BJ_WALK_EARLY_SECS: f32 = 0.0;
		const BJ_TURNING_WALK_SPEED_SCALE: f32 = 1.35;

		let start_walk_now =
			matches!(stage, BjCutsceneStage::Walking)
			|| (matches!(stage, BjCutsceneStage::Turning) && turning_elapsed >= BJ_WALK_EARLY_SECS);

		if start_walk_now {
			const BJ_WALK_SPEED: f32 = 1.10;
			const BJ_STOP_DIST: f32 = 0.30;

			let speed = if matches!(stage, BjCutsceneStage::Turning) {
				BJ_WALK_SPEED * BJ_TURNING_WALK_SPEED_SCALE
			} else {
				BJ_WALK_SPEED
			};

			if dist > BJ_STOP_DIST {
				bj_tr.translation += Vec3::new(dir.x, 0.0, dir.z) * (speed * time.delta_secs());
			}
		}

		let jf = jump_frame.min(3);

		const BJ_JUMP_Y_OFFSETS: [f32; 4] = [0.00, 0.03, 0.05, 0.03];

		let raw_off = match stage {
			BjCutsceneStage::Jumping => BJ_JUMP_Y_OFFSETS[jf],
			BjCutsceneStage::Done => BJ_JUMP_Y_OFFSETS[3],
			_ => 0.0,
		};

		bj_tr.translation.y = base_pose.y + raw_off * base_pose.scale;
	}
}

fn episode_end_finish_to_ui(
	mut commands: Commands,
	mut flow: ResMut<EpisodeEndFlow>,
	mut music: ResMut<MusicMode>,
) {
	let EpisodeEndPhase::Finish(_result) = flow.phase else {
		return;
	};

	music.0 = MusicModeKind::Scores;
	commands.insert_resource(SplashStep::EpisodeVictory);

	flow.phase = EpisodeEndPhase::Inactive;
}

fn world_to_tile(pos: Vec3) -> Option<(u32, u32)> {
	let tx = (pos.x + 0.5).floor() as i32;
	let tz = (pos.z + 0.5).floor() as i32;

	if tx < 0 || tz < 0 {
		return None;
	}

	Some((tx as u32, tz as u32))
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
	let tau = std::f32::consts::TAU;
	let mut delta = (b - a) % tau;
	if delta > std::f32::consts::PI {
		delta -= tau;
	} else if delta < -std::f32::consts::PI {
		delta += tau;
	}
	a + delta * t
}
