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

	// Prevent re-triggering while already in any locked sequence
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

	// DOS behavior vibe: snap-turn the view so the player is no longer aiming at the exit
	player_tr.rotation = player_tr.rotation * Quat::from_rotation_y(std::f32::consts::PI);

	let bj_mesh = meshes.add(Rectangle::new(0.95, 1.30));
	let bj_mat = materials.add(StandardMaterial {
		base_color_texture: Some(images.bj_victory_walk[0].clone()),
		alpha_mode: AlphaMode::Mask(0.5),
		unlit: true,
		double_sided: true,
		..default()
	});

	// Child-of-camera so it always frames correctly regardless of world geometry
	let bj_entity = commands
		.spawn((
			Name::new("BJ Victory"),
			Mesh3d(bj_mesh),
			MeshMaterial3d(bj_mat.clone()),
			Transform::from_translation(Vec3::new(0.0, -0.25, -2.00)),
			Visibility::Visible,
		))
		.set_parent_in_place(player_e)
		.id();

	let episode = current_level.0.episode() as u8;

	let result = EpisodeEndResult {
		episode,
		score: hud.score as u32,
	};

	flow.phase = EpisodeEndPhase::BjCutscene(BjCutscene {
		stage: BjCutsceneStage::Turning,
		stage_timer: Timer::from_seconds(0.35, TimerMode::Once),
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
	q_bj: Query<Entity>,
) {
	let EpisodeEndPhase::BjCutscene(cut) = &mut flow.phase else {
		return;
	};

	match cut.stage {
		BjCutsceneStage::Turning => {
			cut.stage_timer.tick(time.delta());
			if cut.stage_timer.just_finished() {
				cut.stage = BjCutsceneStage::Walking;
				cut.frame_timer.reset();
			}
		}
		BjCutsceneStage::Walking => {
			cut.frame_timer.tick(time.delta());
			if !cut.frame_timer.just_finished() {
				return;
			}
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
		BjCutsceneStage::Jumping => {
			if !cut.played_yeah {
				cut.played_yeah = true;

				sfx.write(PlaySfx {
					kind: SfxKind::EpisodeVictoryYea,
					pos: Vec3::ZERO,
				});
			}

			cut.frame_timer.tick(time.delta());
			if !cut.frame_timer.just_finished() {
				return;
			}
			cut.frame_timer.reset();

			cut.jump_frame += 1;

			if cut.jump_frame >= 4 {
				cut.stage = BjCutsceneStage::Done;
			} else if let Some(mat) = materials.get_mut(&cut.bj_material) {
				mat.base_color_texture = Some(images.bj_victory_jump[cut.jump_frame].clone());
			}
		}
		BjCutsceneStage::Done => {}
	}

	if cut.stage != BjCutsceneStage::Done {
		return;
	}

	if q_bj.get(cut.bj_entity).is_ok() {
		commands.entity(cut.bj_entity).despawn();
	}

	let result = cut.result;
	flow.phase = EpisodeEndPhase::Finish(result);
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

	let dt = time.delta_secs();
	cam.elapsed += dt;

	let mut t = cam.elapsed / cam.duration;
	if t > 1.0 {
		t = 1.0;
	}

	// Smoothstep to feel like the original snap-pan without being instant
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
