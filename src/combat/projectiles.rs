/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::render::alpha::AlphaMode;
use rand::Rng;

use davelib::decorations::SolidStatics;
use davelib::map::{MapGrid, Tile};
use davelib::player::{GodMode, Player, PlayerVitals};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProjectileKind {
	Fireball,
	#[allow(dead_code)]
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
		ProjectileKind::Fireball => (0.32, 0.32),
		ProjectileKind::Rocket => (0.40, 0.40),
		ProjectileKind::Syringe => (0.40, 0.40),
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

	let quad = meshes.add(Rectangle::new(1.0, 1.0));

	commands.insert_resource(ProjectileAssets {
		quad,
		fireball_0,
		fireball_1,
		syringe,
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
		info!("SPAWNING PROJECTILE: kind={:?}, origin={:?}", e.kind, e.origin);
		let dir = Vec3::new(e.dir.x, 0.0, e.dir.z);
		let dir = if dir.length_squared() > 0.0001 { dir.normalize() } else { continue };

		let (mut w, mut h) = kind_size(e.kind);
		if matches!(e.kind, ProjectileKind::Fireball) {
			w *= FIREBALL_SCALE;
			h *= FIREBALL_SCALE;
		}

		let tex0 = match e.kind {
			ProjectileKind::Fireball => assets.fireball_0.clone(),
			ProjectileKind::Rocket => {
				warn!("ProjectileKind::Rocket spawned but no sprites are wired yet");
				continue;
			}
			ProjectileKind::Syringe => {
				// Starts at a0 and will be corrected in update_projectile_views
				assets.syringe[0].clone()
			}
		};

		let mat = mats.add(StandardMaterial {
			base_color_texture: Some(tex0),
			alpha_mode: AlphaMode::Blend,
			unlit: true,
			cull_mode: None,
			..default()
		});

		let _id = commands
			.spawn((
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
			))
			.id();
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

pub fn tick_projectiles(
	time: Res<Time>,
	mut commands: Commands,
	grid: Option<Res<MapGrid>>,
	solid: Option<Res<SolidStatics>>,
	god: Option<Res<GodMode>>,
	mut q_player: Query<(&Transform, &mut PlayerVitals), (With<Player>, Without<Projectile>)>,
	mut q: Query<(Entity, &mut Transform, &Projectile)>,
) {
	let Some(grid) = grid else { return; };

	let Some((player_xform, mut vitals)) = q_player.iter_mut().next() else { return; };
	let player_pos = player_xform.translation;

	let god = god.map(|g| g.0).unwrap_or(false);
	let dt = time.delta_secs();

	let player_r = 0.22;
	let proj_r = 0.10;
	let hit_r = player_r + proj_r;

	for (e, mut xform, proj) in q.iter_mut() {
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
				commands.entity(e).despawn();
				continue;
			}
		}

		let Some(tile_b) = tile_at_world(&grid, b) else {
			commands.entity(e).despawn();
			continue;
		};

		if tile_blocks_projectile(tile_b) {
			commands.entity(e).despawn();
			continue;
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
			ProjectileKind::Syringe => {
				proj.anim.tick(time.delta());
				if !proj.anim.just_finished() {
					continue;
				}

				// Ping-pong 4 frames to simulate end-over-end flip
				// Sequence: 0,1,2,3,2,1 then repeat
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
			ProjectileKind::Rocket => {
				// Not wired yet
			}
		}
	}
}
