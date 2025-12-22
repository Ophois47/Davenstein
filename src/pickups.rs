use bevy::prelude::*;
use crate::combat::WeaponSlot;
use crate::ui::HudState;
use std::f32::consts::FRAC_PI_2;
use davelib::audio::{PlaySfx, SfxKind};
use davelib::map::{MapGrid, Tile};
use davelib::player::Player;

#[derive(Component, Debug, Clone, Copy)]
pub struct Pickup {
    pub tile: IVec2, // (x, z) tile coords
    pub kind: PickupKind,
}

#[derive(Debug, Clone, Copy)]
pub enum PickupKind {
    Weapon(WeaponSlot),
    AmmoClip { rounds: i32 }, // +8 in map, +4 from enemy drop
}

// Visual size (height in world units). Width is derived from sprite aspect
const PICKUP_H: f32 = 0.28;

// These aspect ratios match the actual extracted sprites we're using:
// chaingun.png: 60x21  => ~2.857
// machinegun.png: 47x18 => ~2.611
const CHAINGUN_ASPECT: f32 = 60.0 / 21.0;
const MACHINEGUN_ASPECT: f32 = 47.0 / 18.0;
const AMMOCLIP_ASPECT: f32 = 16.0 / 12.0;
const AMMOCLIP_H: f32 = 0.22;

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

fn ammo_clip_size() -> (f32, f32) {
    (AMMOCLIP_H * AMMOCLIP_ASPECT, AMMOCLIP_H)
}

fn ammo_clip_texture() -> &'static str {
    "textures/pickups/ammo_clip.png"
}

fn world_to_tile_xz(pos_xz: Vec2) -> IVec2 {
    // Matches the logic used in player_move()
    IVec2::new((pos_xz.x + 0.5).floor() as i32, (pos_xz.y + 0.5).floor() as i32)
}

fn pickup_base_rot() -> Quat {
    Quat::from_rotation_x(FRAC_PI_2)
}

fn find_first_empty_tile(grid: &MapGrid, avoid: IVec2) -> Option<IVec2> {
    for z in 0..grid.height {
        for x in 0..grid.width {
            if !matches!(grid.tile(x, z), Tile::Empty) {
                continue;
            }
            let t = IVec2::new(x as i32, z as i32);
            if t != avoid {
                return Some(t);
            }
        }
    }
    None
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

    let desired_weapons: &[(WeaponSlot, IVec2)] = &[
        (WeaponSlot::Chaingun, IVec2::new(27, 14)),
        (WeaponSlot::MachineGun, IVec2::new(29, 14)),
    ];

    let desired_ammo: &[(IVec2, i32)] = &[
        // found ammo = +8
        (IVec2::new(27, 16), 8),
        (IVec2::new(27, 17), 8),
        (IVec2::new(27, 18), 8),
        (IVec2::new(26, 16), 8),
        (IVec2::new(26, 17), 8),
        (IVec2::new(26, 18), 8),
    ];

    for &(weapon, mut tile) in desired_weapons {
        // Validate the desired tile is in-bounds, empty, and not the player tile.
        let in_bounds = tile.x >= 0
            && tile.y >= 0
            && (tile.x as usize) < grid.width
            && (tile.y as usize) < grid.height;

        let ok_tile = in_bounds
            && tile != player_tile
            && matches!(grid.tile(tile.x as usize, tile.y as usize), Tile::Empty);

        if !ok_tile {
            // Fallback: just find *some* empty tile (avoiding player).
            let Some(fallback) = find_first_empty_tile(&grid, player_tile) else {
                warn!("spawn_test_weapon_pickup: no empty tiles found for {:?}", weapon);
                continue;
            };
            tile = fallback;
        }

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

    for &(mut tile, rounds) in desired_ammo {
        // same validation/fallback pattern you already use
        let in_bounds = tile.x >= 0
            && tile.y >= 0
            && (tile.x as usize) < grid.width
            && (tile.y as usize) < grid.height;

        let ok_tile = in_bounds
            && tile != player_tile
            && matches!(grid.tile(tile.x as usize, tile.y as usize), Tile::Empty);

        if !ok_tile {
            let Some(fallback) = find_first_empty_tile(&grid, player_tile) else {
                warn!("spawn_test_weapon_pickup: no empty tiles found for AmmoClip");
                continue;
            };
            tile = fallback;
        }

        let (w, h) = ammo_clip_size();
        let tex_path = ammo_clip_texture();

        info!("Spawning TEST ammo clip at tile {:?} (+{})", tile, rounds);

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
            Name::new("Pickup_Test_AmmoClip"),
            Pickup {
                tile,
                kind: PickupKind::AmmoClip { rounds },
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
    mut q_pickups: Query<(&GlobalTransform, &mut Transform), (With<Pickup>, Without<Player>)>,
) {
    let mut it = q_player.iter();
    let Some(player_tf) = it.next() else { return; };

    let player_pos = player_tf.translation;
    let base = pickup_base_rot();

    for (gt, mut tf) in q_pickups.iter_mut() {
        let pos = gt.translation();

        let mut dir = player_pos - pos;
        dir.y = 0.0;

        let len2 = dir.length_squared();
        if len2 < 0.0001 {
            continue;
        }
        dir /= len2.sqrt();

        let yaw = dir.x.atan2(dir.z);
        tf.rotation = Quat::from_rotation_y(yaw) * base;
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
            PickupKind::AmmoClip { rounds } => {
                // ammo clip pickup sfx
                sfx.write(PlaySfx {
                    kind: SfxKind::PickupAmmo,
                    pos: player_tf.translation,
                });

                hud.ammo += rounds;
                info!("Picked up ammo clip: +{} (ammo now {})", rounds, hud.ammo);
            }
        }

        commands.entity(e).despawn();
    }
}
