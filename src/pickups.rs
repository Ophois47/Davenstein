use bevy::prelude::*;
use crate::combat::WeaponSlot;
use crate::ui::HudState;
use std::f32::consts::FRAC_PI_2;
use davelib::audio::{PlaySfx, SfxKind};
use davelib::enemies::GuardCorpse;
use davelib::map::{MapGrid, Tile};
use davelib::player::Player;

// Visual size (height in world units). Width is derived from sprite aspect
const PICKUP_H: f32 = 0.28;
const AMMO_H: f32 = 0.22;
const TREASURE_H: f32 = 0.24;
const TREASURE_Y_LIFT: f32 = 0.08;
const TREASURE_DEPTH_BIAS: f32 = -150.0;

// These aspect ratios match the actual extracted sprites we're using:
// chaingun.png: 60x21  => ~2.857
// machinegun.png: 47x18 => ~2.611
const CHAINGUN_ASPECT: f32 = 60.0 / 21.0;
const MACHINEGUN_ASPECT: f32 = 47.0 / 18.0;
const AMMO_ASPECT: f32 = 16.0 / 12.0;
const CROSS_ASPECT: f32   = 19.0 / 18.0;
const CHALICE_ASPECT: f32 = 18.0 / 15.0;
const CHEST_ASPECT: f32   = 25.0 / 13.0;
const CROWN_ASPECT: f32   = 24.0 / 17.0;

#[derive(Component, Debug, Clone, Copy)]
pub struct Pickup {
    pub tile: IVec2, // (x, z) tile coords
    pub kind: PickupKind,
}

#[derive(Debug, Clone, Copy)]
pub enum PickupKind {
    Weapon(WeaponSlot),
    Ammo { rounds: i32 }, // +8 in map, +4 from enemy drop
    Treasure(TreasureKind),
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

fn treasure_size(t: TreasureKind) -> (f32, f32) {
    let aspect = match t {
        TreasureKind::Cross => CROSS_ASPECT,
        TreasureKind::Chalice => CHALICE_ASPECT,
        TreasureKind::Chest => CHEST_ASPECT,
        TreasureKind::Crown => CROWN_ASPECT,
    };
    (TREASURE_H * aspect, TREASURE_H)
}

fn treasure_y(h: f32) -> f32 {
    (h * 0.5) + TREASURE_Y_LIFT
}

fn treasure_texture(t: TreasureKind) -> &'static str {
    match t {
        TreasureKind::Cross => "textures/pickups/treasure_cross.png",
        TreasureKind::Chalice => "textures/pickups/treasure_chalice.png",
        TreasureKind::Chest => "textures/pickups/treasure_chest.png",
        TreasureKind::Crown => "textures/pickups/treasure_crown.png",
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DroppedLoot;

fn weapon_pickup_size(w: WeaponSlot) -> (f32, f32) {
    match w {
        WeaponSlot::Chaingun => (PICKUP_H * CHAINGUN_ASPECT, PICKUP_H),
        WeaponSlot::MachineGun => (PICKUP_H * MACHINEGUN_ASPECT, PICKUP_H),
        _ => (PICKUP_H, PICKUP_H),
    }
}

fn weapon_pickup_texture(w: WeaponSlot) -> &'static str {
    match w {
        WeaponSlot::Chaingun => "textures/pickups/chaingun.png",
        WeaponSlot::MachineGun => "textures/pickups/machinegun.png",
        _ => "textures/pickups/chaingun.png", // placeholder (won't be used yet)
    }
}

fn ammo_size() -> (f32, f32) {
    (AMMO_H * AMMO_ASPECT, AMMO_H)
}

fn ammo_texture() -> &'static str {
    "textures/pickups/ammo.png"
}

fn world_to_tile_xz(pos_xz: Vec2) -> IVec2 {
    // Matches the logic used in player_move()
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
    // Depth tweak: with AlphaMode::Mask this will actually affect depth testing.
    // If you ever see it “punch through” walls at extreme angles, reduce magnitude (e.g. -50.0).
    const DROP_DEPTH_BIAS: f32 = -250.0;

    // Tiny lift just to avoid z-fighting with the floor (not a “float toward camera”).
    const DROP_Y_LIFT: f32 = 0.01;

    for (e, gt) in q_corpses.iter() {
        // Drop once per corpse.
        commands.entity(e).insert(DroppedLoot);

        // Drop at the corpse tile.
        let p = gt.translation();
        let tile = world_to_tile_xz(Vec2::new(p.x, p.z));

        let rounds = 4;

        let (w, h) = ammo_size();
        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(ammo_texture());

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),

            // IMPORTANT: Mask writes depth, so the corpse can’t overwrite it later.
            // Pick a cutoff that keeps edges crisp; adjust to 0.25 if you see “holes”.
            alpha_mode: AlphaMode::Mask(0.5),

            unlit: true,
            cull_mode: None,

            // IMPORTANT: make this slightly “closer” in depth than the corpse at the same tile.
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

/// Startup: spawn one test weapon pickup (Chaingun) on the first empty tile
/// (not on the player's current tile). Placeholder mesh for now.
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

    // Track used tiles so fallback placement doesn't stack items.
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
            depth_bias: TREASURE_DEPTH_BIAS,

            unlit: true,
            cull_mode: None,
            ..default()
        });

        let y = treasure_y(h);

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
}

pub fn billboard_pickups(
    q_player: Query<&Transform, (With<Player>, Without<Pickup>)>,
    mut q_pickups: Query<(&Pickup, &mut Transform), (With<Pickup>, Without<Player>)>,
) {
    let Some(player_tf) = q_player.iter().next() else { return; };

    let player_pos = player_tf.translation;
    let base_rot = pickup_base_rot();

    // Treasure-only lift to keep bottom pixels off the floor seam.
    // This will finally “do something” because it’s applied every frame.
    const TREASURE_Y_EPS: f32 = 0.06;

    // Microscopic XZ nudge for DROPPED AMMO only (rounds == 4 in your guard drops).
    // Tiny enough to be invisible, big enough to break corpse/drop coplanar fighting.
    const DROP_IN_FRONT_EPS: f32 = 0.004;

    for (p, mut tf) in q_pickups.iter_mut() {
        // Tile center is your stable anchor. We only override position for
        // (1) treasure Y, and (2) dropped ammo XZ micro offset.
        let mut pos = tf.translation;

        // --- Treasure: force Y up (treasure only; nothing else changes) ---
        if let PickupKind::Treasure(t) = p.kind {
            let (_w, h) = treasure_size(t);
            pos.y = treasure_y(h) + TREASURE_Y_EPS;

            // Keep treasure anchored at the tile center in XZ so it doesn't drift.
            pos.x = p.tile.x as f32;
            pos.z = p.tile.y as f32;
        }

        // --- Dropped ammo: micro XZ nudge toward the player (rounds == 4) ---
        if let PickupKind::Ammo { rounds } = p.kind {
            if rounds == 4 {
                // Anchor to tile center, preserve current Y
                pos.x = p.tile.x as f32;
                pos.z = p.tile.y as f32;

                // Direction toward player in XZ
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

        // --- Rotation: yaw-only billboard to face player ---
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

/// FixedUpdate: collect pickups when player steps onto their tile.
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

        match p.kind {
            PickupKind::Weapon(w) => {
                // Play per-weapon pickup SFX (plays even if already owned).
                let kind = match w {
                    WeaponSlot::Chaingun => Some(SfxKind::PickupChaingun),
                    WeaponSlot::MachineGun => Some(SfxKind::PickupMachineGun),
                    _ => None,
                };

                if let Some(kind) = kind {
                    sfx.write(PlaySfx {
                        kind,
                        pos: player_tf.translation,
                    });
                }

                if !hud.owns(w) {
                    hud.grant(w);
                    hud.selected = w; // auto-switch only when newly acquired
                    info!("Picked up weapon: {:?} (now owned, auto-selected)", w);
                } else {
                    info!("Picked up weapon: {:?} (already owned)", w);
                }
            }
            PickupKind::Ammo { rounds } => {
                // ammo pickup sfx
                sfx.write(PlaySfx {
                    kind: SfxKind::PickupAmmo,
                    pos: player_tf.translation,
                });

                hud.ammo += rounds;
                info!("Picked up ammo: +{} (ammo now {})", rounds, hud.ammo);
            }
            PickupKind::Treasure(t) => {
                // Per-treasure pickup SFX
                let kind = match t {
                    TreasureKind::Cross => SfxKind::PickupTreasureCross,
                    TreasureKind::Chalice => SfxKind::PickupTreasureChalice,
                    TreasureKind::Chest => SfxKind::PickupTreasureChest,
                    TreasureKind::Crown => SfxKind::PickupTreasureCrown,
                };

                sfx.write(PlaySfx {
                    kind,
                    pos: player_tf.translation,
                });

                hud.score += t.points();
                info!("Picked up treasure: {:?} (+{} score)", t, t.points());
            }
        }

        commands.entity(e).despawn();
    }
}
