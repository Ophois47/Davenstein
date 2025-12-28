/*
Davenstein - by David Petnick
*/
mod hitscan;

use bevy::prelude::*;
use bevy::time::{Timer, TimerMode};

use hitscan::raycast_grid;
use davelib::actors::{
    Dead,
    Health,
    OccupiesTile,
};
use davelib::audio::PlaySfx;
use davelib::decorations::SolidStatics;
use davelib::enemies::{
    Guard,
    GuardDying,
    GuardPain,
};
use davelib::map::MapGrid;

#[derive(Message, Debug, Clone, Copy)]
pub struct FireShot {
    #[allow(dead_code)]
    pub weapon: WeaponSlot,
    pub origin: Vec3,
    pub dir: Vec3,
    pub max_dist: f32,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FireShot>()
            .add_systems(Update, process_fire_shots);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponSlot {
    Knife = 0,
    Pistol = 1,
    MachineGun = 2,
    Chaingun = 3,
}

impl WeaponSlot {
    pub fn from_digit_key(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Digit1 => Some(Self::Knife),
            KeyCode::Digit2 => Some(Self::Pistol),
            KeyCode::Digit3 => Some(Self::MachineGun),
            KeyCode::Digit4 => Some(Self::Chaingun),
            _ => None,
        }
    }
}

fn process_fire_shots(
    grid: Res<MapGrid>,
    solid: Res<SolidStatics>,
    mut shots: MessageReader<FireShot>,
    mut _sfx: MessageWriter<PlaySfx>,
    mut commands: Commands,
    q_alive: Query<(Entity, &OccupiesTile, &GlobalTransform), (With<Guard>, Without<Dead>)>,
    mut q_hp: Query<&mut Health, (With<Guard>, Without<Dead>)>,
    mut rng: Local<u32>,
) {
    const ENEMY_RADIUS: f32 = 0.35;   // Tile Units (slightly forgiving, Wolf-ish auto-aim feel)
    const ENEMY_HALF_H: f32 = 0.55;   // Slightly forgiving vertical hitbox
    const ENEMY_CENTER_Y: f32 = 0.50; // Center at Y=0.5

    fn xorshift32(s: &mut u32) -> u32 {
        let mut x = *s;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *s = x;
        x
    }

    fn roll_pistol_damage(tile_dist: i32, rng: &mut u32) -> i32 {
        // Close: 0..63, Mid: 0..31, Far: 0..15
        let bucket: u32 = if tile_dist <= 1 { 63 } else if tile_dist <= 3 { 31 } else { 15 };
        (xorshift32(rng) % (bucket + 1)) as i32
    }

    fn ray_hit_vertical_cylinder(
        origin: Vec3,
        dir: Vec3,
        center: Vec3,
        radius: f32,
        half_h: f32,
    ) -> Option<f32> {
        // 2D Ray-Circle in XZ, Then Clamp by Y at T
        let o = Vec2::new(origin.x, origin.z);
        let d = Vec2::new(dir.x, dir.z);
        let c = Vec2::new(center.x, center.z);

        let a = d.dot(d);
        if a < 0.0000001 {
            return None;
        }

        let oc = o - c;
        let b = 2.0 * oc.dot(d);
        let cc = oc.dot(oc) - radius * radius;
        let disc = b * b - 4.0 * a * cc;
        if disc < 0.0 {
            return None;
        }

        let sqrt_disc = disc.sqrt();
        let t0 = (-b - sqrt_disc) / (2.0 * a);
        let t1 = (-b + sqrt_disc) / (2.0 * a);

        let t = if t0 >= 0.0 {
            t0
        } else if t1 >= 0.0 {
            t1
        } else {
            return None;
        };

        let y_at = origin.y + dir.y * t;
        let y_min = center.y - half_h;
        let y_max = center.y + half_h;

        if y_at >= y_min && y_at <= y_max {
            Some(t)
        } else {
            None
        }
    }

    // Seed RNG Once
    if *rng == 0 {
        *rng = 0xC0FFEE_u32;
    }

    for shot in shots.read() {
        let dir = shot.dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            continue;
        }

        let world_hit = raycast_grid(&grid, &solid, shot.origin, dir, shot.max_dist);
        let world_dist = world_hit.as_ref().map(|h| h.dist).unwrap_or(shot.max_dist);

        // Find Nearest Living Guard Hit Before the Wall / Floor Hit
        let mut best: Option<(Entity, f32, i32)> = None;

        let ptx = (shot.origin.x + 0.5).floor() as i32;
        let ptz = (shot.origin.z + 0.5).floor() as i32;

        for (e, _occ, gt) in q_alive.iter() {
            let p = gt.translation();
            let center = Vec3::new(p.x, ENEMY_CENTER_Y, p.z);

            let Some(t) = ray_hit_vertical_cylinder(
                shot.origin,
                dir,
                center,
                ENEMY_RADIUS,
                ENEMY_HALF_H,
            ) else {
                continue;
            };

            if t <= shot.max_dist && t < world_dist {
                let etx = (center.x + 0.5).floor() as i32;
                let etz = (center.z + 0.5).floor() as i32;
                let dist_tiles = (ptx - etx).abs().max((ptz - etz).abs());

                match best {
                    None => best = Some((e, t, dist_tiles)),
                    Some((_, best_t, _)) if t < best_t => best = Some((e, t, dist_tiles)),
                    _ => {}
                }
            }
        }

        // Enemy Hit Consumes Shot
        if let Some((e, _t, dist_tiles)) = best {
            let dmg = roll_pistol_damage(dist_tiles, &mut *rng);

            if let Ok(mut hp) = q_hp.get_mut(e) {
                hp.cur -= dmg;
                if hp.cur <= 0 {
                    hp.cur = 0;

                    commands.entity(e).insert(Dead);
                    commands.entity(e).insert(GuardDying { frame: 0, tics: 0 });
                } else {
                    // 80ms Wince Animation
                    commands.entity(e).insert(GuardPain {
                        timer: Timer::from_seconds(0.20, TimerMode::Once),
                    });
                }
            }

            continue;
        }
    }
}
