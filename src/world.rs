/*
Davenstein - by David Petnick
*/
// Wolf3D E1M1 wall textures + door jambs — what actually worked
//
// Problem:
// - E1M1 walls showed wrong textures (often door/jamb-looking tiles).
// - Once walls became correct, door jambs were missing around doors.
// Goal: Map Wolf plane0 wall IDs to the *correct* original textures and restore jambs.
//
// What worked (final solution):
//
// 1) Use a VSWAP-ordered wall atlas and index it by chunk number (0..105).
//    - Atlas layout: walls 0..105 packed as 16x7 tiles, each 64x64.
//    - UV mapping must respect Bevy’s UV convention: (0,0) is top-left.
//      => DO NOT flip V for the wall atlas.
//    - Add half-texel inset in UVs to reduce bleeding between tiles.
//
// 2) Apply Wolf-style light/dark pairing stored as adjacent atlas chunks.
//    - In this atlas, wall “types” are stored as pairs:
//        type 0 => chunks (0 light, 1 dark)
//        type 1 => chunks (2 light, 3 dark)
//        ...
//    - Map plane0 wall_id (1..63) to 0-based wall_type:
//        wall_type = wall_id - 1
//      then:
//        pair_base = wall_type * 2
//        light_idx = pair_base
//        dark_idx  = pair_base + 1
//    - Render Z faces (north/south) with the LIGHT chunk,
//      and X faces (west/east) with the DARK chunk (classic Wolf directional shading).
//
// 3) Door jambs were “missing” due to spawn logic (jamb faces were impossible to spawn).
//    - Old face condition blocked doors:
//        if !is_wall(neighbor) && !is_door(neighbor) { spawn_face(...) }
//      This prevents wall faces adjacent to doors from spawning, so jambs can’t appear.
//    - Minimal fix:
//        Spawn a wall face whenever the neighbor is NOT a wall (including doors),
//        then choose mesh/material based on whether the neighbor is a door:
//          neighbor is Door  => jamb_panel + jamb_mat
//          neighbor is Empty => wall mesh + wall material
//
// Result:
// - Walls now match correct E1M1 textures.
// - Door jambs appear correctly around door openings.
// - No hacks beyond correct atlas order + UV mapping + correct door-adjacent face spawning.
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
use crate::pushwalls::PushwallMarkers;

const TILE_SIZE: f32 = 1.0;
const WALL_H: f32 = 1.0;
const DOOR_THICKNESS: f32 = 0.20;

const DOOR_NORMAL_LIGHT: usize = 98;
const DOOR_NORMAL_DARK: usize = 99;
const DOOR_JAMB_LIGHT: usize = 100;
const DOOR_JAMB_DARK: usize = 101;
const DOOR_ELEV_LIGHT: usize = 102;
const DOOR_ELEV_DARK: usize = 103;
const DOOR_SILVER: usize = 104;
const DOOR_GOLD: usize = 105;

#[derive(Component)]
pub struct WallFace;

#[derive(Message, Clone, Copy, Debug)]
pub struct RebuildWalls {
    /// Optional tile to treat as a wall for adjacency tests, but NOT spawned
    /// as a static wall face (the moving pushwall will render it).
    pub skip: Option<IVec2>,
}

#[derive(Resource, Clone)]
pub struct WallRenderCache {
    pub atlas_panels: Vec<Handle<Mesh>>,
    pub jamb_panel: Handle<Mesh>,
    pub wall_base: Quat,
    pub wall_mat: Handle<StandardMaterial>,
    pub wall_mat_dark: Handle<StandardMaterial>,
    pub jamb_mat: Handle<StandardMaterial>,
}

// ---------- Assets ----------
#[derive(Resource)]
pub struct GameAssets {
    pub wall_tex: Handle<Image>,
    pub floor_tex: Handle<Image>,
}

fn load_assets(asset_server: &AssetServer) -> GameAssets {
    GameAssets {
        // Wolf wall-sheet (top-left 8x8 = the 64 wall textures in index order).
        // We remap UVs per wall ID, so this is shared by all wall materials.
        wall_tex: asset_server.load("textures/walls/wolf_walls.png"),
        floor_tex: asset_server.load("textures/floors/floor.png"),
    }
}

fn spawn_wall_faces_for_grid(
    commands: &mut Commands,
    grid: &MapGrid,
    cache: &WallRenderCache,
    skip: Option<IVec2>,
) {
    // Real wall test from the grid.
    let is_wall_real = |xx: i32, zz: i32| -> bool {
        if xx < 0 || zz < 0 {
            return false;
        }
        let (xu, zu) = (xx as usize, zz as usize);
        if xu >= grid.width || zu >= grid.height {
            return false;
        }
        matches!(grid.tile(xu, zu), Tile::Wall)
    };

    // Neighbor-wall test for face culling.
    // IMPORTANT: if the neighbor is the moving pushwall tile (`skip`), treat it as EMPTY
    // so adjacent walls will still spawn their faces toward the moving pushwall.
    let is_wall_neighbor = |xx: i32, zz: i32| -> bool {
        if let Some(st) = skip {
            if st.x == xx && st.y == zz {
                return false;
            }
        }
        is_wall_real(xx, zz)
    };

    let is_door = |xx: i32, zz: i32| -> bool {
        if xx < 0 || zz < 0 {
            return false;
        }
        let (xu, zu) = (xx as usize, zz as usize);
        if xu >= grid.width || zu >= grid.height {
            return false;
        }
        matches!(grid.tile(xu, zu), Tile::DoorClosed | Tile::DoorOpen)
    };

    let mut spawn_face =
        |mesh: Handle<Mesh>, mat: Handle<StandardMaterial>, pos: Vec3, yaw: f32| {
            commands.spawn((
                WallFace,
                Mesh3d(mesh),
                MeshMaterial3d(mat),
                Transform {
                    translation: pos,
                    rotation: Quat::from_rotation_y(yaw) * cache.wall_base,
                    ..default()
                },
                Visibility::Visible,
            ));
        };

    // Helper: fetch a jamb mesh from the atlas, with a safe fallback.
    let jamb_mesh = |idx: usize| -> Handle<Mesh> {
        cache
            .atlas_panels
            .get(idx)
            .cloned()
            .unwrap_or_else(|| cache.jamb_panel.clone())
    };

    for z in 0..grid.height {
        for x in 0..grid.width {
            // Never spawn static faces for the moving pushwall tile itself.
            if let Some(st) = skip {
                if st.x == x as i32 && st.y == z as i32 {
                    continue;
                }
            }

            // Only actual wall tiles spawn wall faces.
            if !matches!(grid.tile(x, z), Tile::Wall) {
                continue;
            }

            let wall_id = grid.plane0_code(x, z);
            if wall_id == 0 {
                continue;
            }

            // Wolf-style paired light/dark chunks in VSWAP order.
            let wall_type = (wall_id as usize).saturating_sub(1);
            let pair_base = wall_type.saturating_mul(2);
            if cache.atlas_panels.is_empty() {
                continue;
            }
            let max_i = cache.atlas_panels.len() - 1;
            let light_idx = pair_base.min(max_i);
            let dark_idx = (pair_base + 1).min(max_i);

            let wall_mesh_light = cache.atlas_panels[light_idx].clone();
            let wall_mesh_dark = cache.atlas_panels[dark_idx].clone();

            let cx = x as f32 * TILE_SIZE;
            let cz = z as f32 * TILE_SIZE;
            let y = WALL_H * 0.5;

            // NORTH (-Z)
            if z == 0 || !is_wall_neighbor(x as i32, z as i32 - 1) {
                let neighbor_is_door = z > 0 && is_door(x as i32, z as i32 - 1);
                spawn_face(
                    if neighbor_is_door {
                        jamb_mesh(DOOR_JAMB_LIGHT)
                    } else {
                        wall_mesh_light.clone()
                    },
                    if neighbor_is_door {
                        cache.wall_mat.clone()
                    } else {
                        cache.wall_mat.clone()
                    },
                    Vec3::new(cx, y, cz - TILE_SIZE * 0.5),
                    0.0,
                );
            }

            // SOUTH (+Z)
            if z + 1 >= grid.height || !is_wall_neighbor(x as i32, z as i32 + 1) {
                let neighbor_is_door = (z + 1) < grid.height && is_door(x as i32, z as i32 + 1);
                spawn_face(
                    if neighbor_is_door {
                        jamb_mesh(DOOR_JAMB_LIGHT)
                    } else {
                        wall_mesh_light.clone()
                    },
                    if neighbor_is_door {
                        cache.wall_mat.clone()
                    } else {
                        cache.wall_mat.clone()
                    },
                    Vec3::new(cx, y, cz + TILE_SIZE * 0.5),
                    PI,
                );
            }

            // WEST (-X)
            if x == 0 || !is_wall_neighbor(x as i32 - 1, z as i32) {
                let neighbor_is_door = x > 0 && is_door(x as i32 - 1, z as i32);
                spawn_face(
                    if neighbor_is_door {
                        jamb_mesh(DOOR_JAMB_DARK)
                    } else {
                        wall_mesh_dark.clone()
                    },
                    if neighbor_is_door {
                        cache.wall_mat.clone()
                    } else {
                        cache.wall_mat_dark.clone()
                    },
                    Vec3::new(cx - TILE_SIZE * 0.5, y, cz),
                    FRAC_PI_2,
                );
            }

            // EAST (+X)
            if x + 1 >= grid.width || !is_wall_neighbor(x as i32 + 1, z as i32) {
                let neighbor_is_door = (x + 1) < grid.width && is_door(x as i32 + 1, z as i32);
                spawn_face(
                    if neighbor_is_door {
                        jamb_mesh(DOOR_JAMB_DARK)
                    } else {
                        wall_mesh_dark.clone()
                    },
                    if neighbor_is_door {
                        cache.wall_mat.clone()
                    } else {
                        cache.wall_mat_dark.clone()
                    },
                    Vec3::new(cx + TILE_SIZE * 0.5, y, cz),
                    -FRAC_PI_2,
                );
            }
        }
    }
}

pub fn rebuild_wall_faces_on_request(
    mut commands: Commands,
    grid: Res<MapGrid>,
    cache: Res<WallRenderCache>,
    mut msgs: MessageReader<RebuildWalls>,
    q_faces: Query<Entity, With<WallFace>>,
) {
    // Coalesce all rebuild requests this frame; last one wins for skip.
    let mut any = false;
    let mut skip = None;
    for m in msgs.read() {
        any = true;
        skip = m.skip;
    }
    if !any {
        return;
    }

    for e in q_faces.iter() {
        commands.entity(e).despawn();
    }

    spawn_wall_faces_for_grid(&mut commands, &grid, &cache, skip);
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    guard_sprites: Res<crate::enemies::GuardSprites>,
) {
    const E1M1_PLANE0: &str = include_str!("../assets/maps/e1m1_plane0_u16.txt");
    const E1M1_PLANE1: &str = include_str!("../assets/maps/e1m1_plane1_u16.txt");

    let plane0 = MapGrid::parse_u16_grid(E1M1_PLANE0, 64, 64);
    let plane1 = MapGrid::parse_u16_grid(E1M1_PLANE1, 64, 64);

    let pushwall_markers = PushwallMarkers::from_wolf_plane1(64, 64, &plane1);
    let (grid, spawn, guards) = MapGrid::from_wolf_planes(64, 64, &plane0, &plane1);
    let (spawn, spawn_yaw) = spawn.unwrap_or((IVec2::new(1, 1), 0.0));

    // Make Map Available for Collision / Doors / Raycasts
    commands.insert_resource(grid.clone());
    // Blocking statics (decorations) occupancy
    commands.insert_resource(crate::decorations::SolidStatics::new(grid.width, grid.height));
    // Pushwall markers (plane1 == 98)
    commands.insert_resource(pushwall_markers);

    // Load + Store Assets
    let assets = load_assets(&asset_server);
    let wall_tex = assets.wall_tex.clone();
    let floor_tex = assets.floor_tex.clone();
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

    // Doors now use the SAME atlas texture as walls (wolf_walls.png).
    // We still keep a separate material handle so door panels remain independently configurable.
    let door_mat = materials.add(StandardMaterial {
        base_color_texture: Some(wall_tex.clone()),
        unlit: true,
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

    // Build wall panels from the atlas (front-facing only)
    let mut atlas_panels: Vec<Handle<Mesh>> = Vec::with_capacity(VSWAP_WALL_CHUNKS);
    for i in 0..VSWAP_WALL_CHUNKS {
        let (u0, u1, v0, v1) = atlas_uv(i);
        atlas_panels.push(build_atlas_panel(&mut meshes, u0, u1, v0, v1, false));
    }

    // Door panels need a mirrored-U back face. We create only the back meshes we need.
    let door98_front = atlas_panels[DOOR_NORMAL_LIGHT].clone();
    let door98_back = {
        let (u0, u1, v0, v1) = atlas_uv(DOOR_NORMAL_LIGHT);
        build_atlas_panel(&mut meshes, u0, u1, v0, v1, true)
    };

    let door99_front = atlas_panels[DOOR_NORMAL_DARK].clone();
    let door99_back = {
        let (u0, u1, v0, v1) = atlas_uv(DOOR_NORMAL_DARK);
        build_atlas_panel(&mut meshes, u0, u1, v0, v1, true)
    };

    let elev102_front = atlas_panels[DOOR_ELEV_LIGHT].clone();
    let elev102_back = {
        let (u0, u1, v0, v1) = atlas_uv(DOOR_ELEV_LIGHT);
        build_atlas_panel(&mut meshes, u0, u1, v0, v1, true)
    };

    let elev103_front = atlas_panels[DOOR_ELEV_DARK].clone();
    let elev103_back = {
        let (u0, u1, v0, v1) = atlas_uv(DOOR_ELEV_DARK);
        build_atlas_panel(&mut meshes, u0, u1, v0, v1, true)
    };

    let silver_front = atlas_panels[DOOR_SILVER].clone();
    let silver_back = {
        let (u0, u1, v0, v1) = atlas_uv(DOOR_SILVER);
        build_atlas_panel(&mut meshes, u0, u1, v0, v1, true)
    };

    let gold_front = atlas_panels[DOOR_GOLD].clone();
    let gold_back = {
        let (u0, u1, v0, v1) = atlas_uv(DOOR_GOLD);
        build_atlas_panel(&mut meshes, u0, u1, v0, v1, true)
    };

    // (Unchanged for now; used by wall-face rebuild)
    // Jamb fallback now comes from the atlas too (tile 100).
    let jamb_mat = wall_mat.clone();
    let jamb_panel = atlas_panels[DOOR_JAMB_LIGHT].clone();

    let wall_base = Quat::from_rotation_x(-FRAC_PI_2); // Make Plane3d Vertical

    // Cache the reusable wall rendering assets so pushwalls can spawn a moving wall,
    // and so we can rebuild static wall faces when pushwalls cross tile boundaries.
    let wall_cache = WallRenderCache {
        atlas_panels: atlas_panels.clone(),
        jamb_panel,
        wall_base,
        wall_mat: wall_mat.clone(),
        wall_mat_dark: wall_mat_dark.clone(),
        jamb_mat,
    };
    commands.insert_resource(wall_cache.clone());

    // Walls + Doors From Grid
    // Doors from grid (static wall faces are spawned separately so we can rebuild them)
    for z in 0..grid.height {
        for x in 0..grid.width {
            let tile = grid.tile(x, z);

            if !matches!(tile, Tile::DoorClosed | Tile::DoorOpen) {
                continue;
            }

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

            // Pick door atlas tile based on Wolf plane0 door code + yaw axis.
            // Codes:
            //   90/91 normal door
            //   92/93 gold key
            //   94/95 silver key
            //   100/101 elevator door
            let code = grid.plane0_code(x, z);
            let is_z_axis = yaw.abs() < 0.001;

            let (front_panel, back_panel) = match code {
                100 | 101 => {
                    if is_z_axis { (&elev102_front, &elev102_back) } else { (&elev103_front, &elev103_back) }
                }
                92 | 93 => (&gold_front, &gold_back),
                94 | 95 => (&silver_front, &silver_back),
                _ => {
                    if is_z_axis { (&door98_front, &door98_back) } else { (&door99_front, &door99_back) }
                }
            };

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
                        Mesh3d(front_panel.clone()),
                        MeshMaterial3d(door_mat.clone()),
                        Transform {
                            translation: normal * half_thickness,
                            rotation: rot,
                            ..default()
                        },
                    ));

                    // Back (Mirrored)
                    parent.spawn((
                        Mesh3d(back_panel.clone()),
                        MeshMaterial3d(door_mat.clone()),
                        Transform {
                            translation: -normal * half_thickness,
                            rotation: Quat::from_rotation_y(PI) * rot,
                            ..default()
                        },
                    ));
                });
        }
    }

    // Static wall faces (includes door jamb faces). Spawned separately so we can rebuild later.
    spawn_wall_faces_for_grid(&mut commands, &grid, &wall_cache, None);

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
        0.5,
        spawn.y as f32 * TILE_SIZE,
    );

    commands.spawn((
        Camera3d::default(),
        IsDefaultUiCamera,
        Player,
        crate::player::PlayerVitals::default(),
        LookAngles::new(spawn_yaw + PI, 0.0),
        SpatialListener::new(0.2),
        Transform::from_translation(player_pos).with_rotation(Quat::from_rotation_y(spawn_yaw + PI)),
    ));
}
