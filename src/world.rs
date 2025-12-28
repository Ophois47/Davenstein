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
        // Wolf wall-sheet (top-left 8x8 = the 64 wall textures in index order).
        // We remap UVs per wall ID, so this is shared by all wall materials.
        wall_tex: asset_server.load("textures/walls/wolf_walls.png"),
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
    // Legend:
    // '#' = Wall
    // 'D' = Closed Door
    // 'O' = Open Door
    // '.' or ' ' = Empty
    // 'P' = Player Spawn
    // 'G' = Enemy Guard
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
        base_color_texture: Some(wall_tex.clone()),
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let wall_mat_dark = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex.clone()),
        base_color: Color::srgb(0.75, 0.75, 0.75),
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

    // --- Wall atlas mapping (WL6 VSWAP walls 0..105 packed 16x7, 64x64 each) ---
    const VSWAP_WALL_CHUNKS: usize = 106;
    const ATLAS_COLS: usize = 16;
    const ATLAS_ROWS: usize = (VSWAP_WALL_CHUNKS + ATLAS_COLS - 1) / ATLAS_COLS; // = 7

    fn atlas_uv(index: usize) -> (f32, f32, f32, f32) {
        // Atlas is authored top-to-bottom, and Bevy image UVs treat (0,0) as top-left.
        // So: do NOT flip V. We still return (u0, u1, v0, v1) where v0 is "bottom" and v1 is "top"
        // because build_atlas_panel interpolates sz bottom->top: uv.y = v0 + sz*(v1 - v0).
        //
        // Half-texel inset reduces bleeding between tiles.
        const TILE_PX: f32 = 64.0;
        const ATLAS_W_PX: f32 = ATLAS_COLS as f32 * TILE_PX; // 1024
        const ATLAS_H_PX: f32 = ATLAS_ROWS as f32 * TILE_PX; // 448
        const HALF_U: f32 = 0.5 / ATLAS_W_PX;
        const HALF_V: f32 = 0.5 / ATLAS_H_PX;

        let col = index % ATLAS_COLS;
        let row = index / ATLAS_COLS;

        let u0 = col as f32 / ATLAS_COLS as f32 + HALF_U;
        let u1 = (col + 1) as f32 / ATLAS_COLS as f32 - HALF_U;

        // v increases downward (top-left origin). Top edge is smaller v.
        let v_top = row as f32 / ATLAS_ROWS as f32;
        let v_bottom = (row + 1) as f32 / ATLAS_ROWS as f32;

        // Return bottom first (v0) and top second (v1) to match build_atlas_panel's bottom->top sz.
        let v0 = v_bottom - HALF_V; // bottom edge
        let v1 = v_top + HALF_V;    // top edge

        (u0, u1, v0, v1)
    }

    fn build_atlas_panel(
        meshes: &mut Assets<Mesh>,
        u0: f32,
        u1: f32,
        v0: f32,
        v1: f32,
        flip_u: bool,
    ) -> Handle<Mesh> {
        use bevy::mesh::VertexAttributeValues;

        let mut m: Mesh = Plane3d::default().mesh().size(TILE_SIZE, WALL_H).build();
        let positions: Vec<[f32; 3]> = match m.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => p.clone(),
            _ => Vec::new(),
        };

        let (min_x, max_x, min_z, max_z) = positions.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
            |(min_x, max_x, min_z, max_z), p| {
                (
                    min_x.min(p[0]),
                    max_x.max(p[0]),
                    min_z.min(p[2]),
                    max_z.max(p[2]),
                )
            },
        );

        let dx = (max_x - min_x).max(1e-6);
        let dz = (max_z - min_z).max(1e-6);

        if let Some(VertexAttributeValues::Float32x2(uvs)) = m.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
            for (p, uv) in positions.iter().zip(uvs.iter_mut()) {
                let mut sx = (p[0] - min_x) / dx; // left->right
                if flip_u {
                    sx = 1.0 - sx;
                }
                let sz = (p[2] - min_z) / dz; // bottom->top
                uv[0] = u0 + sx * (u1 - u0);
                uv[1] = v0 + sz * (v1 - v0);
            }
        }

        meshes.add(m)
    }

    // Build wall panels from the atlas
    let mut atlas_panels: Vec<Handle<Mesh>> = Vec::with_capacity(VSWAP_WALL_CHUNKS);
    for i in 0..VSWAP_WALL_CHUNKS {
        let (u0, u1, v0, v1) = atlas_uv(i);
        atlas_panels.push(build_atlas_panel(&mut meshes, u0, u1, v0, v1, false));
    }

    // Doors use door.png, so they must NOT use atlas-cell UVs.
    // Keep the “old” orientation by flipping V here.
    let door_panel_front = build_atlas_panel(&mut meshes, 0.0, 1.0, 1.0, 0.0, false);
    let door_panel_back = build_atlas_panel(&mut meshes, 0.0, 1.0, 1.0, 0.0, true);
    let jamb_panel = build_atlas_panel(&mut meshes, 0.0, 1.0, 1.0, 0.0, false);
    let wall_base = Quat::from_rotation_x(-FRAC_PI_2); // Make Plane3d Vertical

    // Walls + Doors From Grid
    for z in 0..grid.height {
        for x in 0..grid.width {
            let tile = grid.tile(x, z);

            match tile {
                Tile::Wall => {
                    let cx = x as f32 * TILE_SIZE;
                    let cz = z as f32 * TILE_SIZE;
                    let y = WALL_H * 0.5;

                    // Wolf wall IDs in plane0 are 1..=63 (0 means empty).
                    let wall_id = grid.plane0_code(x, z);

                    // 0-based "wall type" from the map
                    let wall_type = (wall_id as usize).saturating_sub(1);

                    // Many Wolf-style atlases store light/dark as adjacent chunks:
                    // type 0 => (0 light, 1 dark), type 1 => (2 light, 3 dark), ...
                    let pair_base = wall_type.saturating_mul(2);

                    let (light_idx, dark_idx) = if pair_base + 1 < VSWAP_WALL_CHUNKS {
                        (pair_base, pair_base + 1)
                    } else {
                        // fallback (shouldn't happen for E1M1 since wall ids are low)
                        let idx = wall_type.min(VSWAP_WALL_CHUNKS - 1);
                        (idx, idx)
                    };

                    let wall_mesh_light = atlas_panels[light_idx].clone();
                    let wall_mesh_dark  = atlas_panels[dark_idx].clone();

                    let is_wall = |xx: usize, zz: usize| matches!(grid.tile(xx, zz), Tile::Wall);
                    let is_door = |t: Tile| matches!(t, Tile::DoorClosed | Tile::DoorOpen);

                    let mut spawn_face =
                        |mesh: Handle<Mesh>, pos: Vec3, yaw: f32, mat: Handle<StandardMaterial>| {
                            commands.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(mat),
                                Transform {
                                    translation: pos,
                                    rotation: Quat::from_rotation_y(yaw) * wall_base,
                                    ..default()
                                },
                            ));
                        };

                    // -Z (north)
                    if z == 0 || !is_wall(x, z - 1) {
                        let neighbor_is_door = z > 0 && is_door(grid.tile(x, z - 1));
                        spawn_face(
                            if neighbor_is_door { jamb_panel.clone() } else { wall_mesh_light.clone() },
                            Vec3::new(cx, y, cz - TILE_SIZE * 0.5),
                            0.0,
                            if neighbor_is_door { jamb_mat.clone() } else { wall_mat.clone() },
                        );
                    }

                    // +Z (south)
                    if z + 1 >= grid.height || !is_wall(x, z + 1) {
                        let neighbor_is_door = (z + 1) < grid.height && is_door(grid.tile(x, z + 1));
                        spawn_face(
                            if neighbor_is_door { jamb_panel.clone() } else { wall_mesh_light.clone() },
                            Vec3::new(cx, y, cz + TILE_SIZE * 0.5),
                            std::f32::consts::PI,
                            if neighbor_is_door { jamb_mat.clone() } else { wall_mat.clone() },
                        );
                    }

                    // -X (west)
                    if x == 0 || !is_wall(x - 1, z) {
                        let neighbor_is_door = x > 0 && is_door(grid.tile(x - 1, z));
                        spawn_face(
                            if neighbor_is_door { jamb_panel.clone() } else { wall_mesh_dark.clone() },
                            Vec3::new(cx - TILE_SIZE * 0.5, y, cz),
                            std::f32::consts::FRAC_PI_2,
                            if neighbor_is_door { jamb_mat.clone() } else { wall_mat_dark.clone() },
                        );
                    }

                    // +X (east)
                    if x + 1 >= grid.width || !is_wall(x + 1, z) {
                        let neighbor_is_door = (x + 1) < grid.width && is_door(grid.tile(x + 1, z));
                        spawn_face(
                            if neighbor_is_door { jamb_panel.clone() } else { wall_mesh_dark.clone() },
                            Vec3::new(cx + TILE_SIZE * 0.5, y, cz),
                            -std::f32::consts::FRAC_PI_2,
                            if neighbor_is_door { jamb_mat.clone() } else { wall_mat_dark.clone() },
                        );
                    }
                }
                Tile::DoorClosed | Tile::DoorOpen => {
                    let is_open = matches!(tile, Tile::DoorOpen);

                    // Determine Orientation From Adjacent Walls
                    let left_wall = x > 0 && matches!(grid.tile(x - 1, z), Tile::Wall);
                    let right_wall = x + 1 < grid.width && matches!(grid.tile(x + 1, z), Tile::Wall);
                    let up_wall = z > 0 && matches!(grid.tile(x, z - 1), Tile::Wall);
                    let down_wall = z + 1 < grid.height && matches!(grid.tile(x, z + 1), Tile::Wall);

                    let walls_x = (left_wall as u8) + (right_wall as u8);
                    let walls_z = (up_wall as u8) + (down_wall as u8);

                    // If walls are more above/below, corridor runs E/W => door plane faces +/-X => yaw 90°
                    // Otherwise corridor runs N/S => yaw 0°
                    let yaw = if walls_z > walls_x { FRAC_PI_2 } else { 0.0 };

                    if walls_x == 0 && walls_z == 0 {
                        bevy::log::warn!("Door at ({},{}) has no adjacent walls?", x, z);
                        let code = grid.plane0_code(x, z);
                        warn!("Door at ({},{}) plane0_code={} has no adjacent walls?", x, z, code);
                    }

                    // Plane3d normal is +Y; rotate vertical then yaw.
                    let base = Quat::from_rotation_x(-FRAC_PI_2);
                    let rot = Quat::from_rotation_y(yaw) * base;

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

                            // Back (mirrored)
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
        (spawn.x as f32 + 0.5) * TILE_SIZE,
        0.6,
        (spawn.y as f32 + 0.5) * TILE_SIZE,
    );

    commands.spawn((
        Camera3d::default(),
        IsDefaultUiCamera,
        Player,
        crate::player::PlayerVitals::default(),
        LookAngles::new(spawn_yaw, 0.0),
        SpatialListener::new(0.2),
        Transform::from_translation(player_pos).with_rotation(Quat::from_rotation_y(spawn_yaw)),
    ));
}

