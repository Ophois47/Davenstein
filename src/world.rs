use bevy::audio::SpatialListener;
use bevy::prelude::*;
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
) {
    use bevy::mesh::VertexAttributeValues;

    // Legend:  '#' = wall, 'D' = closed door, '.' or ' ' = empty, 'P' = player spawn
    const MAP: [&str; 16] = [
        "########################",
        "#..........#...........#",
        "#..........D...........#",
        "#..........#...........#",
        "#..........#....##D#####",
        "#..........#....#......#",
        "#..........#....#......#",
        "##D#########....###D####",
        "#......................#",
        "#....###########.......#",
        "#....#.........#.......#",
        "#....#.........D.......#",
        "#....#.........#.......#",
        "#....###########.......#",
        "#.....................P#",
        "########################",
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

    // ---------- Reusable meshes / constants (CREATE ONCE) ----------
    let wall_face = meshes.add(Plane3d::default().mesh().size(TILE_SIZE, WALL_H));
    let wall_base = Quat::from_rotation_x(-FRAC_PI_2); // make Plane3d vertical
    let half_tile = TILE_SIZE * 0.5;

    // Build a Plane3d mesh but flip UVs deterministically.
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

    // Flip V to fix upside-down on both sides.
    // Don't flip U; the back panel's 180° rotation already makes it read correctly.
    let door_panel_front = make_panel(false, true);
    let door_panel_back  = make_panel(true, true);

    let is_door = |t: Tile| matches!(t, Tile::DoorClosed | Tile::DoorOpen);

    // ---------- Walls + Doors from grid ----------
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

                    // Determine orientation from WALLS (robust, Wolf-style)
                    let left_wall = x > 0 && matches!(grid.tile(x - 1, z), Tile::Wall);
                    let right_wall = x + 1 < grid.width && matches!(grid.tile(x + 1, z), Tile::Wall);
                    let up_wall = z > 0 && matches!(grid.tile(x, z - 1), Tile::Wall);
                    let down_wall = z + 1 < grid.height && matches!(grid.tile(x, z + 1), Tile::Wall);

                    // If walls are above+below, passage runs E/W (X), so door faces +/-X.
                    // Otherwise default to passage running N/S (Z), door faces +/-Z.
                    let yaw = if up_wall && down_wall { FRAC_PI_2 } else { 0.0 };

                    // Plane3d normal is +Y; rotate vertical so normal becomes +Z, then yaw.
                    let base = Quat::from_rotation_x(-FRAC_PI_2);
                    let rot = Quat::from_rotation_y(yaw) * base;

                    // Plane3d's local normal is +Y
                    let normal = rot * Vec3::Y;

                    // Door thickness offset for the two panels
                    let half_thickness = (DOOR_THICKNESS * TILE_SIZE) * 0.5;

                    let center = Vec3::new(
                        x as f32 * TILE_SIZE,
                        WALL_H * 0.5,
                        z as f32 * TILE_SIZE,
                    );

                    // Robust orientation (optional but recommended)
                    let walls_x = (left_wall as u8) + (right_wall as u8);
                    let walls_z = (up_wall as u8) + (down_wall as u8);
                    let yaw = if walls_z > walls_x { FRAC_PI_2 } else { 0.0 };

                    // Door slides along its local +X after yaw.
                    // This is the key: no sign flipping / fallback = consistent “handle side” feel
                    let slide_axis = Quat::from_rotation_y(yaw) * Vec3::X;

                    // Optional sanity log if the door placement is weird
                    if walls_x == 0 && walls_z == 0 {
                        bevy::log::warn!("Door at ({},{}) has no adjacent walls?", x, z);
                    }

                    let progress = if is_open { 1.0 } else { 0.0 };
                    let start_pos = center + slide_axis * (progress * TILE_SIZE);

                    let vis = if is_open { Visibility::Hidden } else { Visibility::Visible };

                    commands
                        .spawn((
                            DoorTile(IVec2::new(x as i32, z as i32)),
                            DoorState { open_timer: 0.0 },
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

                            // Back (mirrored via UV flip mesh so the handle stays on the right)
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
        Transform::from_translation(player_pos).looking_at(player_pos + Vec3::NEG_Z, Vec3::Y),
    ));
}
