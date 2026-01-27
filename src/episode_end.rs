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
	yaw_from: f32,
	yaw_to: f32,
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
	stage: DeathCamStage,
	boss_e: Entity,
	kind: DeathCamBossKind,
	replay_requested: bool,
	saw_dying: bool,
	elapsed: f32,
	duration: f32,
	start_yaw: f32,
	start_pitch: f32,
	end_yaw: f32,
	end_pitch: f32,
    kill_pos: Vec3,
    replay_pos_set: bool,
	result: EpisodeEndResult,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeathCamStage {
	Turning,
	WaitForCorpse,
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

fn deathcam_pick_replay_pos(grid: &MapGrid, boss_pos: Vec3, kill_pos: Vec3, cam_y: f32) -> Vec3 {
	let mut dir = boss_pos - kill_pos;
	dir.y = 0.0;

	let mut dir = dir.normalize_or_zero();
	if dir.length_squared() < 1e-6 {
		dir = Vec3::new(0.0, 0.0, 1.0);
	}

	const MIN_DIST_TILES: f32 = 1.25;
	const STEP_TILES: f32 = 0.0625;
	const MAX_DIST_TILES: f32 = 8.0;

	let mut dist = MIN_DIST_TILES;
	while dist <= MAX_DIST_TILES {
		let mut p = boss_pos - dir * dist;
		p.y = cam_y;

		if deathcam_pos_ok(grid, p) {
			return p;
		}

		dist += STEP_TILES;
	}

	let mut p = boss_pos - dir * MIN_DIST_TILES;
	p.y = cam_y;
	p
}

fn start_death_cam(
	mut flow: ResMut<EpisodeEndFlow>,
	mut lock: ResMut<PlayerControlLock>,
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
	q_player: Query<&Transform, With<Player>>,
) {
	if !matches!(flow.phase, EpisodeEndPhase::Inactive) {
		return;
	}

	let Some((boss_e, boss_tr, hitler, schabbs, otto, general)) = q_dead_boss.iter().next() else {
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

	let Some(player_tr) = q_player.iter().next() else {
		return;
	};

	lock.0 = true;

	let episode = current_level.0.episode() as u8;
	let result = EpisodeEndResult {
		episode,
		score: hud.score as u32,
	};

	const DEATH_CAM_MAX_PITCH: f32 = 0.35;
	const DEATH_CAM_TURN_SECS: f32 = 1.25;

	let (start_yaw, start_pitch_raw, _roll) = player_tr.rotation.to_euler(EulerRot::YXZ);
	let start_pitch = start_pitch_raw.clamp(-DEATH_CAM_MAX_PITCH, DEATH_CAM_MAX_PITCH);

	let to = boss_tr.translation - player_tr.translation;
	let flat_len2 = to.x * to.x + to.z * to.z;

	let (end_yaw, end_pitch) = if flat_len2 <= 1e-6 {
		(start_yaw, start_pitch)
	} else {
		let dir = to.normalize();

		let yaw = (-dir.x).atan2(-dir.z);

		let pitch_raw = dir.y.atan2((dir.x * dir.x + dir.z * dir.z).sqrt());
		let pitch = pitch_raw.clamp(-DEATH_CAM_MAX_PITCH, DEATH_CAM_MAX_PITCH);
		(yaw, pitch)
	};

	flow.phase = EpisodeEndPhase::DeathCam(DeathCam {
		stage: DeathCamStage::Turning,
		boss_e,
		kind,
		kill_pos: player_tr.translation,
		replay_pos_set: false,
		replay_requested: false,
		saw_dying: false,
		elapsed: 0.0,
		duration: DEATH_CAM_TURN_SECS,
		start_yaw,
		start_pitch,
		end_yaw,
		end_pitch,
		result,
	});
}

fn tick_death_cam(
	mut commands: Commands,
	mut flow: ResMut<EpisodeEndFlow>,
	time: Res<Time>,
	grid: Option<Res<MapGrid>>,
	mut q_player: Query<&mut Transform, With<Player>>,
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
	const REPLAY_MIN_DIST_TILES: f32 = 2.20;
	const REPLAY_STEP_TILES: f32 = 0.0625;
	const REPLAY_MAX_DIST_TILES: f32 = 8.0;

	let Some(mut player_tr) = q_player.iter_mut().next() else {
		let result = cam.result;
		flow.phase = EpisodeEndPhase::Finish(result);
		return;
	};

	let player_pos = player_tr.translation;

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

	let pos_ok = |pos: Vec3| -> bool {
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
	};

	let pick_replay_pos = |boss_pos: Vec3, kill_pos: Vec3, cam_y: f32| -> Vec3 {
		let mut dir = boss_pos - kill_pos;
		dir.y = 0.0;

		let mut dir = dir.normalize_or_zero();
		if dir.length_squared() < 1e-6 {
			dir = Vec3::new(0.0, 0.0, 1.0);
		}

		let mut dist = REPLAY_MIN_DIST_TILES;
		while dist <= REPLAY_MAX_DIST_TILES {
			let mut p = boss_pos - dir * dist;
			p.y = cam_y;

			if pos_ok(p) {
				return p;
			}

			dist += REPLAY_STEP_TILES;
		}

		let mut p = boss_pos - dir * REPLAY_MIN_DIST_TILES;
		p.y = cam_y;
		p
	};

	match cam.stage {
		DeathCamStage::Turning => {
			cam.elapsed += time.delta_secs();

			let mut t = cam.elapsed / cam.duration.max(1e-6);
			if t > 1.0 {
				t = 1.0;
			}

			let t = t * t * (3.0 - 2.0 * t);

			let yaw = lerp_angle(cam.start_yaw, cam.end_yaw, t);
			let pitch = cam.start_pitch + (cam.end_pitch - cam.start_pitch) * t;

			player_tr.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

			if cam.elapsed >= cam.duration {
				cam.elapsed = 0.0;
				cam.duration = DEATH_CAM_PRE_REPLAY_SECS;
				cam.stage = DeathCamStage::WaitForCorpse;
			}
		}

		DeathCamStage::WaitForCorpse => {
			if !boss_is_corpse {
				cam.elapsed = 0.0;
				player_tr.rotation = Quat::from_euler(EulerRot::YXZ, cam.end_yaw, cam.end_pitch, 0.0);
				return;
			}

			if !cam.replay_pos_set {
				let cam_y = player_tr.translation.y;
				let replay_pos = pick_replay_pos(boss_pos, cam.kill_pos, cam_y);

				player_tr.translation = replay_pos;

				let to = boss_pos - replay_pos;
				let flat_len2 = to.x * to.x + to.z * to.z;

				if flat_len2 > 1e-6 {
					let dir = to.normalize();

					cam.end_yaw = (-dir.x).atan2(-dir.z);

					let pitch_raw = dir.y.atan2((dir.x * dir.x + dir.z * dir.z).sqrt());
					cam.end_pitch = pitch_raw.clamp(-DEATH_CAM_MAX_PITCH, DEATH_CAM_MAX_PITCH);
				}

				player_tr.rotation = Quat::from_euler(EulerRot::YXZ, cam.end_yaw, cam.end_pitch, 0.0);

				cam.replay_pos_set = true;
			} else {
				player_tr.rotation = Quat::from_euler(EulerRot::YXZ, cam.end_yaw, cam.end_pitch, 0.0);
			}

			cam.elapsed += time.delta_secs();
			if cam.elapsed >= cam.duration {
				cam.elapsed = 0.0;
				cam.duration = 0.0;
				cam.stage = DeathCamStage::Replaying;
			}
		}

		DeathCamStage::Replaying => {
			player_tr.rotation = Quat::from_euler(EulerRot::YXZ, cam.end_yaw, cam.end_pitch, 0.0);

			if !cam.replay_requested {
				match cam.kind {
					DeathCamBossKind::Hitler => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::HitlerCorpse>();
						commands.entity(cam.boss_e).insert(davelib::enemies::HitlerDying { frame: 0, tics: 0 });
					}
					DeathCamBossKind::Schabbs => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::SchabbsCorpse>();
						commands.entity(cam.boss_e).insert(davelib::enemies::SchabbsDying { frame: 0, tics: 0 });
					}
					DeathCamBossKind::Otto => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::OttoCorpse>();
						commands.entity(cam.boss_e).insert(davelib::enemies::OttoDying { frame: 0, tics: 0 });
					}
					DeathCamBossKind::General => {
						commands.entity(cam.boss_e).remove::<davelib::enemies::GeneralCorpse>();
						commands.entity(cam.boss_e).insert(davelib::enemies::GeneralDying { frame: 0, tics: 0 });
					}
				}

				cam.replay_requested = true;
				cam.saw_dying = false;
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
			player_tr.rotation = Quat::from_euler(EulerRot::YXZ, cam.end_yaw, cam.end_pitch, 0.0);

			cam.elapsed += time.delta_secs();
			if cam.elapsed >= cam.duration {
				let result = cam.result;
				flow.phase = EpisodeEndPhase::Finish(result);
			}
		}
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

	let (yaw_from, _pitch, _roll) = player_tr.rotation.to_euler(EulerRot::YXZ);

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
	const DOLLY_MAX: f32 = 4.85;

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
	let yaw_to = forward_to_door.x.atan2(-forward_to_door.z);

	player_tr.rotation = Quat::from_euler(EulerRot::YXZ, yaw_from, 0.0, 0.0);

	let cam_end = cam_start + away_dir * dolly_dist;

	commands.entity(player_e).insert(BjDolly {
		start: cam_start,
		end: cam_end,
		yaw_from,
		yaw_to,
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

					let yaw = lerp_angle(dolly.yaw_from, dolly.yaw_to, t);
					player_tr.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);
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
						const BJ_DONE_HOLD_SECS: f32 = 4.30;

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
