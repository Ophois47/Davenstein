use bevy::audio::SpatialListener;
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};
use crate::map::{
	DoorState,
	DoorTile,
	MapGrid,
	Tile,
};
use crate::player::{LookAngles, Player};

const TILE_SIZE: f32 = 1.0;
const WALL_H: f32 = 1.0;
const DOOR_THICKNESS: f32 = 0.20;

// ---------- Assets ----------
#[derive(Resource)]
pub struct GameAssets {
    pub wall_tex: Handle<Image>,
    pub floor_tex: Handle<Image>,
    pub door_tex: Handle<Image>,
}

fn load_assets(asset_server: &AssetServer) -> GameAssets {
    GameAssets {
        wall_tex: asset_server.load("textures/walls/wall.png"),
        floor_tex: asset_server.load("textures/floors/floor.png"),
        door_tex: asset_server.load("textures/doors/door.png"),
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 12x12 example map.
    // Legend:  '#' = wall, 'D' = closed door, '.' or ' ' = empty, 'P' = player spawn
    const MAP: [&str; 12] = [
        "############",
        "#P.........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "#..........#",
        "######D#####",
    ];

    let (grid, spawn) = MapGrid::from_ascii(&MAP);
    let spawn = spawn.unwrap_or(IVec2::new(1, 1));

    // Make map available for collision / doors / raycasts later
    commands.insert_resource(grid.clone());

    // Load + store assets
    let assets = load_assets(&asset_server);

    let wall_tex = assets.wall_tex.clone();
    let floor_tex = assets.floor_tex.clone();
    let door_tex = assets.door_tex.clone();

    commands.insert_resource(assets);

    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        ..default()
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(floor_tex),
        ..default()
    });

    let door_mat = materials.add(StandardMaterial {
        base_color_texture: Some(door_tex),
        ..default()
    });

    // Center helpers (our tiles live at x,z = 0..width-1)
    let room_center = Vec3::new(
        (grid.width as f32 - 1.0) * TILE_SIZE * 0.5,
        0.0,
        (grid.height as f32 - 1.0) * TILE_SIZE * 0.5,
    );

    // Light
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(room_center + Vec3::new(0.0, 6.0, 0.0)),
    ));

    // Floor
    commands.spawn((
        Mesh3d(meshes.add(
            Plane3d::default()
                .mesh()
                .size(grid.width as f32 * TILE_SIZE, grid.height as f32 * TILE_SIZE),
        )),
        MeshMaterial3d(floor_mat),
        Transform::from_translation(room_center),
    ));

    // Walls + Doors from grid
    let cube = meshes.add(Cuboid::default());
    for z in 0..grid.height {
        for x in 0..grid.width {
            let tile = grid.tile(x, z);

            // Rotate border walls so the cube's +Z face points inward
            let rot_wall = if z == 0 {
                Quat::IDENTITY
            } else if z == grid.height - 1 {
                Quat::from_rotation_y(PI)
            } else if x == 0 {
                Quat::from_rotation_y(FRAC_PI_2)
            } else if x == grid.width - 1 {
                Quat::from_rotation_y(-FRAC_PI_2)
            } else {
                Quat::IDENTITY
            };

            let door_panel = meshes.add(
            	Plane3d::default()
            		.mesh()
            		.size(TILE_SIZE, WALL_H),
            );

            match tile {
                Tile::Wall => {
                    commands.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(wall_mat.clone()),
                        Transform {
                            translation: Vec3::new(
                                x as f32 * TILE_SIZE,
                                WALL_H * 0.5,
                                z as f32 * TILE_SIZE,
                            ),
                            rotation: rot_wall,
                            scale: Vec3::new(TILE_SIZE, WALL_H, TILE_SIZE),
                        },
                    ));
                }
                Tile::DoorClosed | Tile::DoorOpen => {
				    let is_open = matches!(tile, Tile::DoorOpen);

				    // Determine door axis (same idea as before)
				    let up_wall = z > 0 && matches!(grid.tile(x, z - 1), Tile::Wall);
				    let down_wall = z + 1 < grid.height && matches!(grid.tile(x, z + 1), Tile::Wall);

				    let base = Quat::from_rotation_x(-FRAC_PI_2);
				    let yaw = if up_wall && down_wall { FRAC_PI_2 } else { 0.0 };
				    let rot = Quat::from_rotation_y(yaw) * base;

				    let normal = rot * Vec3::Y;
				    let half = (DOOR_THICKNESS * TILE_SIZE) * 0.5;

				    let center = Vec3::new(
				        x as f32 * TILE_SIZE,
				        WALL_H * 0.5,
				        z as f32 * TILE_SIZE,
				    );

				    commands
				        .spawn((
					        DoorTile(IVec2::new(x as i32, z as i32)),
					        DoorState { open_timer: 0.0 },
					        Transform::from_translation(center),
					        if is_open { Visibility::Hidden } else { Visibility::Visible },
					    ))
				        .with_children(|parent| {
				            // Front
				            parent.spawn((
				                Mesh3d(door_panel.clone()),
				                MeshMaterial3d(door_mat.clone()),
				                Transform {
				                    translation: normal * half,
				                    rotation: rot,
				                    scale: Vec3::ONE,
				                },
				            ));

				            // Back
				            parent.spawn((
				                Mesh3d(door_panel.clone()),
				                MeshMaterial3d(door_mat.clone()),
				                Transform {
				                    translation: -normal * half,
				                    rotation: Quat::from_rotation_y(PI) * rot,
				                    scale: Vec3::ONE,
				                },
				            ));
				        });
				}
				_ => {}
			}
        }
    }

    // Player spawn from grid
    let player_pos = Vec3::new(
        spawn.x as f32 * TILE_SIZE,
        0.6,
        spawn.y as f32 * TILE_SIZE,
    );

    commands.spawn((
        Camera3d::default(),
        Player,
        LookAngles::default(),
        SpatialListener::new(0.2),
        Transform::from_translation(player_pos)
            .looking_at(player_pos + Vec3::NEG_Z, Vec3::Y),
    ));
}

