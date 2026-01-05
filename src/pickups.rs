/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

use crate::combat::WeaponSlot;
use crate::ui::HudState;
use davelib::audio::{PlaySfx, SfxKind};
use davelib::enemies::{GuardCorpse, SsCorpse};
use davelib::level::WolfPlane1;
use davelib::map::{MapGrid, Tile};
use davelib::player::Player;

// Ammo Pickup Amounts
#[allow(dead_code)]
const MAP_AMMO_ROUNDS: i32 = 8;
const GUARD_DROP_AMMO_ROUNDS: i32 = 4;
// Wolfenstein 3d 1992 MAX_AMMO = 99
const AMMO_MAX: i32 = 99;

// Visual Size, Height in World Units
// Width Derived From Sprite Aspect
const PICKUP_H: f32 = 0.28;
const AMMO_H: f32 = 0.22;
const HEALTH_FIRST_AID_H: f32 = 0.18;
const HEALTH_DINNER_H: f32 = 0.18;
const HEALTH_DOGFOOD_H: f32 = AMMO_H;
const ONEUP_H: f32 = 0.50;
const TREASURE_H: f32 = 0.24;
const KEY_H: f32 = 0.42;

const HEALTH_FIRST_AID_W_SCALE: f32 = 3.6;
const HEALTH_DINNER_W_SCALE: f32 = 4.0;

// Aspect Ratios
const CHAINGUN_ASPECT: f32 = 60.0 / 21.0;
const MACHINEGUN_ASPECT: f32 = 47.0 / 18.0;
const AMMO_ASPECT: f32 = 16.0 / 12.0;
const CROSS_ASPECT: f32 = 20.0 / 19.0;
const CHALICE_ASPECT: f32 = 18.0 / 15.0;
const CHEST_ASPECT: f32 = 25.0 / 13.0;
const CROWN_ASPECT: f32 = 24.0 / 17.0;

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
    Key(KeyKind),
}

#[derive(Debug, Clone, Copy)]
pub enum KeyKind {
    Gold,
    Silver,
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
    const CROSS_SCALE: f32 = 1.25;

    let aspect = match t {
        TreasureKind::Cross => CROSS_ASPECT,
        TreasureKind::Chalice => CHALICE_ASPECT,
        TreasureKind::Chest => CHEST_ASPECT,
        TreasureKind::Crown => CROWN_ASPECT,
    };

    let s = match t {
        TreasureKind::Cross => CROSS_SCALE,
        _ => 1.0,
    };

    (TREASURE_H * aspect * s, TREASURE_H * s)
}

fn key_size() -> (f32, f32) {
    (KEY_H, KEY_H)
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

fn key_texture(k: KeyKind) -> &'static str {
    match k {
        KeyKind::Gold => "textures/pickups/key_gold.png",
        KeyKind::Silver => "textures/pickups/key_silver.png",
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

/// - 29: Dog Food
/// - 43: Key 1 (Gold)
/// - 44: Key 2 (Silver)
/// - 47: Food (Dinner)
/// - 48: Medkit (First Aid)
/// - 49: Ammo (+8)
/// - 50: Machine Gun
/// - 51: Chaingun
/// - 52-55: Treasure (Cross, Chalice, Chest, Crown)
/// - 56: 1UP
pub fn spawn_pickups(
    mut commands: Commands,
    grid: Res<MapGrid>,
    plane1_res: Res<WolfPlane1>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if grid.width != 64 || grid.height != 64 {
        warn!(
            "spawn_pickups: expected 64x64 grid, got {}x{}",
            grid.width, grid.height
        );
        return;
    }

    let expected = grid.width * grid.height;
    if plane1_res.0.len() != expected {
        warn!(
            "spawn_pickups: WolfPlane1 len {} != expected {} ({}x{})",
            plane1_res.0.len(),
            expected,
            grid.width,
            grid.height
        );
        return;
    }

    let plane1: &[u16] = &plane1_res.0;

    // Depth Tuning: Match Existing Drop / Treasure Approach
    const DEPTH_BIAS: f32 = -250.0;

    let to_pickup_kind = |v: u16| -> Option<PickupKind> {
        match v {
            29 => Some(PickupKind::Health(HealthKind::DogFood)),

            43 => Some(PickupKind::Key(KeyKind::Gold)),
            44 => Some(PickupKind::Key(KeyKind::Silver)),

            47 => Some(PickupKind::Health(HealthKind::Dinner)),
            48 => Some(PickupKind::Health(HealthKind::FirstAid)),
            49 => Some(PickupKind::Ammo {
                rounds: MAP_AMMO_ROUNDS,
            }),
            50 => Some(PickupKind::Weapon(WeaponSlot::MachineGun)),
            51 => Some(PickupKind::Weapon(WeaponSlot::Chaingun)),
            52 => Some(PickupKind::Treasure(TreasureKind::Cross)),
            53 => Some(PickupKind::Treasure(TreasureKind::Chalice)),
            54 => Some(PickupKind::Treasure(TreasureKind::Chest)),
            55 => Some(PickupKind::Treasure(TreasureKind::Crown)),
            56 => Some(PickupKind::ExtraLife),
            _ => None,
        }
    };

    let spawn_pickup = |
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        asset_server: &AssetServer,
        tile: IVec2,
        kind: PickupKind,
    | {
        let (w, h, tex_path, alpha_mode) = match kind {
            PickupKind::Weapon(slot) => {
                let (w, h) = weapon_pickup_size(slot);
                (w, h, weapon_pickup_texture(slot), AlphaMode::Mask(0.5))
            }
            PickupKind::Ammo { .. } => {
                let (w, h) = ammo_size();
                (w, h, ammo_texture(), AlphaMode::Mask(0.5))
            }
            PickupKind::Treasure(t) => {
                let (w, h) = treasure_size(t);
                (w, h, treasure_texture(t), AlphaMode::Mask(0.5))
            }
            PickupKind::Health(hk) => {
                let (w, h) = health_pickup_size(hk);
                (w, h, health_texture(hk), AlphaMode::Mask(0.5))
            }
            PickupKind::ExtraLife => {
                let (w, h) = oneup_size();
                (w, h, oneup_texture(), AlphaMode::Mask(0.5))
            }
            PickupKind::Key(k) => {
                let (w, h) = key_size();
                (w, h, key_texture(k), AlphaMode::Mask(0.5))
            }
        };

        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(tex_path);

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),
            alpha_mode,
            depth_bias: DEPTH_BIAS,
            unlit: true,
            cull_mode: None,
            ..default()
        });

        let y = h * 0.5;

        commands.spawn((
            Name::new("Pickup"),
            Pickup { tile, kind },
            Mesh3d(quad),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(tile.x as f32, y, tile.y as f32))
                .with_rotation(pickup_base_rot()),
            GlobalTransform::default(),
        ));
    };

    let idx = |x: usize, z: usize| -> usize { z * grid.width + x };

    for z in 0..grid.height {
        for x in 0..grid.width {
            let v = plane1[idx(x, z)];
            let Some(kind) = to_pickup_kind(v) else {
                continue;
            };

            // Only Place Pickups on Walkable Tiles
            if !matches!(grid.tile(x, z), Tile::Empty) {
                continue;
            }

            let tile = IVec2::new(x as i32, z as i32);
            spawn_pickup(
                &mut commands,
                &mut meshes,
                &mut materials,
                &asset_server,
                tile,
                kind,
            );
        }
    }
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

pub fn drop_ss_loot(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    hud: Res<HudState>,
    q_corpses: Query<(Entity, &GlobalTransform), (With<SsCorpse>, Without<DroppedLoot>)>,
) {
    // Depth Tweak: with AlphaMode::Mask this will actually affect depth testing
    const DROP_DEPTH_BIAS: f32 = -250.0;

    // Tiny Lift to Avoid Z-Fighting With Floor
    const DROP_Y_LIFT: f32 = 0.01;

    let has_machinegun = hud.owns(WeaponSlot::MachineGun) || hud.owns(WeaponSlot::Chaingun);
    let kind = if has_machinegun {
        PickupKind::Ammo { rounds: MAP_AMMO_ROUNDS }
    } else {
        PickupKind::Weapon(WeaponSlot::MachineGun)
    };

    let (w, h, tex_path) = match kind {
        PickupKind::Ammo { .. } => {
            let (w, h) = ammo_size();
            (w, h, ammo_texture())
        }
        PickupKind::Weapon(slot) => {
            let (w, h) = weapon_pickup_size(slot);
            (w, h, weapon_pickup_texture(slot))
        }
        // Should Never Happen
        _ => return,
    };

    for (e, gt) in q_corpses.iter() {
        // Drop Once per Corpse
        commands.entity(e).insert(DroppedLoot);

        // Drop at the Corpse Tile
        let p = gt.translation();
        let tile = world_to_tile_xz(Vec2::new(p.x, p.z));

        let quad = meshes.add(Plane3d::default().mesh().size(w, h));
        let tex: Handle<Image> = asset_server.load(tex_path);

        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            cull_mode: None,
            depth_bias: DROP_DEPTH_BIAS,
            ..default()
        });

        let y = (h * 0.5) + DROP_Y_LIFT;

        commands.spawn((
            Name::new("Pickup_Drop_SS"),
            Pickup { tile, kind },
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

        // Yaw Only Face the Player
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
    q_player: Query<(Entity, &Transform), With<Player>>,
    mut q_vitals: Query<&mut davelib::player::PlayerVitals, With<davelib::player::Player>>,
    mut q_pkeys: Query<&mut davelib::player::PlayerKeys, With<davelib::player::Player>>,
    mut hud: ResMut<HudState>,
    mut face_ov: ResMut<crate::ui::HudFaceOverride>,
    mut pickup_flash: ResMut<crate::ui::PickupFlash>,
    q_pickups: Query<(Entity, &Pickup)>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some((player_e, player_tf)) = q_player.iter().next() else {
        return;
    };

    let player_tile = world_to_tile_xz(Vec2::new(
        player_tf.translation.x,
        player_tf.translation.z,
    ));

    let Some(mut vitals) = q_vitals.iter_mut().next() else {
        return;
    };

    for (e, p) in q_pickups.iter() {
        if p.tile != player_tile {
            continue;
        }

        let mut consumed = true;

        match p.kind {
            PickupKind::Weapon(w) => {
                if hud.owns(w) {
                    consumed = false;
                } else {
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

                    hud.grant(w);
                    hud.selected = w;

                    if w == WeaponSlot::Chaingun {
                        face_ov.active = true;
                        face_ov.timer.reset();
                    }
                }
            }

            PickupKind::Ammo { rounds } => {
                if hud.ammo >= AMMO_MAX {
                    consumed = false;
                } else {
                    let gain = rounds.min(AMMO_MAX - hud.ammo);
                    hud.ammo += gain;

                    sfx.write(PlaySfx {
                        kind: SfxKind::PickupAmmo,
                        pos: player_tf.translation,
                    });
                }
            }

            PickupKind::Treasure(t) => {
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
            }

            PickupKind::Health(hk) => {
                if vitals.hp >= vitals.hp_max {
                    consumed = false;
                } else {
                    let gain = hk.heal().min(vitals.hp_max - vitals.hp);
                    vitals.hp += gain;

                    let kind = match hk {
                        HealthKind::FirstAid => SfxKind::PickupHealthFirstAid,
                        HealthKind::Dinner => SfxKind::PickupHealthDinner,
                        HealthKind::DogFood => SfxKind::PickupHealthDogFood,
                    };

                    sfx.write(PlaySfx {
                        kind,
                        pos: player_tf.translation,
                    });
                }
            }

            PickupKind::ExtraLife => {
                hud.lives += 1;
                vitals.hp = vitals.hp_max;
                hud.ammo = (hud.ammo + 25).min(AMMO_MAX);

                sfx.write(PlaySfx {
                    kind: SfxKind::PickupOneUp,
                    pos: player_tf.translation,
                });
            }

            PickupKind::Key(k) => {
                // Use PlayerKeys as Gameplay Truth, Keep HUD Mirrored
                match q_pkeys.get_mut(player_e) {
                    Ok(mut pk) => {
                        let already = match k {
                            KeyKind::Gold => pk.gold,
                            KeyKind::Silver => pk.silver,
                        };

                        if already {
                            consumed = false;
                        } else {
                            match k {
                                KeyKind::Gold => {
                                    pk.gold = true;
                                    hud.key_gold = true;
                                }
                                KeyKind::Silver => {
                                    pk.silver = true;
                                    hud.key_silver = true;
                                }
                            }

                            sfx.write(PlaySfx {
                                kind: SfxKind::PickupKey,
                                pos: player_tf.translation,
                            });
                        }
                    }
                    Err(_) => {
                        // If Player Somehow Spawned Without PlayerKeys, Attach Immediately
                        let already = match k {
                            KeyKind::Gold => hud.key_gold,
                            KeyKind::Silver => hud.key_silver,
                        };

                        if already {
                            consumed = false;
                        } else {
                            match k {
                                KeyKind::Gold => hud.key_gold = true,
                                KeyKind::Silver => hud.key_silver = true,
                            }

                            commands.entity(player_e).insert(match k {
                                KeyKind::Gold => davelib::player::PlayerKeys { 
                                    gold: true, 
                                    silver: false,
                                },
                                KeyKind::Silver => davelib::player::PlayerKeys {
                                    gold: false,
                                    silver: true,
                                },
                            });

                            sfx.write(PlaySfx {
                                kind: SfxKind::PickupKey,
                                pos: player_tf.translation,
                            });
                        }
                    }
                }
            }
        }

        if consumed {
            pickup_flash.trigger(Srgba::new(1.0, 62.0 / 64.0, 0.0, 1.0));
            commands.entity(e).despawn();
        }
    }
}
