/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

use crate::map::MapGrid;

/// Tile-occupancy for Wolf-style blocking "statics" (decorations).
///
/// Design goal for the first milestone:
/// - If a decoration is marked "block", it blocks:
///   - player movement
///   - enemy movement
///   - hitscan / line of sight
///
/// This matches the original Wolf3D behavior (actorat[tile]=1).
#[derive(Component)]
pub struct BillboardUpright;

#[derive(Component)]
pub struct BillboardFloor;

#[derive(Component, Copy, Clone)]
pub struct BillboardTilt(pub f32); // radians around X; 0 = upright, -PI/2 = flat

#[derive(Resource, Debug, Clone)]
pub struct SolidStatics {
    width: usize,
    height: usize,
    solid: Vec<bool>,
}

impl SolidStatics {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            solid: vec![false; width * height],
        }
    }

    #[inline]
    fn idx(&self, x: usize, z: usize) -> usize {
        z * self.width + x
    }

    pub fn clear(&mut self) {
        self.solid.fill(false);
    }

    pub fn set_solid(&mut self, x: i32, z: i32, v: bool) {
        if x < 0 || z < 0 || x >= self.width as i32 || z >= self.height as i32 {
            return;
        }
        let i = self.idx(x as usize, z as usize);
        self.solid[i] = v;
    }

    pub fn is_solid(&self, x: i32, z: i32) -> bool {
        if x < 0 || z < 0 || x >= self.width as i32 || z >= self.height as i32 {
            return true; // outside map blocks
        }
        self.solid[self.idx(x as usize, z as usize)]
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Decoration {
    pub plane1_code: u16,
    pub blocks: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatKind {
    Dressing,
    Block,
    Pickup,
}

fn stat_idx_from_plane1(code: u16) -> Option<usize> {
    if code < 23 {
        return None;
    }
    Some((code - 23) as usize)
}

fn choose_tile_path_from_plane1(code: u16) -> Option<&'static str> {
    let idx = stat_idx_from_plane1(code)?;

    // Wolf (non-Spear) statics: indices 0..47 are the ones you pasted in WL_ACT1.C.
    // If you later add Spear-only statics, extend this.
    if idx > 47 {
        return None;
    }

    // Use a simple numeric scheme so the code never depends on file ordering.
    // You rename your files to match this scheme.
    const PATHS: [&str; 48] = [
        "textures/decorations/stat_00_puddle.png",
        "textures/decorations/stat_01_green_barrel.png",
        "textures/decorations/stat_02_table_chairs.png",
        "textures/decorations/stat_03_floor_lamp.png",
        "textures/decorations/stat_04_chandelier.png",
        "textures/decorations/stat_05_hanged_man.png",
        "textures/decorations/stat_06_bad_food.png",
        "textures/decorations/stat_07_red_pillar.png",
        "textures/decorations/stat_08_tree.png",
        "textures/decorations/stat_09_skeleton_flat.png",
        "textures/decorations/stat_10_sink.png",
        "textures/decorations/stat_11_potted_plant.png",
        "textures/decorations/stat_12_urn.png",
        "textures/decorations/stat_13_bare_table.png",
        "textures/decorations/stat_14_ceiling_light.png",
        "textures/decorations/stat_15_kitchen_stuff.png",
        "textures/decorations/stat_16_suit_of_armor.png",
        "textures/decorations/stat_17_hanging_cage.png",
        "textures/decorations/stat_18_skeleton_in_cage.png",
        "textures/decorations/stat_19_skeleton_relax.png",
        "textures/decorations/stat_20_key1.png",
        "textures/decorations/stat_21_key2.png",
        "textures/decorations/stat_22_stuff_a.png",
        "textures/decorations/stat_23_stuff_b.png",
        "textures/decorations/stat_24_good_food.png",
        "textures/decorations/stat_25_first_aid.png",
        "textures/decorations/stat_26_clip.png",
        "textures/decorations/stat_27_machine_gun.png",
        "textures/decorations/stat_28_chaingun.png",
        "textures/decorations/stat_29_cross.png",
        "textures/decorations/stat_30_chalice.png",
        "textures/decorations/stat_31_bible.png",
        "textures/decorations/stat_32_crown.png",
        "textures/decorations/stat_33_one_up.png",
        "textures/decorations/stat_34_gibs.png",
        "textures/decorations/stat_35_barrel.png",
        "textures/decorations/stat_36_well.png",
        "textures/decorations/stat_37_empty_well.png",
        "textures/decorations/stat_38_gibs2.png",
        "textures/decorations/stat_39_flag.png",
        "textures/decorations/stat_40_call_apogee.png",
        "textures/decorations/stat_41_junk_a.png",
        "textures/decorations/stat_42_junk_b.png",
        "textures/decorations/stat_43_junk_c.png",
        "textures/decorations/stat_44_pots.png",
        "textures/decorations/stat_45_stove.png",
        "textures/decorations/stat_46_spears.png",
        "textures/decorations/stat_47_vines.png",
    ];

    Some(PATHS[idx])
}

/// Wolf3D WL_ACT1.C `statinfo[]` distilled to what we need:
/// - Block vs Dressing vs Pickup
///
/// Index is: `idx = plane1_code - 23`.
const STAT_KIND: [StatKind; 49] = [
    StatKind::Dressing, // 0 puddle
    StatKind::Block,    // 1 green barrel
    StatKind::Block,    // 2 table/chairs
    StatKind::Block,    // 3 floor lamp
    StatKind::Dressing, // 4 chandelier
    StatKind::Block,    // 5 hanged man
    StatKind::Pickup,   // 6 bad food (alpo)
    StatKind::Block,    // 7 red pillar
    StatKind::Block,    // 8 tree
    StatKind::Dressing, // 9 skeleton flat
    StatKind::Block,    // 10 sink
    StatKind::Block,    // 11 potted plant
    StatKind::Block,    // 12 urn
    StatKind::Block,    // 13 bare table
    StatKind::Dressing, // 14 ceiling light
    StatKind::Dressing, // 15 kitchen stuff (WL6)
    StatKind::Block,    // 16 suit of armor
    StatKind::Block,    // 17 hanging cage
    StatKind::Block,    // 18 skeleton in cage
    StatKind::Dressing, // 19 skeleton relax
    StatKind::Pickup,   // 20 key 1
    StatKind::Pickup,   // 21 key 2
    StatKind::Block,    // 22 "stuff" (WL6)
    StatKind::Dressing, // 23 "stuff"
    StatKind::Pickup,   // 24 good food
    StatKind::Pickup,   // 25 first aid
    StatKind::Pickup,   // 26 clip
    StatKind::Pickup,   // 27 machine gun
    StatKind::Pickup,   // 28 chaingun
    StatKind::Pickup,   // 29 cross
    StatKind::Pickup,   // 30 chalice
    StatKind::Pickup,   // 31 bible
    StatKind::Pickup,   // 32 crown
    StatKind::Pickup,   // 33 1UP
    StatKind::Pickup,   // 34 gibs
    StatKind::Block,    // 35 barrel
    StatKind::Block,    // 36 well
    StatKind::Block,    // 37 empty well
    StatKind::Pickup,   // 38 gibs 2
    StatKind::Block,    // 39 flag
    StatKind::Block,    // 40 call apogee (WL6)
    StatKind::Dressing, // 41 junk
    StatKind::Dressing, // 42 junk
    StatKind::Dressing, // 43 junk
    StatKind::Dressing, // 44 pots (WL6)
    StatKind::Block,    // 45 stove
    StatKind::Block,    // 46 spears
    StatKind::Dressing, // 47 vines
    StatKind::Pickup,   // 48 clip2
];

pub fn billboard_floor_decals(
    q_player: Query<&Transform, (With<crate::player::Player>, Without<BillboardFloor>)>,
    mut q_floor: Query<&mut Transform, (With<BillboardFloor>, Without<crate::player::Player>)>,
) {
    let Some(player_tf) = q_player.iter().next() else { return; };
    let player_pos = player_tf.translation;

    for mut tf in q_floor.iter_mut() {
        // Flat decal: rotate around Y so its "long axis" aims at the player.
        let mut to_player = player_pos - tf.translation;
        to_player.y = 0.0;

        let len2 = to_player.length_squared();
        if len2 > 0.0001 {
            let dir = to_player / len2.sqrt();
            let yaw = dir.x.atan2(dir.z);

            // Keep it flat: we want rotation = (flat on ground) * (yaw)
            let flat = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
            tf.rotation = Quat::from_rotation_y(yaw) * flat;
        }
    }
}

#[allow(dead_code)]
fn choose_static_path_from_plane1(code: u16) -> Option<String> {
    if code < 23 {
        return None;
    }
    let idx = (code - 23) as usize;
    if idx > 47 {
        return None;
    }
    Some(format!("textures/decorations/stat_{:02}.png", idx))
}

/// Spawn Wolf3D E1M1 "statics" (decorations) from plane1 codes using WL_ACT1.C `statinfo[]`.
///
/// This does *not* spawn pickups/treasure/weapons (those are handled by your pickups module).
pub fn spawn_wolf_e1m1_decorations(
    mut commands: Commands,
    grid: Res<MapGrid>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut solid: ResMut<SolidStatics>,
) {
    const E1M1_PLANE1: &str = include_str!("../assets/maps/e1m1_plane1_u16.txt");

    if grid.width != 64 || grid.height != 64 {
        warn!(
            "spawn_wolf_e1m1_decorations: expected 64x64 grid for E1M1, got {}x{}",
            grid.width, grid.height
        );
        return;
    }

    solid.clear();

    let plane1 = crate::map::MapGrid::parse_u16_grid(E1M1_PLANE1, 64, 64);
    let idx = |x: usize, z: usize| -> usize { z * 64 + x };

    // Wolf statics: idx = plane1_code - 23
    // idx 0 = puddle, idx 9 = skeleton flat
    fn is_floor_decal_plane1(code: u16) -> bool {
        matches!(code, 23 | 32)
    }

    // Upright sprites (billboarded): square-ish
    let w = 0.95_f32;
    let h = 0.95_f32;
    let quad_upright = meshes.add(Rectangle::new(w, h));

    // Floor decals: make puddle much "deeper" so it reads from a shallow angle.
    let quad_decal_default = meshes.add(Rectangle::new(0.95, 1.20));
    let quad_decal_puddle = meshes.add(Rectangle::new(0.95, 3.50));
    let quad_decal_skel = meshes.add(Rectangle::new(0.95, 2.00));

    // Small epsilon to avoid z-fighting with the floor
    let floor_y = 0.01_f32;

    for z in 0..64 {
        for x in 0..64 {
            let code = plane1[idx(x, z)];
            if code < 23 {
                continue; // actors / player start etc.
            }

            let si = (code - 23) as usize;
            if si >= STAT_KIND.len() {
                continue;
            }

            let kind = STAT_KIND[si];
            if kind == StatKind::Pickup {
                continue; // pickups module handles these
            }

            let blocks = kind == StatKind::Block;
            if blocks {
                solid.set_solid(x as i32, z as i32, true);
            }

            let floor_decal = !blocks && is_floor_decal_plane1(code);

            let Some(tex_path) = choose_tile_path_from_plane1(code) else {
                continue;
            };
            let tex: Handle<Image> = asset_server.load(tex_path);

            let mat = materials.add(StandardMaterial {
                base_color_texture: Some(tex),
                alpha_mode: AlphaMode::Mask(0.5),
                unlit: true,
                cull_mode: None,
                ..default()
            });

            if floor_decal {
                let decal_mesh = match code {
                    23 => quad_decal_puddle.clone(), // puddle
                    32 => quad_decal_skel.clone(),   // skeleton flat
                    _ => quad_decal_default.clone(),
                };

                commands.spawn((
                    Name::new("Decoration_FloorDecal"),
                    Decoration { plane1_code: code, blocks },
                    // Flat decal: billboard system will set yaw + this tilt each frame.
                    BillboardTilt(-std::f32::consts::FRAC_PI_2),
                    Mesh3d(decal_mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(Vec3::new(x as f32, floor_y, z as f32)),
                    GlobalTransform::default(),
                ));
            } else {
                // Upright sprite: bottom at y=0
                let y = h * 0.5;

                commands.spawn((
                    Name::new(if blocks { "Decoration_Block" } else { "Decoration" }),
                    Decoration { plane1_code: code, blocks },
                    BillboardTilt(0.0),
                    Mesh3d(quad_upright.clone()),
                    MeshMaterial3d(mat),
                    Transform::from_translation(Vec3::new(x as f32, y, z as f32)),
                    GlobalTransform::default(),
                ));
            }
        }
    }
}

pub fn billboard_decorations(
    q_player: Query<&Transform, (With<crate::player::Player>, Without<Decoration>)>,
    mut q_decor: Query<(&mut Transform, &BillboardTilt), (With<Decoration>, Without<crate::player::Player>)>,
) {
    let Some(player_tf) = q_player.iter().next() else { return; };
    let player_pos = player_tf.translation;

    for (mut tf, tilt) in q_decor.iter_mut() {
        let mut to_player = player_pos - tf.translation;
        to_player.y = 0.0;

        let len2 = to_player.length_squared();
        if len2 > 0.0001 {
            let dir = to_player / len2.sqrt();
            let yaw = dir.x.atan2(dir.z);

            // Same billboard yaw as everything else + fixed tilt.
            tf.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(tilt.0);
        }
    }
}
