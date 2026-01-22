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
use crate::player::{LookAngles, Player, PlayerKeys};
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
    /// as a static wall face (the moving pushwall will render it)
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
        // Wolf wall-sheet (top-left 8x8 = the 64 wall textures in index order)
        // We remap UVs per wall ID, so this is shared by all wall materials
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
    // Real wall test from the grid
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
    // IMPORTANT: if the neighbor is the moving pushwall tile (skip), treat it as EMPTY
    // so adjacent walls will still spawn their faces toward the moving pushwall
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
        commands.entity(e).try_despawn();
    }

    spawn_wall_faces_for_grid(&mut commands, &grid, &cache, skip);
}

pub fn setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	enemy_sprites: crate::enemies::AllEnemySprites,
	current_level: Res<crate::level::CurrentLevel>,
	mut level_score: ResMut<crate::level_score::LevelScore>,
	skill_level: Res<crate::skill::SkillLevel>,
) {
	// --- Map Load (Wolf Planes) ---
	let (plane0_text, plane1_text) = match current_level.0 {
		// Episode 1
		crate::level::LevelId::E1M1 => (
			include_str!("../assets/maps/episode1/e1m1_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m1_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M2 => (
			include_str!("../assets/maps/episode1/e1m2_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m2_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M3 => (
			include_str!("../assets/maps/episode1/e1m3_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m3_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M4 => (
			include_str!("../assets/maps/episode1/e1m4_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m4_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M5 => (
			include_str!("../assets/maps/episode1/e1m5_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m5_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M6 => (
			include_str!("../assets/maps/episode1/e1m6_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m6_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M7 => (
			include_str!("../assets/maps/episode1/e1m7_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m7_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M8 => (
			include_str!("../assets/maps/episode1/e1m8_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m8_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M9 => (
			include_str!("../assets/maps/episode1/e1m9_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m9_plane1_u16.txt"),
		),
		crate::level::LevelId::E1M10 => (
			include_str!("../assets/maps/episode1/e1m10_plane0_u16.txt"),
			include_str!("../assets/maps/episode1/e1m10_plane1_u16.txt"),
		),

		// Episode 2
		crate::level::LevelId::E2M1 => (
			include_str!("../assets/maps/episode2/e2m1_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m1_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M2 => (
			include_str!("../assets/maps/episode2/e2m2_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m2_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M3 => (
			include_str!("../assets/maps/episode2/e2m3_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m3_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M4 => (
			include_str!("../assets/maps/episode2/e2m4_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m4_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M5 => (
			include_str!("../assets/maps/episode2/e2m5_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m5_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M6 => (
			include_str!("../assets/maps/episode2/e2m6_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m6_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M7 => (
			include_str!("../assets/maps/episode2/e2m7_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m7_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M8 => (
			include_str!("../assets/maps/episode2/e2m8_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m8_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M9 => (
			include_str!("../assets/maps/episode2/e2m9_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m9_plane1_u16.txt"),
		),
		crate::level::LevelId::E2M10 => (
			include_str!("../assets/maps/episode2/e2m10_plane0_u16.txt"),
			include_str!("../assets/maps/episode2/e2m10_plane1_u16.txt"),
		),

		// Episode 3
		crate::level::LevelId::E3M1 => (
			include_str!("../assets/maps/episode3/e3m1_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m1_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M2 => (
			include_str!("../assets/maps/episode3/e3m2_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m2_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M3 => (
			include_str!("../assets/maps/episode3/e3m3_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m3_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M4 => (
			include_str!("../assets/maps/episode3/e3m4_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m4_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M5 => (
			include_str!("../assets/maps/episode3/e3m5_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m5_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M6 => (
			include_str!("../assets/maps/episode3/e3m6_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m6_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M7 => (
			include_str!("../assets/maps/episode3/e3m7_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m7_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M8 => (
			include_str!("../assets/maps/episode3/e3m8_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m8_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M9 => (
			include_str!("../assets/maps/episode3/e3m9_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m9_plane1_u16.txt"),
		),
		crate::level::LevelId::E3M10 => (
			include_str!("../assets/maps/episode3/e3m10_plane0_u16.txt"),
			include_str!("../assets/maps/episode3/e3m10_plane1_u16.txt"),
		),

		// Episode 4
		crate::level::LevelId::E4M1 => (
			include_str!("../assets/maps/episode4/e4m1_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m1_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M2 => (
			include_str!("../assets/maps/episode4/e4m2_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m2_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M3 => (
			include_str!("../assets/maps/episode4/e4m3_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m3_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M4 => (
			include_str!("../assets/maps/episode4/e4m4_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m4_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M5 => (
			include_str!("../assets/maps/episode4/e4m5_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m5_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M6 => (
			include_str!("../assets/maps/episode4/e4m6_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m6_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M7 => (
			include_str!("../assets/maps/episode4/e4m7_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m7_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M8 => (
			include_str!("../assets/maps/episode4/e4m8_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m8_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M9 => (
			include_str!("../assets/maps/episode4/e4m9_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m9_plane1_u16.txt"),
		),
		crate::level::LevelId::E4M10 => (
			include_str!("../assets/maps/episode4/e4m10_plane0_u16.txt"),
			include_str!("../assets/maps/episode4/e4m10_plane1_u16.txt"),
		),

		// Episode 5
		crate::level::LevelId::E5M1 => (
			include_str!("../assets/maps/episode5/e5m1_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m1_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M2 => (
			include_str!("../assets/maps/episode5/e5m2_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m2_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M3 => (
			include_str!("../assets/maps/episode5/e5m3_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m3_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M4 => (
			include_str!("../assets/maps/episode5/e5m4_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m4_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M5 => (
			include_str!("../assets/maps/episode5/e5m5_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m5_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M6 => (
			include_str!("../assets/maps/episode5/e5m6_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m6_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M7 => (
			include_str!("../assets/maps/episode5/e5m7_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m7_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M8 => (
			include_str!("../assets/maps/episode5/e5m8_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m8_plane1_u16.txt"),
		),
		crate::level::LevelId::E5M9 => (
			include_str!("../assets/maps/episode5/e5m9_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m9_plane1_u16.txt"),
		),

		crate::level::LevelId::E5M10 => (
			include_str!("../assets/maps/episode5/e5m10_plane0_u16.txt"),
			include_str!("../assets/maps/episode5/e5m10_plane1_u16.txt"),
		),

		// Episode 6
		crate::level::LevelId::E6M1 => (
			include_str!("../assets/maps/episode6/e6m1_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m1_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M2 => (
			include_str!("../assets/maps/episode6/e6m2_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m2_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M3 => (
			include_str!("../assets/maps/episode6/e6m3_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m3_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M4 => (
			include_str!("../assets/maps/episode6/e6m4_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m4_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M5 => (
			include_str!("../assets/maps/episode6/e6m5_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m5_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M6 => (
			include_str!("../assets/maps/episode6/e6m6_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m6_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M7 => (
			include_str!("../assets/maps/episode6/e6m7_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m7_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M8 => (
			include_str!("../assets/maps/episode6/e6m8_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m8_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M9 => (
			include_str!("../assets/maps/episode6/e6m9_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m9_plane1_u16.txt"),
		),
		crate::level::LevelId::E6M10 => (
			include_str!("../assets/maps/episode6/e6m10_plane0_u16.txt"),
			include_str!("../assets/maps/episode6/e6m10_plane1_u16.txt"),
		),
	};

	let plane0 = MapGrid::parse_u16_grid(plane0_text, 64, 64);
	let plane1 = MapGrid::parse_u16_grid(plane1_text, 64, 64);

	// Make plane1 available as the single source of truth for decorations/pickups later
	commands.insert_resource(crate::level::WolfPlane1(plane1.clone()));

	let pushwall_markers = PushwallMarkers::from_wolf_plane1(64, 64, &plane1);
	let (grid, spawn, guards, mutants, ss, officers, dogs, hans, gretel, mecha_hitler, ghost_hitler, schabbs, otto) =
		MapGrid::from_wolf_planes(64, 64, &plane0, &plane1);

	// --- Enemy difficulty selection ---
	// Wolf thing codes repeat in 3 bands spaced by +36
	// base = 0, mid = 36, hard = 72
	let skill_off = skill_level.spawn_offset();

	let idx = |t: IVec2| -> usize { (t.y as usize) * 64 + (t.x as usize) };

	let guards: Vec<IVec2> = guards
		.into_iter()
		.filter(|&t| {
			let code = plane1[idx(t)];
			let base = (108..=115).contains(&code);
			let med = (144..=151).contains(&code);
			let hard = (180..=187).contains(&code);
			match skill_off {
				0 => base,
				36 => base || med,
				72 => base || med || hard,
				_ => base,
			}
		})
		.collect();

	let mutants: Vec<IVec2> = mutants
		.into_iter()
		.filter(|&t| {
			let code = plane1[idx(t)];
			let base = (216..=223).contains(&code);
			let med = (234..=241).contains(&code);
			let hard = (252..=259).contains(&code);
			match skill_off {
				0 => base,
				36 => base || med,
				72 => base || med || hard,
				_ => base,
			}
		})
		.collect();

	let ss: Vec<IVec2> = ss
		.into_iter()
		.filter(|&t| {
			let code = plane1[idx(t)];
			let base = (126..=133).contains(&code);
			let med = (162..=169).contains(&code);
			let hard = (198..=205).contains(&code);
			match skill_off {
				0 => base,
				36 => base || med,
				72 => base || med || hard,
				_ => base,
			}
		})
		.collect();

	let officers: Vec<IVec2> = officers
		.into_iter()
		.filter(|&t| {
			let code = plane1[idx(t)];
			let base = (116..=123).contains(&code);
			let med = (152..=159).contains(&code);
			let hard = (188..=195).contains(&code);
			match skill_off {
				0 => base,
				36 => base || med,
				72 => base || med || hard,
				_ => base,
			}
		})
		.collect();

	let dogs: Vec<IVec2> = dogs
		.into_iter()
		.filter(|&t| {
			let code = plane1[idx(t)];
			let base = (134..=141).contains(&code);
			let med = (170..=177).contains(&code);
			let hard = (206..=213).contains(&code);
			match skill_off {
				0 => base,
				36 => base || med,
				72 => base || med || hard,
				_ => base,
			}
		})
		.collect();

	// Bosses
	// Not Difficulty-Banded, Spawn Always If Present
	let hans: Vec<IVec2> = hans
		.into_iter()
		.filter(|&t| plane1[idx(t)] == 214)
		.collect();

	let gretel: Vec<IVec2> = gretel
		.into_iter()
		.filter(|&t| plane1[idx(t)] == 197)
		.collect();

	let mecha_hitler: Vec<IVec2> = mecha_hitler
		.into_iter()
		.filter(|&t| plane1[idx(t)] == 178)
		.collect();

    let ghost_hitler: Vec<IVec2> = ghost_hitler
        .into_iter()
        .filter(|&t| plane1[idx(t)] == 160)
        .collect();

    let schabbs: Vec<IVec2> = schabbs
        .into_iter()
        .filter(|&t| plane1[idx(t)] == 196)
        .collect();

	let otto: Vec<IVec2> = otto
        .into_iter()
        .filter(|&t| plane1[idx(t)] == 215)
        .collect();

	let hitler_phase2_total = mecha_hitler.len();

	info!(
		"Enemy Spawns: Guards={}, Mutants={}, SS={}, Officers={}, Dogs={}, Ghost Hitler={}",
		guards.len(),
        mutants.len(),
		ss.len(),
		officers.len(),
		dogs.len(),
        ghost_hitler.len(),
	);

	info!(
		"Boss Spawns: Hans={}, Gretel={}, Mecha Hitler={} (implies Hitler Phase II={}), Schabbs={}, Otto={}",
		hans.len(),
		gretel.len(),
		mecha_hitler.len(),
		hitler_phase2_total,
        schabbs.len(),
		otto.len(),
	);

	info!(
		"Difficulty: {} (spawn_offset={})",
		skill_level.name(),
		skill_level.spawn_offset()
	);

	// Intermission Screen Totals
    // FIXME: Is this right? Should bosses be counted in this way?
	let kills_total = guards.len()
        + mutants.len()
        + ss.len()
        + officers.len()
        + dogs.len()
        + hans.len()
		+ gretel.len()
		+ mecha_hitler.len()
    	+ hitler_phase2_total
        + ghost_hitler.len()
        + schabbs.len()
		+ otto.len();

	let secrets_total = plane1.iter().filter(|&&c| c == 98).count();
	let treasure_total = plane1
		.iter()
		.filter(|&&c| matches!(c, 52 | 53 | 54 | 55))
		.count();

	level_score.reset_for_level(kills_total, secrets_total, treasure_total);

	let (spawn, spawn_yaw) = spawn.unwrap_or((IVec2::new(1, 1), 0.0));

	// Make Map Available for Collision / Doors / Raycasts
	commands.insert_resource(grid.clone());
	// Blocking statics (decorations) occupancy
	commands.insert_resource(crate::decorations::SolidStatics::new(
		grid.width,
		grid.height,
	));
	// Pushwall markers (plane1 == 98)
	commands.insert_resource(pushwall_markers);

	// --- Assets / materials ---
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

	// Doors use the SAME atlas texture as walls (wolf_walls.png)
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

	    // Floor + ceiling share the same mesh handle
    let floor_mesh = meshes.add(
        Plane3d::default()
            .mesh()
            .size(
                grid.width as f32 * TILE_SIZE,
                grid.height as f32 * TILE_SIZE,
            ),
    );

    // Floor
    commands.spawn((
        Name::new("floor"),
        Mesh3d(floor_mesh.clone()),
        MeshMaterial3d(floor_mat),
        Transform::from_translation(room_center),
    ));

    // Ceiling Plane Tinted per Level
    let ceiling_mat = materials.add(StandardMaterial {
        base_color: current_level.0.ceiling_color(),
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Name::new("ceiling"),
        Mesh3d(floor_mesh),
        MeshMaterial3d(ceiling_mat),
        Transform::from_translation(room_center + Vec3::new(0.0, WALL_H, 0.0))
            .with_rotation(Quat::from_rotation_x(PI)),
    ));

	// --- Wall atlas mapping (WL6 VSWAP walls 0..105 packed 16x7, 64x64 each) ---
	const VSWAP_WALL_CHUNKS: usize = 106;
	const ATLAS_COLS: usize = 16;
	const ATLAS_ROWS: usize = (VSWAP_WALL_CHUNKS + ATLAS_COLS - 1) / ATLAS_COLS;

	fn atlas_uv(index: usize) -> (f32, f32, f32, f32) {
		const TILE_PX: f32 = 64.0;
		const ATLAS_W_PX: f32 = ATLAS_COLS as f32 * TILE_PX;
		const ATLAS_H_PX: f32 = ATLAS_ROWS as f32 * TILE_PX;
		const HALF_U: f32 = 0.5 / ATLAS_W_PX;
		const HALF_V: f32 = 0.5 / ATLAS_H_PX;

		let col = index % ATLAS_COLS;
		let row = index / ATLAS_COLS;

		let u0 = col as f32 / ATLAS_COLS as f32 + HALF_U;
		let u1 = (col + 1) as f32 / ATLAS_COLS as f32 - HALF_U;

		let v_top = row as f32 / ATLAS_ROWS as f32;
		let v_bottom = (row + 1) as f32 / ATLAS_ROWS as f32;

		let v0 = v_bottom - HALF_V;
		let v1 = v_top + HALF_V;

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
			(
				f32::INFINITY,
				f32::NEG_INFINITY,
				f32::INFINITY,
				f32::NEG_INFINITY,
			),
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

		if let Some(VertexAttributeValues::Float32x2(uvs)) = m.attribute_mut(Mesh::ATTRIBUTE_UV_0)
		{
			for (p, uv) in positions.iter().zip(uvs.iter_mut()) {
				let mut sx = (p[0] - min_x) / dx;
				if flip_u {
					sx = 1.0 - sx;
				}
				let sz = (p[2] - min_z) / dz;
				uv[0] = u0 + sx * (u1 - u0);
				uv[1] = v0 + sz * (v1 - v0);
			}
		}

		meshes.add(m)
	}

	let mut atlas_panels: Vec<Handle<Mesh>> = Vec::with_capacity(VSWAP_WALL_CHUNKS);
	for i in 0..VSWAP_WALL_CHUNKS {
		let (u0, u1, v0, v1) = atlas_uv(i);
		atlas_panels.push(build_atlas_panel(&mut meshes, u0, u1, v0, v1, false));
	}

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

	let elev102_u = atlas_panels[DOOR_ELEV_LIGHT].clone();
	let elev102_u_flip = {
		let (u0, u1, v0, v1) = atlas_uv(DOOR_ELEV_LIGHT);
		build_atlas_panel(&mut meshes, u0, u1, v0, v1, true)
	};

	let elev103_u = atlas_panels[DOOR_ELEV_DARK].clone();
	let elev103_u_flip = {
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

	let wall_base = Quat::from_rotation_x(-FRAC_PI_2);

	let jamb_mat = wall_mat.clone();
	let jamb_panel = atlas_panels[DOOR_JAMB_LIGHT].clone();

	let wall_cache = WallRenderCache {
		atlas_panels: atlas_panels.clone(),
		jamb_panel,
		wall_base,
		wall_mat: wall_mat.clone(),
		wall_mat_dark: wall_mat_dark.clone(),
		jamb_mat,
	};
	commands.insert_resource(wall_cache.clone());

	// --- Doors from grid ---
	for z in 0..grid.height {
		for x in 0..grid.width {
			let tile = grid.tile(x, z);
			if !matches!(tile, Tile::DoorClosed | Tile::DoorOpen) {
				continue;
			}

			let is_open = matches!(tile, Tile::DoorOpen);

			// Determine orientation from adjacent walls
			let left_wall = x > 0 && matches!(grid.tile(x - 1, z), Tile::Wall);
			let right_wall = x + 1 < grid.width && matches!(grid.tile(x + 1, z), Tile::Wall);
			let up_wall = z > 0 && matches!(grid.tile(x, z - 1), Tile::Wall);
			let down_wall = z + 1 < grid.height && matches!(grid.tile(x, z + 1), Tile::Wall);

			let walls_x = (left_wall as u8) + (right_wall as u8);
			let walls_z = (up_wall as u8) + (down_wall as u8);

			let code = grid.plane0_code(x, z);

			if walls_x == 0 && walls_z == 0 {
				warn!(
					"Door at ({},{}) plane0_code={} has no adjacent walls?",
					x, z, code
				);
			}

			let yaw_base = if walls_z > walls_x { FRAC_PI_2 } else { 0.0 };
			let yaw = yaw_base + PI;

			let base = Quat::from_rotation_x(-FRAC_PI_2);
			let rot = Quat::from_rotation_y(yaw) * base;
			let normal = rot * Vec3::Y;

			let half_thickness = (DOOR_THICKNESS * TILE_SIZE) * 0.5;

			let center = Vec3::new(
				x as f32 * TILE_SIZE,
				WALL_H * 0.5,
				z as f32 * TILE_SIZE,
			);

			// Door slides along local +X after yaw
			// Using yaw (not yaw_base) preserves the pre-regression sign convention
			let mut slide_axis = Quat::from_rotation_y(yaw) * Vec3::X;

			// Wolf door codes are paired per lock type for which side they retract into
			// Odd partner flips the retract direction on the same axis
			let is_paired_door_code = matches!(code, 90..=95 | 100 | 101);
			if is_paired_door_code && ((code & 1) == 1) {
				slide_axis = -slide_axis;
			}

			let progress = if is_open { 1.0 } else { 0.0 };
			let start_pos = center + slide_axis * (progress * TILE_SIZE);
			let vis = if is_open { Visibility::Hidden } else { Visibility::Visible };

			// Choose door atlas tile by Wolf plane0 door code + axis
			// Codes:
			//   90/91 normal door
			//   92/93 gold key
			//   94/95 silver key
			//   100/101 elevator door
			let eps = 0.001;
			let yaw_n = yaw_base.rem_euclid(PI);
			let is_z_axis = yaw_n.abs() < eps;

			let (panel_u, panel_u_flip) = match code {
				100 | 101 => {
					if is_z_axis {
						(&elev102_u, &elev102_u_flip)
					} else {
						(&elev103_u, &elev103_u_flip)
					}
				}
				92 | 93 => (&gold_front, &gold_back),
				94 | 95 => (&silver_front, &silver_back),
				_ => {
					if is_z_axis {
						(&door98_front, &door98_back)
					} else {
						(&door99_front, &door99_back)
					}
				}
			};

			let need_flip_u = |n: Vec3| -> bool {
				let ax = n.x.abs();
				let az = n.z.abs();

				if ax > az {
					n.x < 0.0
				} else {
					n.z > 0.0
				}
			};

			let front_panel = if need_flip_u(normal) {
				panel_u_flip.clone()
			} else {
				panel_u.clone()
			};

			let back_panel = if need_flip_u(-normal) {
				panel_u_flip.clone()
			} else {
				panel_u.clone()
			};

			commands
				.spawn((
					DoorTile(IVec2::new(x as i32, z as i32)),
					DoorState {
						open_timer: 0.0,
						want_open: is_open,
					},
					DoorAnim {
						progress,
						closed_pos: center,
						slide_axis,
					},
					Transform::from_translation(start_pos),
					vis,
				))
				.with_children(|parent| {
					parent.spawn((
						Mesh3d(front_panel),
						MeshMaterial3d(door_mat.clone()),
						Transform {
							translation: normal * half_thickness,
							rotation: rot,
							..default()
						},
					));

					parent.spawn((
						Mesh3d(back_panel),
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

	// Static wall faces (includes door jamb faces)
	spawn_wall_faces_for_grid(&mut commands, &grid, &wall_cache, None);

	for g in guards {
		crate::enemies::spawn_guard(&mut commands, &mut meshes, &mut materials, &enemy_sprites.guards, g);
	}

    for m in mutants {
        crate::enemies::spawn_mutant(&mut commands, &mut meshes, &mut materials, &enemy_sprites.mutants, m);
    }

	for s in ss {
		crate::enemies::spawn_ss(&mut commands, &mut meshes, &mut materials, &enemy_sprites.ss, s);
	}

	for o in officers {
		crate::enemies::spawn_officer(&mut commands, &mut meshes, &mut materials, &enemy_sprites.officers, o);
	}

	for d in dogs {
		crate::enemies::spawn_dog(&mut commands, &mut meshes, &mut materials, &enemy_sprites.dogs, d);
	}

	for h in hans {
		crate::enemies::spawn_hans(&mut commands, &mut meshes, &mut materials, &enemy_sprites.hans, h);
	}

	for g in gretel {
		crate::enemies::spawn_gretel(&mut commands, &mut meshes, &mut materials, &enemy_sprites.gretel, g);
	}

	for mh in mecha_hitler {
		crate::enemies::spawn_mecha_hitler(&mut commands, &mut meshes, &mut materials, &enemy_sprites.mecha_hitler, mh);
	}

    for gh in ghost_hitler {
        crate::enemies::spawn_ghost_hitler(&mut commands, &mut meshes, &mut materials, &enemy_sprites.ghost_hitler, gh);
    }

    for sc in schabbs {
        crate::enemies::spawn_schabbs(&mut commands, &mut meshes, &mut materials, &enemy_sprites.schabbs, sc);
    }

	for ot in otto {
        crate::enemies::spawn_otto(&mut commands, &mut meshes, &mut materials, &enemy_sprites.otto, ot);
    }

	let player_pos = Vec3::new(spawn.x as f32 * TILE_SIZE, 0.5, spawn.y as f32 * TILE_SIZE);

	commands.spawn((
		Camera3d::default(),
		IsDefaultUiCamera,
		Player,
		PlayerKeys::default(),
		crate::player::PlayerVitals::default(),
		LookAngles::new(spawn_yaw, 0.0),
		SpatialListener::new(0.2),
		Transform::from_translation(player_pos).with_rotation(Quat::from_rotation_y(spawn_yaw)),
	));
}
