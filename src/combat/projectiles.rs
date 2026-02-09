/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use rand::RngExt;

use davelib::audio::{PlaySfx, SfxKind};
use davelib::decorations::SolidStatics;
use davelib::map::{MapGrid, Tile};
use davelib::player::{
	GodMode,
	Player,
	PlayerVitals,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProjectileKind {
	Fireball,
	Rocket,
	Syringe,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct SpawnProjectile {
	pub kind: ProjectileKind,
	pub origin: Vec3,
	pub dir: Vec3,
}

#[derive(Resource)]
pub struct ProjectileAssets {
	pub quad: Handle<Mesh>,
	pub fireball_0: Handle<Image>,
	pub fireball_1: Handle<Image>,
	pub syringe: [Handle<Image>; 4],
	pub rocket: [Handle<Image>; 8],
	pub rocket_smoke: [Handle<Image>; 4],
	pub rocket_impact: [Handle<Image>; 4],
}

#[derive(Component)]
pub struct Projectile {
	pub kind: ProjectileKind,
	pub dir: Vec3,
	pub speed: f32,
	pub anim: Timer,
	pub frame: usize,
}

#[derive(Component)]
pub struct ProjectileView {
	pub mat: Handle<StandardMaterial>,
}

// Rocket Smoke Logic
#[derive(Component)]
pub struct RocketSmokeEmitter {
	pub tics: u8,
}

#[derive(Component)]
pub struct SmokePuff {
	pub frame: usize,
	pub tics: u8,
}

#[derive(Component)]
pub struct SmokePuffView {
	pub mat: Handle<StandardMaterial>,
}

const ROCKET_SMOKE_EMIT_TICS: u8 = 3;
const SMOKE_FRAME_TICS: u8 = 3;
const SMOKE_FRAMES: usize = 4;

// Rocket Impact Logic
#[derive(Component)]
pub struct RocketImpact {
	pub frame: usize,
	pub tics: u8,
}

#[derive(Component)]
pub struct RocketImpactView {
	pub mat: Handle<StandardMaterial>,
}

const IMPACT_FRAME_TICS: u8 = 3;
const IMPACT_FRAMES: usize = 4;

fn kind_speed(kind: ProjectileKind) -> f32 {
	match kind {
		ProjectileKind::Fireball => 1.6,
		ProjectileKind::Rocket => 8.5,
		ProjectileKind::Syringe => 8.5,
	}
}

fn kind_damage(kind: ProjectileKind) -> i32 {
	match kind {
		ProjectileKind::Fireball => rand::rng().random_range(0..32),
		ProjectileKind::Rocket => rand::rng().random_range(10..41),
		ProjectileKind::Syringe => rand::rng().random_range(5..21),
	}
}

fn kind_anim_period(kind: ProjectileKind) -> f32 {
	match kind {
		ProjectileKind::Fireball => 0.08,
		ProjectileKind::Rocket => 0.12,
		ProjectileKind::Syringe => 0.12,
	}
}

fn kind_size(kind: ProjectileKind) -> (f32, f32) {
	match kind {
		ProjectileKind::Fireball => (0.34, 0.34),
		ProjectileKind::Rocket => (0.40, 0.40),
		ProjectileKind::Syringe => (0.50, 0.50),
	}
}

fn world_to_tile_xz(p: Vec3) -> (i32, i32) {
	let tx = (p.x + 0.5).floor() as i32;
	let tz = (p.z + 0.5).floor() as i32;
	(tx, tz)
}

fn tile_at_world(grid: &MapGrid, p: Vec3) -> Option<Tile> {
	let (tx, tz) = world_to_tile_xz(p);
	if tx < 0 || tz < 0 || tx >= grid.width as i32 || tz >= grid.height as i32 {
		return None;
	}
	Some(grid.tile(tx as usize, tz as usize))
}

/// Calculate which of 8 directional sprites to show based on:
/// - The direction the projectile is traveling (proj_dir)
/// - The direction from projectile to player (to_player)
/// Matches Classic Wolfenstein 3-D Sprite Direction Logic
/// Direction 0 = Facing Away from Player (South)
/// Direction 4 = Facing Toward Player (North)
/// Directions Go Clockwise: 0,1,2,3,4,5,6,7
fn calculate_dir8_index(proj_dir: Vec3, to_player: Vec3) -> usize {
	// Normalize Both Directions in XZ Plane
	let proj_xz = Vec2::new(proj_dir.x, proj_dir.z).normalize_or_zero();
	let player_xz = Vec2::new(to_player.x, to_player.z).normalize_or_zero();
	
	if proj_xz == Vec2::ZERO || player_xz == Vec2::ZERO {
		return 0;
	}
	
	// Calculate Angle of Projectile's Direction 
	// Relative to Player's View Direction
	// Get Angle From Player to Rocket
	let view_angle = player_xz.y.atan2(player_xz.x);
	
	// Get Angle of Rocket's Travel Direction
	let proj_angle = proj_xz.y.atan2(proj_xz.x);
	
	// The Difference Tells us Rocket's Orientation Relative to Player's View
	let mut relative_angle = proj_angle - view_angle;
	
	// Normalize to [0, 2π)
	while relative_angle < 0.0 {
		relative_angle += std::f32::consts::TAU;
	}
	while relative_angle >= std::f32::consts::TAU {
		relative_angle -= std::f32::consts::TAU;
	}
	
	// Convert to 8 Directions (Octants)
	// 0 = Facing Away (South), 4 = Facing Toward (North), Clockwise
	let octant = ((relative_angle + std::f32::consts::PI / 8.0) / (std::f32::consts::TAU / 8.0)).floor() as i32;
	
	// Map to Wolfenstein 3D Convention:
	// Octant 0 (East) -> Direction 2 (East)
	// Octant 1 (NE) -> Direction 3 (NE)  
	// Octant 2 (North) -> Direction 4 (North, Toward Player)
	// Octant 3 (NW) -> Direction 5 (NW)
	// Octant 4 (West) -> Direction 6 (West)
	// Octant 5 (SW) -> Direction 7 (SW)
	// Octant 6 (South) -> Direction 0 (South, Away From Player)
	// Octant 7 (SE) -> Direction 1 (SE)
	
	let dir = match octant {
		0 => 2,  // E
		1 => 3,  // NE
		2 => 4,  // N (Toward Player)
		3 => 5,  // NW
		4 => 6,  // W
		5 => 7,  // SW
		6 => 0,  // S (Away From Player)
		7 => 1,  // SE
		_ => 0,
	};
	
	dir as usize
}

fn spawn_rocket_impact(
	commands: &mut Commands,
	mats: &mut Assets<StandardMaterial>,
	assets: &ProjectileAssets,
	pos: Vec3,
) {
	let mat = mats.add(StandardMaterial {
		base_color_texture: Some(assets.rocket_impact[0].clone()),
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		cull_mode: None,
		..default()
	});

	commands.spawn((
		RocketImpact { frame: 0, tics: IMPACT_FRAME_TICS },
		RocketImpactView { mat: mat.clone() },
		Mesh3d(assets.quad.clone()),
		MeshMaterial3d(mat),
		Transform::from_translation(pos).with_scale(Vec3::new(0.85, 0.85, 1.0)),
	));
}

pub fn tick_rocket_impacts(
	mut commands: Commands,
	assets: Option<Res<ProjectileAssets>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
	mut q: Query<(Entity, &mut RocketImpact, &RocketImpactView)>,
) {
	let Some(assets) = assets else { return; };

	for (e, mut imp, view) in q.iter_mut() {
		if imp.tics > 0 {
			imp.tics -= 1;
		}

		if imp.tics != 0 {
			continue;
		}

		imp.frame += 1;
		if imp.frame >= IMPACT_FRAMES {
			commands.entity(e).despawn();
			continue;
		}

		imp.tics = IMPACT_FRAME_TICS;

		let Some(mat) = mats.get_mut(&view.mat) else { continue; };
		let tex = assets.rocket_impact[imp.frame].clone();
		if mat.base_color_texture.as_ref() != Some(&tex) {
			mat.base_color_texture = Some(tex);
		}
	}
}

pub fn update_rocket_impact_views(
	q_player: Query<&Transform, (With<Player>, Without<RocketImpact>)>,
	mut q: Query<&mut Transform, (With<RocketImpact>, Without<Player>)>,
) {
	let Some(player_xform) = q_player.iter().next() else { return; };
	let player_pos = player_xform.translation;

	for mut xform in q.iter_mut() {
		let to_player = player_pos - xform.translation;
		let yaw = to_player.x.atan2(to_player.z);
		xform.rotation = Quat::from_rotation_y(yaw);
	}
}

pub fn setup_projectile_assets(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut meshes: ResMut<Assets<Mesh>>,
) {
	let fireball_0: Handle<Image> =
		asset_server.load("enemies/ghost_hitler/fake_hitler_fireball_0.png");
	let fireball_1: Handle<Image> =
		asset_server.load("enemies/ghost_hitler/fake_hitler_fireball_1.png");

	let syringe: [Handle<Image>; 4] = std::array::from_fn(|i| {
		asset_server.load(format!("enemies/schabbs/syringe_a{i}.png"))
	});

	let rocket: [Handle<Image>; 8] = std::array::from_fn(|i| {
		asset_server.load(format!("enemies/otto/otto_rocket_{i}.png"))
	});

	let rocket_smoke: [Handle<Image>; 4] = std::array::from_fn(|i| {
		asset_server.load(format!("enemies/otto/otto_smoke_{i}.png"))
	});

	let rocket_impact: [Handle<Image>; 4] = std::array::from_fn(|i| {
		asset_server.load(format!("enemies/otto/otto_impact_{i}.png"))
	});

	let quad = meshes.add(Rectangle::new(1.0, 1.0));

	commands.insert_resource(ProjectileAssets {
		quad,
		fireball_0,
		fireball_1,
		syringe,
		rocket,
		rocket_smoke,
		rocket_impact,
	});
}

pub fn spawn_projectiles(
	mut commands: Commands,
	mut mats: ResMut<Assets<StandardMaterial>>,
	assets: Option<Res<ProjectileAssets>>,
	mut ev: MessageReader<SpawnProjectile>,
) {
	let Some(assets) = assets else { return; };

	const FIREBALL_SCALE: f32 = 3.5;

	for e in ev.read() {
		let dir = Vec3::new(e.dir.x, 0.0, e.dir.z);
		let dir = if dir.length_squared() > 0.0001 { dir.normalize() } else { continue };

		let (mut w, mut h) = kind_size(e.kind);
		if matches!(e.kind, ProjectileKind::Fireball) {
			w *= FIREBALL_SCALE;
			h *= FIREBALL_SCALE;
		}

		let tex0 = match e.kind {
			ProjectileKind::Fireball => assets.fireball_0.clone(),
			ProjectileKind::Rocket => assets.rocket[0].clone(),
			ProjectileKind::Syringe => assets.syringe[0].clone(),
		};

		let mat = mats.add(StandardMaterial {
			base_color_texture: Some(tex0),
			alpha_mode: AlphaMode::Blend,
			unlit: true,
			cull_mode: None,
			..default()
		});

		let mut ent = commands.spawn((
			Projectile {
				kind: e.kind,
				dir,
				speed: kind_speed(e.kind),
				anim: Timer::from_seconds(kind_anim_period(e.kind), TimerMode::Repeating),
				frame: 0,
			},
			ProjectileView { mat: mat.clone() },
			Mesh3d(assets.quad.clone()),
			MeshMaterial3d(mat),
			Transform::from_translation(e.origin).with_scale(Vec3::new(w, h, 1.0)),
		));

		if matches!(e.kind, ProjectileKind::Rocket) {
			ent.insert(RocketSmokeEmitter { tics: ROCKET_SMOKE_EMIT_TICS });
		}
	}
}

fn segment_hits_player_xz(a: Vec3, b: Vec3, p: Vec3, r: f32) -> bool {
	let ax = a.x;
	let az = a.z;
	let bx = b.x;
	let bz = b.z;
	let px = p.x;
	let pz = p.z;

	let abx = bx - ax;
	let abz = bz - az;
	let apx = px - ax;
	let apz = pz - az;

	let ab_len2 = abx * abx + abz * abz;
	if ab_len2 < 0.000001 {
		let dx = px - ax;
		let dz = pz - az;
		return dx * dx + dz * dz <= r * r;
	}

	let mut t = (apx * abx + apz * abz) / ab_len2;
	t = t.clamp(0.0, 1.0);

	let cx = ax + abx * t;
	let cz = az + abz * t;

	let dx = px - cx;
	let dz = pz - cz;

	dx * dx + dz * dz <= r * r
}

fn segment_hits_solid_statics(a: Vec3, b: Vec3, solid: &SolidStatics) -> bool {
	let d = b - a;
	let len = Vec3::new(d.x, 0.0, d.z).length();
	if len <= 0.0001 {
		let (tx, tz) = world_to_tile_xz(a);
		return solid.is_solid(tx, tz);
	}

	let step = 0.08;
	let steps = (len / step).ceil().max(1.0) as i32;

	for i in 1..=steps {
		let t = (i as f32) / (steps as f32);
		let p = a.lerp(b, t);
		let (tx, tz) = world_to_tile_xz(p);
		if solid.is_solid(tx, tz) {
			return true;
		}
	}

	false
}

fn tile_blocks_projectile(t: Tile) -> bool {
	match t {
		Tile::Empty => false,
		Tile::DoorOpen => false,
		Tile::Wall => true,
		Tile::DoorClosed => true,
	}
}

pub fn tick_smoke_puffs(
	mut commands: Commands,
	assets: Option<Res<ProjectileAssets>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
	mut q: Query<(Entity, &mut SmokePuff, &SmokePuffView)>,
) {
	let Some(assets) = assets else { return; };

	for (e, mut puff, view) in q.iter_mut() {
		if puff.tics > 0 {
			puff.tics -= 1;
		}

		if puff.tics != 0 {
			continue;
		}

		puff.frame += 1;
		if puff.frame >= SMOKE_FRAMES {
			commands.entity(e).despawn();
			continue;
		}

		puff.tics = SMOKE_FRAME_TICS;

		let Some(mat) = mats.get_mut(&view.mat) else { continue; };
		let tex = assets.rocket_smoke[puff.frame].clone();
		if mat.base_color_texture.as_ref() != Some(&tex) {
			mat.base_color_texture = Some(tex);
		}
	}
}

pub fn update_smoke_puff_views(
	q_player: Query<&Transform, (With<Player>, Without<SmokePuff>)>,
	mut q: Query<&mut Transform, (With<SmokePuff>, Without<Player>)>,
) {
	let Some(player_xform) = q_player.iter().next() else { return; };
	let player_pos = player_xform.translation;

	for mut xform in q.iter_mut() {
		let to_player = player_pos - xform.translation;
		let yaw = to_player.x.atan2(to_player.z);
		xform.rotation = Quat::from_rotation_y(yaw);
	}
}

pub fn tick_projectiles(
	time: Res<Time>,
	mut commands: Commands,
	assets: Option<Res<ProjectileAssets>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
	grid: Option<Res<MapGrid>>,
	solid: Option<Res<SolidStatics>>,
	god: Option<Res<GodMode>>,
	mut sfx: MessageWriter<PlaySfx>,
	mut q_player: Query<(&Transform, &mut PlayerVitals), (With<Player>, Without<Projectile>)>,
	mut q: Query<(Entity, &mut Transform, &Projectile, Option<&mut RocketSmokeEmitter>)>,
) {
	let Some(grid) = grid else { return; };
	let Some(assets) = assets else { return; };

	let Some((player_xform, mut vitals)) = q_player.iter_mut().next() else { return; };
	let player_pos = player_xform.translation;

	let god = god.map(|g| g.0).unwrap_or(false);
	let dt = time.delta_secs();

	let player_r = 0.22;
	let proj_r = 0.10;
	let hit_r = player_r + proj_r;

	for (e, mut xform, proj, emitter) in q.iter_mut() {
		let a = xform.translation;
		let b = a + proj.dir * proj.speed * dt;

		if !god && segment_hits_player_xz(a, b, player_pos, hit_r) {
			let dmg = kind_damage(proj.kind);
			vitals.hp = (vitals.hp - dmg).max(0);
			commands.entity(e).despawn();
			continue;
		}

		if let Some(solid) = solid.as_deref() {
			if segment_hits_solid_statics(a, b, solid) {
				if matches!(proj.kind, ProjectileKind::Rocket) {
					let hit_pos = a + proj.dir * 0.12;
					spawn_rocket_impact(&mut commands, &mut mats, &assets, hit_pos);
					sfx.write(PlaySfx { kind: SfxKind::RocketImpact, pos: hit_pos });
				}

				commands.entity(e).despawn();
				continue;
			}
		}

		let Some(tile_b) = tile_at_world(&grid, b) else {
			commands.entity(e).despawn();
			continue;
		};

		if tile_blocks_projectile(tile_b) {
			if matches!(proj.kind, ProjectileKind::Rocket) {
				let hit_pos = a + proj.dir * 0.12;
				spawn_rocket_impact(&mut commands, &mut mats, &assets, hit_pos);
				sfx.write(PlaySfx { kind: SfxKind::RocketImpact, pos: hit_pos });
			}

			commands.entity(e).despawn();
			continue;
		}

		if let Some(mut em) = emitter {
			if em.tics > 0 {
				em.tics -= 1;
			}

			if em.tics == 0 {
				em.tics = ROCKET_SMOKE_EMIT_TICS;

				let mat = mats.add(StandardMaterial {
					base_color_texture: Some(assets.rocket_smoke[0].clone()),
					alpha_mode: AlphaMode::Blend,
					unlit: true,
					cull_mode: None,
					..default()
				});

				commands.spawn((
					SmokePuff { frame: 0, tics: SMOKE_FRAME_TICS },
					SmokePuffView { mat: mat.clone() },
					Mesh3d(assets.quad.clone()),
					MeshMaterial3d(mat),
					Transform::from_translation(a).with_scale(Vec3::new(0.55, 0.55, 1.0)),
				));
			}
		}

		xform.translation = b;
	}
}

pub fn update_projectile_views(
	time: Res<Time>,
	assets: Option<Res<ProjectileAssets>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
	q_player: Query<&Transform, (With<Player>, Without<ProjectileView>)>,
	mut q: Query<(&mut Transform, &mut Projectile, &ProjectileView)>,
) {
	let Some(assets) = assets else { return; };

	let Some(player_xform) = q_player.iter().next() else { return; };
	let player_pos = player_xform.translation;

	for (mut xform, mut proj, view) in q.iter_mut() {
		let to_player = player_pos - xform.translation;
		let yaw = to_player.x.atan2(to_player.z);
		xform.rotation = Quat::from_rotation_y(yaw);

		let Some(mat) = mats.get_mut(&view.mat) else { continue; };

		match proj.kind {
            ProjectileKind::Fireball => {
                proj.anim.tick(time.delta());
                if !proj.anim.just_finished() {
                    continue;
                }

                proj.frame = (proj.frame + 1) & 1;

                let tex = if proj.frame == 0 {
                    assets.fireball_0.clone()
                } else {
                    assets.fireball_1.clone()
                };

                if mat.base_color_texture.as_ref() != Some(&tex) {
                    mat.base_color_texture = Some(tex);
                }
            }
            ProjectileKind::Rocket => {
                // Calculate Which Directional Sprite to Show
                let dir_index = calculate_dir8_index(proj.dir, to_player);
                let tex = assets.rocket[dir_index].clone();
                
                if mat.base_color_texture.as_ref() != Some(&tex) {
                    mat.base_color_texture = Some(tex);
                }
            }
            ProjectileKind::Syringe => {
                proj.anim.tick(time.delta());
                if !proj.anim.just_finished() {
                    continue;
                }

                // 4 frames to Simulate End Over End Flip
                // Sequence: 0,1,2,3,2,1 Then Repeat
                proj.frame = (proj.frame + 1) % 6;

                let i = match proj.frame {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    3 => 3,
                    4 => 2,
                    _ => 1,
                };

                let tex = assets.syringe[i].clone();
                if mat.base_color_texture.as_ref() != Some(&tex) {
                    mat.base_color_texture = Some(tex);
                }
            }
        }
	}
}
