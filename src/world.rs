use bevy::prelude::*;
use crate::player::{Player, LookAngles};

// ---------- Assets ----------
#[allow(dead_code)]
#[derive(Resource)]
struct GameAssets {
    wall_tex: Handle<Image>,
    floor_tex: Handle<Image>,
    // Examples for later:
    // shoot_sfx: Handle<AudioSource>,
    // door_open_sfx: Handle<AudioSource>,
}

#[allow(dead_code)]
fn load_assets(asset_server: &AssetServer) -> GameAssets {
    GameAssets {
        wall_tex: asset_server.load("textures/walls/wall.png"),
        floor_tex: asset_server.load("textures/floors/floor.png"),
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const TILE: f32 = 1.0;
    const WALL_H: f32 = 1.0;
    const ROOM_W: usize = 12;
    const ROOM_H: usize = 12;

    // --- load textures (you said you'll handle assets; these are the paths Bevy will look for) ---
    let wall_tex: Handle<Image> = asset_server.load("textures/walls/wall.png");
    let floor_tex: Handle<Image> = asset_server.load("textures/floors/floor.png");

    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        ..default()
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(floor_tex),
        ..default()
    });

    // Room center in world coords (tiles are at 0..ROOM-1)
    let room_center = Vec3::new(
        (ROOM_W as f32 - 1.0) * TILE * 0.5,
        0.0,
        (ROOM_H as f32 - 1.0) * TILE * 0.5,
    );

    // Light (also center it so lighting makes sense)
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(room_center + Vec3::new(0.0, 6.0, 0.0)),
    ));

    // Floor: size covers the whole room, but translated to sit under your 0..11 tiles
    commands.spawn((
        Mesh3d(meshes.add(
            Plane3d::default()
                .mesh()
                .size(ROOM_W as f32 * TILE, ROOM_H as f32 * TILE),
        )),
        MeshMaterial3d(floor_mat),
        Transform::from_translation(room_center),
    ));

    // Walls: rotate border cubes so the inward-facing side uses the same cube face
    let cube = meshes.add(Cuboid::default());
    for z in 0..ROOM_H {
        for x in 0..ROOM_W {
            let is_border = x == 0 || z == 0 || x == ROOM_W - 1 || z == ROOM_H - 1;
            if !is_border {
                continue;
            }

            use std::f32::consts::{FRAC_PI_2, PI};

            // Make the cube's +Z face point inward for each edge.
            // (Corners still show 2 faces; think "pillar".)
            let rot = if z == 0 {
                Quat::IDENTITY
            } else if z == ROOM_H - 1 {
                Quat::from_rotation_y(PI)
            } else if x == 0 {
                Quat::from_rotation_y(FRAC_PI_2)
            } else {
                Quat::from_rotation_y(-FRAC_PI_2)
            };

            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(wall_mat.clone()),
                Transform {
                    translation: Vec3::new(x as f32 * TILE, WALL_H * 0.5, z as f32 * TILE),
                    rotation: rot,
                    scale: Vec3::new(TILE, WALL_H, TILE),
                },
            ));
        }
    }

    // Camera/player (keep yours as-is; just showing a typical placement)
    commands.spawn((
        Camera3d::default(),
        Player,
        LookAngles::default(),
        Transform::from_xyz(6.0, 0.6, 6.0).looking_at(Vec3::new(6.0, 0.6, 5.0), Vec3::Y),
    ));
}
