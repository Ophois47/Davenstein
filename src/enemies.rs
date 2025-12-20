use bevy::prelude::*;
use bevy::time::Timer;
use crate::actors::{Dead, Health, OccupiesTile};
use crate::player::Player;

const GUARD_MAX_HP: i32 = 6;

#[derive(Resource)]
pub struct GuardSprites {
    pub idle: [Handle<Image>; 8],
    pub pain: Handle<Image>,
    pub death: [Handle<Image>; 4],
    pub corpse: Handle<Image>,
}

impl FromWorld for GuardSprites {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            idle: std::array::from_fn(|i| asset_server.load(format!("enemies/guard/guard_idle_a{i}.png"))),
            pain: asset_server.load("enemies/guard/guard_pain.png"),
            death: [
                asset_server.load("enemies/guard/guard_death_0.png"),
                asset_server.load("enemies/guard/guard_death_1.png"),
                asset_server.load("enemies/guard/guard_death_2.png"),
                asset_server.load("enemies/guard/guard_death_3.png"),
            ],
            corpse: asset_server.load("enemies/guard/guard_corpse.png"),
        }
    }
}

#[derive(Component)]
pub struct Guard;

#[derive(Component)]
pub struct GuardPain {
    pub timer: Timer,
}

pub fn tick_guard_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardPain), With<Guard>>,
) {
    for (e, mut pain) in q.iter_mut() {
        pain.timer.tick(time.delta());

        if pain.timer.is_finished() {
            commands.entity(e).remove::<GuardPain>();
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GuardDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // fixed-step counter
}

#[derive(Component)]
pub struct GuardCorpse;

#[derive(Component, Clone, Copy)]
pub struct Dir8(pub u8); // 0..7, 0 = facing -Z

#[derive(Component, Clone, Copy)]
pub struct View8(pub u8); // cached to avoid redundant texture swaps

pub fn spawn_guard(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &GuardSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    // A vertical quad in the XY plane (normal +Z), UVs “upright”
    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,       // Wolf look: no lighting on sprites
        cull_mode: None,   // safe for billboards
        ..default()
    });

    commands.spawn((
        Guard,
        Dir8(0),
        View8(0),
        Health::new(GUARD_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos), // <-- no base rotation, no negative scale
    ));
}

fn quantize_view8(enemy_dir8: u8, enemy_pos: Vec3, player_pos: Vec3) -> u8 {
    use std::f32::consts::TAU;

    let to_player = player_pos - enemy_pos;
    let flat = Vec3::new(to_player.x, 0.0, to_player.z);
    if flat.length_squared() < 1e-6 {
        return 0;
    }

    let step = TAU / 8.0;
    // 0 when pointing +Z (matches update_guard_views yaw)
    let angle_to_player = flat.x.atan2(flat.z).rem_euclid(TAU);
    // Define Dir8(0) as facing +Z, Dir8(2)=+X, Dir8(4)=-Z, Dir8(6)=-X
    let enemy_yaw = (enemy_dir8 as f32) * step;
    let rel = (angle_to_player - enemy_yaw).rem_euclid(TAU);

    (((rel + step * 0.5) / step).floor() as i32 & 7) as u8
}

pub fn tick_guard_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardDying), With<Guard>>,
) {
    // Wolf-ish: simple fixed tics. At 60Hz FixedUpdate:
    // 6 tics/frame ≈ 0.10s per frame → 4 frames ≈ 0.4s total.
    const DEATH_FRAMES: u8 = 4;
    const TICS_PER_FRAME: u8 = 6;

    for (e, mut dying) in q.iter_mut() {
        dying.tics = dying.tics.saturating_add(1);

        if dying.tics >= TICS_PER_FRAME {
            dying.tics = 0;
            dying.frame = dying.frame.saturating_add(1);

            if dying.frame >= DEATH_FRAMES {
                // End of animation → permanent corpse
                commands.entity(e).remove::<GuardDying>();
                commands.entity(e).insert(GuardCorpse);
            }
        }
    }
}

pub fn apply_guard_corpses(
    sprites: Res<GuardSprites>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(
        &MeshMaterial3d<StandardMaterial>,
        &mut Transform,
        Option<&mut Visibility>,
    ), (With<Guard>, Added<GuardCorpse>)>,
) {
    for (mat3d, mut tf, vis) in q.iter_mut() {
        if let Some(mat) = materials.get_mut(&mat3d.0) {
            mat.base_color_texture = Some(sprites.corpse.clone());
            mat.alpha_mode = AlphaMode::Blend;
            mat.unlit = true;
            mat.cull_mode = None;
        }
        if let Some(mut v) = vis {
            *v = Visibility::Visible;
        }

        // Keep the working floor-anchor fix (do NOT push into floor)
        tf.translation.y = 0.5;
    }
}

pub fn update_guard_views(
    sprites: Res<GuardSprites>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<(
        Option<&Dead>,
        Option<&GuardDying>,
        Option<&GuardPain>,
        &GlobalTransform,
        &Dir8,
        &mut View8,
        &MeshMaterial3d<StandardMaterial>,
        &mut Transform,
    ), With<Guard>>,
) {
    let Ok(pgt) = q_player.single() else { return; };
    let cam_pos = pgt.translation();

    for (dead, dying, pain, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        // Always billboard (alive or dead), Wolf-style
        let to_cam = cam_pos - enemy_pos;
        let yaw = to_cam.x.atan2(to_cam.z);
        tf.rotation = Quat::from_rotation_y(yaw);

        // Dying anim (non-directional)
        if let Some(dying) = dying {
            let i = (dying.frame as usize).min(sprites.death.len() - 1);
            if let Some(mat) = materials.get_mut(&mat3d.0) {
                mat.base_color_texture = Some(sprites.death[i].clone());
            }
            continue;
        }

        // Pain sprite (non-directional)
        if pain.is_some() {
            view.0 = 255; // <--- IMPORTANT
            if let Some(mat) = materials.get_mut(&mat3d.0) {
                mat.base_color_texture = Some(sprites.pain.clone());
            }
            continue;
        }

        // Dead (not dying) → corpse is stable, don't overwrite
        if dead.is_some() {
            continue;
        }

        // Alive → 8-dir idle
        let v = quantize_view8(dir8.0, enemy_pos, cam_pos);
        if v != view.0 {
            view.0 = v;
            if let Some(mat) = materials.get_mut(&mat3d.0) {
                mat.base_color_texture = Some(sprites.idle[v as usize].clone());
            }
        }
    }
}

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuardSprites>()
            .add_systems(Update, update_guard_views)
            .add_systems(FixedUpdate, (tick_guard_dying, tick_guard_pain))
            .add_systems(PostUpdate, apply_guard_corpses);
    }
}
