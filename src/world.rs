/*
Davenstein - by David Petnick
*/
use bevy::audio::SpatialListener;
use bevy::prelude::*;
use bevy::ui::prelude::IsDefaultUiCamera;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::map::{
    DoorAnim,
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
    pub jamb_tex: Handle<Image>,
}

fn load_assets(asset_server: &AssetServer) -> GameAssets {
    GameAssets {
        wall_tex: asset_server.load("textures/walls/wall.png"),
        floor_tex: asset_server.load("textures/floors/floor.png"),
        door_tex: asset_server.load("textures/doors/door.png"),
        jamb_tex: asset_server.load("textures/walls/jamb.png"),
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    guard_sprites: Res<crate::enemies::GuardSprites>,
) {
    use bevy::mesh::VertexAttributeValues;

    // Legend:
    // '#' = Wall
    // 'D' = Closed Door
    // 'O' = Open Door
    // '.' or ' ' = Empty
    // 'P' = Player Spawn
    // 'G' = Enemy Guard
    // Toggle: load original Wolf3D E1M1 data (Wolf1 Map1) from assets/maps.
    const USE_WOLF_E1M1: bool = true;

    const E1M1_PLANE0: &str = include_str!("../assets/maps/e1m1_plane0_u16.txt");
    const E1M1_PLANE1: &str = include_str!("../assets/maps/e1m1_plane1_u16.txt");

    const TEST_MAP: [&str; 32] = [
	    "################################",
	    "#..G...G..G...G.......#........#",
	    "#....G....G...G.......#........#",
	    "#..G...G...G....G.....#........#",
	    "#.######D######.......#........#",
	    "#.#...........#.......#........#",
	    "#.#...G.......#.......#........#",
	    "#.#...........#.......#........#",
	    "#.#...........#.......#........#",
	    "#.#############.......#........#",
	    "#.....................D........#",
	    "#######D#################D######",
	    "#.....................#........#",
	    "#.................G...#.G......#",
	    "#.G...................#........#",
	    "#.....................#........#",
	    "#.....................#........#",
	    "#.....................#G.......#",
	    "#.........G...........#........#",
	    "#.....................#........#",
	    "#.....................#........#",
	    "######.#.################.#.####",
	    "#.....................#G.......#",
	    "#...............G.....#........#",
	    "#....G................#........#",
	    "#.....................#........#",
	    "#.....................#........#",
	    "#.....................#........#",
	    "#.....................#........#",
	    "#..............G......D........#",
	    "#.P...................#........#",
	    "################################",
	];

    let (grid, spawn, guards) = if USE_WOLF_E1M1 {
        let plane0 = MapGrid::parse_u16_grid(E1M1_PLANE0, 64, 64);
        let plane1 = MapGrid::parse_u16_grid(E1M1_PLANE1, 64, 64);
        MapGrid::from_wolf_planes(64, 64, &plane0, &plane1)
    } else {
        let (g, spawn, guards) = MapGrid::from_ascii(&TEST_MAP);
        (g, spawn.map(|p| (p, 0.0)), guards)
    };

    let (spawn, spawn_yaw) = spawn.unwrap_or((IVec2::new(1, 1), 0.0));

    // Make Map Available for Collision / Doors / Raycasts
    commands.insert_resource(grid.clone());

    // Load + Store Assets
    let assets = load_assets(&asset_server);
    let wall_tex = assets.wall_tex.clone();
    let floor_tex = assets.floor_tex.clone();
    let door_tex = assets.door_tex.clone();
    let jamb_tex = assets.jamb_tex.clone();
    commands.insert_resource(assets);

    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex),
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let door_mat = materials.add(StandardMaterial {
        base_color_texture: Some(door_tex),
        unlit: true,
        ..default()
    });

    let jamb_mat = materials.add(StandardMaterial {
        base_color_texture: Some(jamb_tex),
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(floor_tex),
        unlit: true,
        ..default()
    });

    // Center Helpers (Tiles Live at X,Z = 0..Width-1)
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

    // Reusable Meshes / Constants
    let wall_face = meshes.add(Plane3d::default().mesh().size(TILE_SIZE, WALL_H));
    let wall_base = Quat::from_rotation_x(-FRAC_PI_2); // Make Plane3d Vertical
    let half_tile = TILE_SIZE * 0.5;

    // Build a Plane3d Mesh but Flip UVs Deterministically
    let mut make_panel = |flip_u: bool, flip_v: bool| -> Handle<Mesh> {
        let mut m: Mesh = Plane3d::default().mesh().size(TILE_SIZE, WALL_H).build();

        if let Some(VertexAttributeValues::Float32x2(uvs)) = m.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
            for uv in uvs.iter_mut() {
                if flip_u {
                    uv[0] = 1.0 - uv[0];
                }
                if flip_v {
                    uv[1] = 1.0 - uv[1];
                }
            }
        }

        meshes.add(m)
    };

    // Flip V to Fix Upside-Down on Both Sides
    let door_panel_front = make_panel(false, true);
    let door_panel_back  = make_panel(true, true);

    let is_door = |t: Tile| matches!(t, Tile::DoorClosed | Tile::DoorOpen);

    // Walls + Doors From Grid
    for z in 0..grid.height {
        for x in 0..grid.width {
            let tile = grid.tile(x, z);

            match tile {
                Tile::Wall => {
                    let cx = x as f32 * TILE_SIZE;
                    let cz = z as f32 * TILE_SIZE;
                    let y = WALL_H * 0.5;

                    let mut spawn_face = |pos: Vec3, yaw: f32, mat: Handle<StandardMaterial>| {
                        commands.spawn((
                            Mesh3d(wall_face.clone()),
                            MeshMaterial3d(mat),
                            Transform {
                                translation: pos,
                                rotation: Quat::from_rotation_y(yaw) * wall_base,
                                ..default()
                            },
                        ));
                    };

                    // North (-Z)
                    if z > 0 {
                        let n = grid.tile(x, z - 1);
                        if !matches!(n, Tile::Wall) {
                            let mat = if is_door(n) { jamb_mat.clone() } else { wall_mat.clone() };
                            spawn_face(Vec3::new(cx, y, cz - half_tile), PI, mat);
                        }
                    }
                    // South (+Z)
                    if z + 1 < grid.height {
                        let s = grid.tile(x, z + 1);
                        if !matches!(s, Tile::Wall) {
                            let mat = if is_door(s) { jamb_mat.clone() } else { wall_mat.clone() };
                            spawn_face(Vec3::new(cx, y, cz + half_tile), 0.0, mat);
                        }
                    }
                    // West (-X)
                    if x > 0 {
                        let w = grid.tile(x - 1, z);
                        if !matches!(w, Tile::Wall) {
                            let mat = if is_door(w) { jamb_mat.clone() } else { wall_mat.clone() };
                            spawn_face(Vec3::new(cx - half_tile, y, cz), -FRAC_PI_2, mat);
                        }
                    }
                    // East (+X)
                    if x + 1 < grid.width {
                        let e = grid.tile(x + 1, z);
                        if !matches!(e, Tile::Wall) {
                            let mat = if is_door(e) { jamb_mat.clone() } else { wall_mat.clone() };
                            spawn_face(Vec3::new(cx + half_tile, y, cz), FRAC_PI_2, mat);
                        }
                    }
                }
                Tile::DoorClosed | Tile::DoorOpen => {
				    let is_open = matches!(tile, Tile::DoorOpen);

				    // Determine Orientation From Adjacent Walls
				    let left_wall  = x > 0 && matches!(grid.tile(x - 1, z), Tile::Wall);
				    let right_wall = x + 1 < grid.width && matches!(grid.tile(x + 1, z), Tile::Wall);
				    let up_wall    = z > 0 && matches!(grid.tile(x, z - 1), Tile::Wall);
				    let down_wall  = z + 1 < grid.height && matches!(grid.tile(x, z + 1), Tile::Wall);

				    let walls_x = (left_wall as u8) + (right_wall as u8);
				    let walls_z = (up_wall as u8) + (down_wall as u8);

				    // If Walls are "More" Above / Below, Corridor 
                    // Runs E/W => Door Plane Faces +/-X => Yaw 90 Degrees
				    // Otherwise Corridor Runs N/S => Yaw 0 Degrees
				    let yaw = if walls_z > walls_x { FRAC_PI_2 } else { 0.0 };

				    if walls_x == 0 && walls_z == 0 {
				        bevy::log::warn!("Door at ({},{}) has no adjacent walls?", x, z);
				    }

				    // Plane3d Normal is +Y, Rotate Vertical so Normal Becomes Horizontal, Then Yaw
				    let base = Quat::from_rotation_x(-FRAC_PI_2);
				    let rot = Quat::from_rotation_y(yaw) * base;

				    // Plane3d Local Normal is +Y
				    let normal = rot * Vec3::Y;

				    // Door Thickness Offset for Two Panels
				    let half_thickness = (DOOR_THICKNESS * TILE_SIZE) * 0.5;

				    let center = Vec3::new(
				        x as f32 * TILE_SIZE,
				        WALL_H * 0.5,
				        z as f32 * TILE_SIZE,
				    );

				    // Door Slides Along Local +X After Yaw
				    let slide_axis = Quat::from_rotation_y(yaw) * Vec3::X;

				    let progress = if is_open { 1.0 } else { 0.0 };
				    let start_pos = center + slide_axis * (progress * TILE_SIZE);

				    let vis = if is_open { Visibility::Hidden } else { Visibility::Visible };

				    commands
				        .spawn((
				            DoorTile(IVec2::new(x as i32, z as i32)),
				            DoorState { open_timer: 0.0, want_open: is_open },
				            DoorAnim {
				                progress,
				                closed_pos: center,
				                slide_axis,
				            },
				            Transform::from_translation(start_pos),
				            vis,
				        ))
				        .with_children(|parent| {
				            // Front
				            parent.spawn((
				                Mesh3d(door_panel_front.clone()),
				                MeshMaterial3d(door_mat.clone()),
				                Transform {
				                    translation: normal * half_thickness,
				                    rotation: rot,
				                    ..default()
				                },
				            ));

				            // Back (Mirrored via UV Flip Mesh so Handle Stays on Right)
				            parent.spawn((
				                Mesh3d(door_panel_back.clone()),
				                MeshMaterial3d(door_mat.clone()),
				                Transform {
				                    translation: -normal * half_thickness,
				                    rotation: Quat::from_rotation_y(PI) * rot,
				                    ..default()
				                },
				            ));
				        });
				}
                _ => {}
            }
        }
    }

    for g in guards {
        crate::enemies::spawn_guard(
            &mut commands,
            &mut meshes,
            &mut materials,
            &guard_sprites,
            g,
        );
    }

    // Player Spawn From Grid
    let player_pos = Vec3::new(
        spawn.x as f32 * TILE_SIZE,
        0.6,
        spawn.y as f32 * TILE_SIZE,
    );

    commands.spawn((
        Camera3d::default(),
        IsDefaultUiCamera,
        Player,
        // NEW: gives the lib a place to apply enemy damage without touching HudState
        crate::player::PlayerVitals::default(),
        LookAngles::new(spawn_yaw, 0.0),
        SpatialListener::new(0.2),
        Transform::from_translation(player_pos).with_rotation(Quat::from_rotation_y(spawn_yaw)),
    ));
}
