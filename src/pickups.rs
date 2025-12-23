/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

use crate::combat::WeaponSlot;
use crate::ui::HudState;
use davelib::audio::{PlaySfx, SfxKind};
use davelib::enemies::GuardCorpse;
use davelib::map::{MapGrid, Tile};
use davelib::player::Player;

// Ammo Pickup Amounts
#[allow(dead_code)]
const MAP_AMMO_ROUNDS: i32 = 8;
const GUARD_DROP_AMMO_ROUNDS: i32 = 4;

// Visual Size, Height in World Units
// Width Derived From Sprite Aspect
const PICKUP_H: f32 = 0.28;
const AMMO_H: f32 = 0.22;
const HEALTH_FIRST_AID_H: f32 = 0.18;
const HEALTH_DINNER_H: f32    = 0.18;
const HEALTH_DOGFOOD_H: f32   = AMMO_H;
const ONEUP_H: f32            = 0.50;
const TREASURE_H: f32 = 0.24;

const HEALTH_FIRST_AID_W_SCALE: f32 = 3.6;
const HEALTH_DINNER_W_SCALE: f32    = 4.0;

// Aspect Ratios
const CHAINGUN_ASPECT: f32 = 60.0 / 21.0;
const MACHINEGUN_ASPECT: f32 = 47.0 / 18.0;
const AMMO_ASPECT: f32 = 16.0 / 12.0;
const CROSS_ASPECT: f32   = 20.0 / 19.0;
const CHALICE_ASPECT: f32 = 18.0 / 15.0;
const CHEST_ASPECT: f32   = 25.0 / 13.0;
const CROWN_ASPECT: f32   = 24.0 / 17.0;

#[derive(Component, Debug, Clone, Copy)]
pub struct Pickup {
    // (X, Z) Tile Coords
    pub tile: IVec2,
    pub kind: PickupKind,
}

#[derive(Debug, Clone, Copy)]
pub enum PickupKind {
    Weapon(WeaponSlot),
    // +8 Map Spawn, +4 Enemy Drop
    Ammo { rounds: i32 },
    Treasure(TreasureKind),
    Health(HealthKind),
    ExtraLife,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DroppedLoot;

#[derive(Debug, Clone, Copy)]
pub enum HealthKind {
    FirstAid,
    Dinner,
    DogFood,
}

impl HealthKind {
    pub const fn heal(self) -> i32 {
        match self {
            HealthKind::FirstAid => 25,
            HealthKind::Dinner => 10,
            HealthKind::DogFood => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreasureKind {
    Cross,
    Chalice,
    Chest,
    Crown,
}

impl TreasureKind {
    pub const fn points(self) -> i32 {
        match self {
            TreasureKind::Cross => 100,
            TreasureKind::Chalice => 500,
            TreasureKind::Chest => 1000,
            TreasureKind::Crown => 5000,
        }
    }
}

fn ammo_size() -> (f32, f32) {
    (AMMO_H * AMMO_ASPECT, AMMO_H)
}

fn weapon_pickup_size(w: WeaponSlot) -> (f32, f32) {
    match w {
        WeaponSlot::Chaingun => (PICKUP_H * CHAINGUN_ASPECT, PICKUP_H),
        WeaponSlot::MachineGun => (PICKUP_H * MACHINEGUN_ASPECT, PICKUP_H),
        _ => (PICKUP_H, PICKUP_H),
    }
}

fn health_pickup_size(h: HealthKind) -> (f32, f32) {
    match h {
        HealthKind::DogFood => {
            let h = HEALTH_DOGFOOD_H;
            (h, h)
        }
        HealthKind::Dinner => {
            let h = HEALTH_DINNER_H;
            (h * HEALTH_DINNER_W_SCALE, h)
        }
        HealthKind::FirstAid => {
            let h = HEALTH_FIRST_AID_H;
            (h * HEALTH_FIRST_AID_W_SCALE, h)
        }
    }
}

fn oneup_size() -> (f32, f32) {
    (ONEUP_H, ONEUP_H)
}

fn treasure_size(t: TreasureKind) -> (f32, f32) {
    let aspect = match t {
        TreasureKind::Cross => CROSS_ASPECT,
        TreasureKind::Chalice => CHALICE_ASPECT,
        TreasureKind::Chest => CHEST_ASPECT,
        TreasureKind::Crown => CROWN_ASPECT,
    };
    (TREASURE_H * aspect, TREASURE_H)
}

fn weapon_pickup_texture(w: WeaponSlot) -> &'static str {
    match w {
        WeaponSlot::Chaingun => "textures/pickups/chaingun.png",
        WeaponSlot::MachineGun => "textures/pickups/machinegun.png",
        _ => "textures/pickups/chaingun.png", // FIXME: Placeholder
    }
}

fn ammo_texture() -> &'static str {
    "textures/pickups/ammo.png"
}

fn health_texture(h: HealthKind) -> &'static str {
    match h {
        HealthKind::FirstAid => "textures/pickups/health_first_aid.png",
        HealthKind::Dinner => "textures/pickups/health_dinner.png",
        HealthKind::DogFood => "textures/pickups/health_dog_food.png",
    }
}


fn treasure_texture(t: TreasureKind) -> &'static str {
    match t {
        TreasureKind::Cross => "textures/pickups/treasure_cross.png",
        TreasureKind::Chalice => "textures/pickups/treasure_chalice.png",
        TreasureKind::Chest => "textures/pickups/treasure_chest.png",
        TreasureKind::Crown => "textures/pickups/treasure_crown.png",
    }
}

fn oneup_texture() -> &'static str {
    "textures/pickups/oneup.png"
}


fn world_to_tile_xz(pos_xz: Vec2) -> IVec2 {
    IVec2::new((pos_xz.x + 0.5).floor() as i32, (pos_xz.y + 0.5).floor() as i32)
}

fn pickup_base_rot() -> Quat {
    Quat::from_rotation_x(FRAC_PI_2)
}

fn find_empty_tile_not_used(grid: &MapGrid, used: &[IVec2], avoid: IVec2) -> Option<IVec2> {
    for z in 0..grid.height {
        for x in 0..grid.width {
            if !matches!(grid.tile(x, z), Tile::Empty) {
                continue;
            }
            let t = IVec2::new(x as i32, z as i32);
            if t == avoid {
                continue;
            }
            if used.contains(&t) {
                continue;
            }
            return Some(t);
        }
    }
    None
}

pub fn drop_guard_ammo(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_corpses: Query<(Entity, &GlobalTransform), (With<GuardCorpse>, Without<DroppedLoot>)>,
) {
    // Depth Tweak: with AlphaMode::Mask this Will Actually Affect Depth Testing
    const DROP_DEPTH_BIAS: f32 = -250.0;

    // Tiny Lift to Avoid Z-Fighting with Floor
    const DROP_Y_LIFT: f32 = 0.01;

    for (e, gt) in q_corpses.iter() {
        // Drop Once per Corpse
        commands.entity(e).insert(DroppedLoot);

        // Drop at the Corpse Tile
        let p = gt.translation();
        let tile = world_to_tile_xz(Vec2::new(p.x, p.z));

        let rounds = GUARD_DROP_AMMO_ROUNDS;

        let (w, h) = ammo_size();
        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(ammo_texture());

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),

            // Mask writes depth, so corpse can't overwrite later
            // Choose Cutoff that Keeps Edges Crisp. Adjust to 0.25 if "holes"
            alpha_mode: AlphaMode::Mask(0.5),

            unlit: true,
            cull_mode: None,

            // Make Slightly "Closer" in Depth Than Corpse at Same Tile
            depth_bias: DROP_DEPTH_BIAS,

            ..default()
        });

        let y = (h * 0.5) + DROP_Y_LIFT;

        commands.spawn((
            Name::new("Pickup_Drop_Ammo"),
            Pickup {
                tile,
                kind: PickupKind::Ammo { rounds },
            },
            Mesh3d(quad),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(tile.x as f32, y, tile.y as f32))
                .with_rotation(pickup_base_rot()),
            GlobalTransform::default(),
        ));
    }
}

// To Test While Developing
pub fn spawn_test_weapon_pickup(
    mut commands: Commands,
    grid: Res<MapGrid>,
    asset_server: Res<AssetServer>,
    q_player: Query<&Transform, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut it = q_player.iter();
    let Some(player_tf) = it.next() else {
        warn!("spawn_test_weapon_pickup: no Player entity found");
        return;
    };
    if it.next().is_some() {
        warn!("spawn_test_weapon_pickup: multiple Player entities found; using the first");
    }

    let player_tile = world_to_tile_xz(Vec2::new(
        player_tf.translation.x,
        player_tf.translation.z,
    ));

    // Track Used Tiles, Fallback Placement Doesn't Stack Items
    let mut used_tiles: Vec<IVec2> = vec![player_tile];

    // --------------------
    // Weapons (test)
    // --------------------
    let desired_weapons: &[(WeaponSlot, IVec2)] = &[
        (WeaponSlot::Chaingun, IVec2::new(27, 14)),
        (WeaponSlot::MachineGun, IVec2::new(29, 14)),
    ];

    for &(weapon, mut tile) in desired_weapons {
        let in_bounds = tile.x >= 0
            && tile.y >= 0
            && (tile.x as usize) < grid.width
            && (tile.y as usize) < grid.height;

        let ok_tile = in_bounds
            && tile != player_tile
            && matches!(grid.tile(tile.x as usize, tile.y as usize), Tile::Empty)
            && !used_tiles.contains(&tile);

        if !ok_tile {
            let Some(fallback) = find_empty_tile_not_used(&grid, &used_tiles, player_tile) else {
                warn!("spawn_test_weapon_pickup: no empty tiles found for {:?}", weapon);
                continue;
            };
            warn!(
                "spawn_test_weapon_pickup: {:?} wanted {:?}, using fallback {:?}",
                weapon, tile, fallback
            );
            tile = fallback;
        }

        used_tiles.push(tile);

        let (w, h) = weapon_pickup_size(weapon);
        let tex_path = weapon_pickup_texture(weapon);

        info!(
            "Spawning TEST weapon pickup at tile {:?} ({:?}) using {}",
            tile, weapon, tex_path
        );

        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(tex_path);

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        let y = h * 0.5;

        commands.spawn((
            Name::new(format!("Pickup_Test_{:?}", weapon)),
            Pickup {
                tile,
                kind: PickupKind::Weapon(weapon),
            },
            Mesh3d(quad),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(tile.x as f32, y, tile.y as f32))
                .with_rotation(pickup_base_rot()),
            GlobalTransform::default(),
        ));
    }

    // --------------------
    // Ammo (test)
    // --------------------
    let desired_ammo: &[(IVec2, i32)] = &[
        (IVec2::new(26, 16), 8),
        (IVec2::new(26, 17), 8),
        (IVec2::new(26, 18), 8),
    ];

    for &(mut tile, rounds) in desired_ammo {
        let in_bounds = tile.x >= 0
            && tile.y >= 0
            && (tile.x as usize) < grid.width
            && (tile.y as usize) < grid.height;

        let ok_tile = in_bounds
            && tile != player_tile
            && matches!(grid.tile(tile.x as usize, tile.y as usize), Tile::Empty)
            && !used_tiles.contains(&tile);

        if !ok_tile {
            let Some(fallback) = find_empty_tile_not_used(&grid, &used_tiles, player_tile) else {
                warn!("spawn_test_weapon_pickup: no empty tiles found for Ammo");
                continue;
            };
            warn!(
                "spawn_test_weapon_pickup: Ammo wanted {:?}, using fallback {:?}",
                tile, fallback
            );
            tile = fallback;
        }

        used_tiles.push(tile);

        let (w, h) = ammo_size();
        let tex_path = ammo_texture();

        info!("Spawning TEST ammo at tile {:?} (+{}) using {}", tile, rounds, tex_path);

        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(tex_path);

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        let y = h * 0.5;

        commands.spawn((
            Name::new("Pickup_Test_Ammo"),
            Pickup {
                tile,
                kind: PickupKind::Ammo { rounds },
            },
            Mesh3d(quad),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(tile.x as f32, y, tile.y as f32))
                .with_rotation(pickup_base_rot()),
            GlobalTransform::default(),
        ));
    }

    // --------------------
    // Treasure (test)
    // --------------------
    // Treasure-only rendering tweak:
    // - Mask writes depth cleanly (no blended-depth weirdness at the floor line)
    // - depth_bias makes it win against the floor at the very bottom pixels
    const TREASURE_DROP_DEPTH_BIAS: f32 = -250.0;

    let desired_treasure: &[(TreasureKind, IVec2)] = &[
        (TreasureKind::Cross, IVec2::new(27, 18)),
        (TreasureKind::Chalice, IVec2::new(29, 18)),
        (TreasureKind::Chest, IVec2::new(27, 20)),
        (TreasureKind::Crown, IVec2::new(29, 20)),
    ];

    for &(t, mut tile) in desired_treasure {
        let in_bounds = tile.x >= 0
            && tile.y >= 0
            && (tile.x as usize) < grid.width
            && (tile.y as usize) < grid.height;

        let ok_tile = in_bounds
            && tile != player_tile
            && matches!(grid.tile(tile.x as usize, tile.y as usize), Tile::Empty)
            && !used_tiles.contains(&tile);

        if !ok_tile {
            let Some(fallback) = find_empty_tile_not_used(&grid, &used_tiles, player_tile) else {
                warn!("spawn_test_weapon_pickup: no empty tiles found for Treasure {:?}", t);
                continue;
            };
            warn!(
                "spawn_test_weapon_pickup: Treasure {:?} wanted {:?}, using fallback {:?}",
                t, tile, fallback
            );
            tile = fallback;
        }

        used_tiles.push(tile);

        let (w, h) = treasure_size(t);
        let tex_path = treasure_texture(t);

        info!(
            "Spawning TEST treasure {:?} at tile {:?} using {}",
            t, tile, tex_path
        );

        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(tex_path);

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),

            // IMPORTANT: treasure should not get “cut into” the floor.
            // Use the same approach as ammo drops: mask + depth bias.
            alpha_mode: AlphaMode::Mask(0.5),
            depth_bias: TREASURE_DROP_DEPTH_BIAS,

            unlit: true,
            cull_mode: None,
            ..default()
        });

        let y = h * 0.5;

        commands.spawn((
            Name::new(format!("Pickup_Test_Treasure_{:?}", t)),
            Pickup {
                tile,
                kind: PickupKind::Treasure(t),
            },
            Mesh3d(quad),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(tile.x as f32, y, tile.y as f32))
                .with_rotation(pickup_base_rot()),
            GlobalTransform::default(),
        ));
    }

    // --------------------
    // Health + 1UP (test)
    // --------------------
    const HEALTH_DEPTH_BIAS: f32 = -250.0;

    let desired_health: &[(HealthKind, IVec2)] = &[
        // Put these deeper into the test room so they aren’t near the door
        (HealthKind::FirstAid, IVec2::new(27, 22)),
        (HealthKind::Dinner,   IVec2::new(29, 22)),
        (HealthKind::DogFood,  IVec2::new(27, 24)),
    ];

    for &(hk, mut tile) in desired_health {
        let in_bounds = tile.x >= 0
            && tile.y >= 0
            && (tile.x as usize) < grid.width
            && (tile.y as usize) < grid.height;

        let ok_tile = in_bounds
            && tile != player_tile
            && matches!(grid.tile(tile.x as usize, tile.y as usize), Tile::Empty)
            && !used_tiles.contains(&tile);

        if !ok_tile {
            let Some(fallback) = find_empty_tile_not_used(&grid, &used_tiles, player_tile) else {
                warn!("spawn_test_weapon_pickup: no empty tiles found for Health");
                continue;
            };
            warn!(
                "spawn_test_weapon_pickup: Health wanted {:?}, using fallback {:?}",
                tile, fallback
            );
            tile = fallback;
        }

        used_tiles.push(tile);

        let (w, h) = health_pickup_size(hk);
        let tex_path = health_texture(hk);

        info!("Spawning TEST health {:?} at tile {:?} using {}", hk, tile, tex_path);

        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(tex_path);

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),
            alpha_mode: AlphaMode::Mask(0.5),
            depth_bias: HEALTH_DEPTH_BIAS,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        let y = h * 0.5;

        commands.spawn((
            Name::new(format!("Pickup_Test_Health_{:?}", hk)),
            Pickup { tile, kind: PickupKind::Health(hk) },
            Mesh3d(quad),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(tile.x as f32, y, tile.y as f32))
                .with_rotation(pickup_base_rot()),
            GlobalTransform::default(),
        ));
    }

    // Extra life (1UP)
    {
        let mut tile = IVec2::new(29, 24);

        let in_bounds = tile.x >= 0
            && tile.y >= 0
            && (tile.x as usize) < grid.width
            && (tile.y as usize) < grid.height;

        let ok_tile = in_bounds
            && tile != player_tile
            && matches!(grid.tile(tile.x as usize, tile.y as usize), Tile::Empty)
            && !used_tiles.contains(&tile);

        if !ok_tile {
            let Some(fallback) = find_empty_tile_not_used(&grid, &used_tiles, player_tile) else {
                warn!("spawn_test_weapon_pickup: no empty tiles found for 1UP");
                return;
            };
            warn!(
                "spawn_test_weapon_pickup: 1UP wanted {:?}, using fallback {:?}",
                tile, fallback
            );
            tile = fallback;
        }

        used_tiles.push(tile);

        let (w, h) = oneup_size();
        let tex_path = oneup_texture();

        info!("Spawning TEST 1UP at tile {:?} using {}", tile, tex_path);

        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(tex_path);

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),
            alpha_mode: AlphaMode::Mask(0.5),
            depth_bias: HEALTH_DEPTH_BIAS,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        let y = h * 0.5;

        commands.spawn((
            Name::new("Pickup_Test_OneUp"),
            Pickup { tile, kind: PickupKind::ExtraLife },
            Mesh3d(quad),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(tile.x as f32, y, tile.y as f32))
                .with_rotation(pickup_base_rot()),
            GlobalTransform::default(),
        ));
    }

}

pub fn billboard_pickups(
    q_player: Query<&Transform, (With<Player>, Without<Pickup>)>,
    mut q_pickups: Query<(&Pickup, &mut Transform), (With<Pickup>, Without<Player>)>,
) {
    let Some(player_tf) = q_player.iter().next() else { return; };

    let player_pos = player_tf.translation;
    let base_rot = pickup_base_rot();

    // Microscopic XZ Nudge for DROPPED AMMO only
    const DROP_IN_FRONT_EPS: f32 = 0.004;

    for (p, mut tf) in q_pickups.iter_mut() {
        let mut pos = tf.translation;

        if let PickupKind::Ammo { rounds } = p.kind {
            // For Dropped Loot
            if rounds == GUARD_DROP_AMMO_ROUNDS {
                // Anchor to Tile Center, Preserve Y From Spawn
                pos.x = p.tile.x as f32;
                pos.z = p.tile.y as f32;

                // Direction Toward Player in XZ
                let mut to_player = player_pos - pos;
                to_player.y = 0.0;

                let len2 = to_player.length_squared();
                if len2 > 0.0001 {
                    let dir = to_player / len2.sqrt();
                    pos.x += dir.x * DROP_IN_FRONT_EPS;
                    pos.z += dir.z * DROP_IN_FRONT_EPS;
                }
            }
        }

        tf.translation = pos;

        // Yaw-Only Face the Player
        let mut to_player = player_pos - tf.translation;
        to_player.y = 0.0;

        let len2 = to_player.length_squared();
        if len2 > 0.0001 {
            let dir = to_player / len2.sqrt();
            let yaw = dir.x.atan2(dir.z);
            tf.rotation = Quat::from_rotation_y(yaw) * base_rot;
        } else {
            tf.rotation = base_rot;
        }
    }
}

pub fn collect_pickups(
    mut commands: Commands,
    q_player: Query<&Transform, With<Player>>,
    mut hud: ResMut<HudState>,
    q_pickups: Query<(Entity, &Pickup)>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let mut it = q_player.iter();
    let Some(player_tf) = it.next() else {
        return;
    };

    let player_tile = world_to_tile_xz(Vec2::new(
        player_tf.translation.x,
        player_tf.translation.z,
    ));

    for (e, p) in q_pickups.iter() {
        if p.tile != player_tile {
            continue;
        }

        let mut consumed = true;

        match p.kind {
            PickupKind::Weapon(w) => {
                let kind = match w {
                    WeaponSlot::Chaingun => Some(SfxKind::PickupChaingun),
                    WeaponSlot::MachineGun => Some(SfxKind::PickupMachineGun),
                    _ => None,
                };

                if let Some(kind) = kind {
                    sfx.write(PlaySfx { kind, pos: player_tf.translation });
                }

                if !hud.owns(w) {
                    hud.grant(w);
                    hud.selected = w;
                }
            }

            PickupKind::Ammo { rounds } => {
                sfx.write(PlaySfx { kind: SfxKind::PickupAmmo, pos: player_tf.translation });
                hud.ammo += rounds;
            }

            PickupKind::Treasure(t) => {
                let kind = match t {
                    TreasureKind::Cross => SfxKind::PickupTreasureCross,
                    TreasureKind::Chalice => SfxKind::PickupTreasureChalice,
                    TreasureKind::Chest => SfxKind::PickupTreasureChest,
                    TreasureKind::Crown => SfxKind::PickupTreasureCrown,
                };

                sfx.write(PlaySfx { kind, pos: player_tf.translation });
                hud.score += t.points();
            }

            PickupKind::Health(hk) => {
                const HP_MAX: i32 = 100;

                if hud.hp >= HP_MAX {
                     // Health Full: Leave on Ground, No Sfx
                    consumed = false;
                } else {
                    let gain = hk.heal().min(HP_MAX - hud.hp);
                    hud.hp += gain;

                    let kind = match hk {
                        HealthKind::FirstAid => SfxKind::PickupHealthFirstAid,
                        HealthKind::Dinner => SfxKind::PickupHealthDinner,
                        HealthKind::DogFood => SfxKind::PickupHealthDogFood,
                    };

                    sfx.write(PlaySfx { kind, pos: player_tf.translation });
                }
            }

            PickupKind::ExtraLife => {
                // Wolfenstein 3D (1992):
                // +1 Life, Full Health, +25 Ammo
                hud.lives += 1;
                hud.hp = 100;
                hud.ammo += 25;

                sfx.write(PlaySfx { kind: SfxKind::PickupOneUp, pos: player_tf.translation });
            }
        }

        if consumed {
            commands.entity(e).despawn();
        }
    }
}
