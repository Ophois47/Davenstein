use bevy::prelude::*;
use crate::player::Player;

#[derive(Resource)]
pub struct GuardSprites {
    pub idle: [Handle<Image>; 8],
}

impl FromWorld for GuardSprites {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            idle: std::array::from_fn(|i| {
                asset_server.load(format!("enemies/guard/guard_idle_a{i}.png"))
            }),
        }
    }
}

#[derive(Component)]
pub struct Guard;

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

pub fn update_guard_views(
    sprites: Res<GuardSprites>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<(
        &GlobalTransform,
        &Dir8,
        &mut View8,
        &MeshMaterial3d<StandardMaterial>,
        &mut Transform,
    ), With<Guard>>,
) {
    let Ok(pgt) = q_player.single() else { return; };
    let cam_pos = pgt.translation();

    for (gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        // Billboard yaw-only toward the player/camera
        let to_cam = cam_pos - enemy_pos;
        let yaw = to_cam.x.atan2(to_cam.z);

        // Rectangle faces +Z at yaw=0, so this is correct (no base rotation).
        tf.rotation = Quat::from_rotation_y(yaw);

        // 8-way sprite selection (your existing function)
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
            .add_systems(Update, update_guard_views);
    }
}
