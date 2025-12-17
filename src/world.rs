use bevy::prelude::*;
use crate::player::{Player, LookAngles, PlayerSettings};

const TILE: f32 = 1.0;
const WALL_H: f32 = 1.0;

// ---------- Assets ----------
#[derive(Resource)]
struct GameAssets {
    wall_tex: Handle<Image>,
    floor_tex: Handle<Image>,
    // Examples for later:
    // shoot_sfx: Handle<AudioSource>,
    // door_open_sfx: Handle<AudioSource>,
}

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
    // If PlayerSettings lives in player.rs, make sure you have:
    // use crate::player::PlayerSettings;
    commands.insert_resource(PlayerSettings::default());

    // Load once, keep local handles, then insert for other systems to use later
    let assets = load_assets(&asset_server);
    let wall_tex = assets.wall_tex.clone();
    let floor_tex = assets.floor_tex.clone();
    commands.insert_resource(assets);

    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        ..default()
    });
    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(floor_tex),
        ..default()
    });

    // Light
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 6.0, 4.0),
    ));

    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(floor_mat),
    ));

    // Simple “room”: border walls
    let cube = meshes.add(Cuboid::default());
    for z in 0..12 {
        for x in 0..12 {
            let is_border = x == 0 || z == 0 || x == 11 || z == 11;
            if !is_border {
                continue;
            }
            commands.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(wall_mat.clone()),
                Transform::from_xyz(x as f32 * TILE, WALL_H * 0.5, z as f32 * TILE)
                    .with_scale(Vec3::new(TILE, WALL_H, TILE)),
            ));
        }
    }

    // Player (make sure Player + LookAngles are imported from crate::player)
    commands.spawn((
        Camera3d::default(),
        Player,
        LookAngles::default(),
        Transform::from_xyz(6.0, 0.6, 6.0).looking_at(Vec3::new(6.0, 0.6, 5.0), Vec3::Y),
    ));
}
