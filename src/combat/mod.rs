/*
Davenstein - by David Petnick
*/
pub mod hitscan;
pub mod projectiles;

use bevy::prelude::*;
use bevy::time::{Timer, TimerMode};

use hitscan::raycast_grid;
use davelib::actors::{
    Dead,
    Health,
    OccupiesTile,
};
use davelib::audio::{PlaySfx, SfxKind};
use davelib::decorations::SolidStatics;
use davelib::enemies::{
    EnemyKind,
    GuardDying,
    GuardPain,
    MutantDying,
    MutantPain,
    SsDying,
    SsPain,
    OfficerDying,
    OfficerPain,
    DogDying,
    DogPain,
    HansDying,
    GretelDying,
    HitlerDying,
    MechaHitlerDying,
    GhostHitlerDying,
    SchabbsDying,
    OttoDying,
    GeneralDying,
};
use davelib::map::MapGrid;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FireShot>()
            .add_message::<projectiles::SpawnProjectile>()
            .add_systems(Startup, projectiles::setup_projectile_assets)
            .add_systems(Update, process_fire_shots.run_if(crate::world_ready))
            .add_systems(FixedUpdate, projectiles::tick_smoke_puffs.run_if(crate::world_ready))
            .add_systems(FixedUpdate, projectiles::tick_rocket_impacts.run_if(crate::world_ready))
            .add_systems(FixedUpdate, projectiles::tick_projectiles.run_if(crate::world_ready))
            .add_systems(FixedUpdate, process_enemy_fireball_shots.run_if(crate::world_ready))
            .add_systems(FixedUpdate, process_enemy_syringe_shots.run_if(crate::world_ready))
            .add_systems(FixedUpdate, process_enemy_rocket_shots.run_if(crate::world_ready))
            .add_systems(FixedUpdate, projectiles::spawn_projectiles.run_if(crate::world_ready))
            .add_systems(
                PostUpdate,
                projectiles::update_projectile_views.run_if(crate::world_ready),
            )
            .add_systems(
                PostUpdate,
                projectiles::update_smoke_puff_views.run_if(crate::world_ready),
            )
            .add_systems(
                PostUpdate,
                projectiles::update_rocket_impact_views.run_if(crate::world_ready),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponSlot {
    Knife = 0,
    Pistol = 1,
    MachineGun = 2,
    Chaingun = 3,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct FireShot {
    pub weapon: WeaponSlot,
    pub origin: Vec3,
    pub dir: Vec3,
    pub max_dist: f32,
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

fn process_enemy_fireball_shots(
    mut fireballs: MessageReader<davelib::ai::EnemyFireballShot>,
    mut spawn: MessageWriter<projectiles::SpawnProjectile>,
) {
    for fb in fireballs.read() {
        spawn.write(projectiles::SpawnProjectile {
            kind: projectiles::ProjectileKind::Fireball,
            origin: fb.origin,
            dir: fb.dir,
        });
    }
}

fn process_enemy_syringe_shots(
    mut syringes: MessageReader<davelib::ai::EnemySyringeShot>,
    mut spawn: MessageWriter<projectiles::SpawnProjectile>,
) {
    for syr in syringes.read() {
        spawn.write(projectiles::SpawnProjectile {
            kind: projectiles::ProjectileKind::Syringe,
            origin: syr.origin,
            dir: syr.dir,
        });
    }
}

fn process_enemy_rocket_shots(
    mut syringes: MessageReader<davelib::ai::EnemyRocketShot>,
    mut spawn: MessageWriter<projectiles::SpawnProjectile>,
) {
    for syr in syringes.read() {
        spawn.write(projectiles::SpawnProjectile {
            kind: projectiles::ProjectileKind::Rocket,
            origin: syr.origin,
            dir: syr.dir,
        });
    }
}

fn process_fire_shots(
    grid: Option<Res<MapGrid>>,
    solid: Option<Res<SolidStatics>>,
    mut shots: MessageReader<FireShot>,
    mut sfx: MessageWriter<PlaySfx>,
    mut commands: Commands,
    q_alive: Query<
        (Entity, &EnemyKind, &OccupiesTile, &GlobalTransform),
        (With<EnemyKind>, Without<Dead>),
    >,
    mut q_hp: Query<&mut Health, (With<EnemyKind>, Without<Dead>)>,
    mut q_ai: Query<&mut davelib::ai::EnemyAi, (With<EnemyKind>, Without<Dead>)>,
    mut level_score: ResMut<davelib::level_score::LevelScore>,
    mut rng: Local<u32>,
) {
    let (Some(grid), Some(solid)) = (grid, solid) else {
        return;
    };

    fn hitbox(kind: EnemyKind) -> (f32, f32, f32) {
        // (radius, half_h, center_y)
        match kind {
            EnemyKind::Dog => (0.38, 0.45, 0.40),
            // Boss is Visually Big, Slightly Larger Hitbox
            EnemyKind::Hans => (0.52, 0.70, 0.55),
            // Guard / SS / Officer / Mutant
            _ => (0.48, 0.65, 0.50),
        }
    }

    fn xorshift32(s: &mut u32) -> u32 {
        let mut x = *s;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *s = x;
        x
    }

    fn rnd_byte(rng: &mut u32) -> i32 {
        (xorshift32(rng) & 0xFF) as i32
    }

    fn ray_exit_dist_xz(origin: Vec3, dir: Vec3, w: usize, h: usize) -> Option<f32> {
        let x_min = -0.5_f32;
        let x_max = w as f32 - 0.5;
        let z_min = -0.5_f32;
        let z_max = h as f32 - 0.5;

        let ox = origin.x;
        let oz = origin.z;
        let dx = dir.x;
        let dz = dir.z;

        let mut t_enter = f32::NEG_INFINITY;
        let mut t_exit = f32::INFINITY;

        const EPS: f32 = 1e-8;

        if dx.abs() < EPS {
            if ox < x_min || ox > x_max {
                return None;
            }
        } else {
            let inv = 1.0 / dx;
            let mut t0 = (x_min - ox) * inv;
            let mut t1 = (x_max - ox) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_enter = t_enter.max(t0);
            t_exit = t_exit.min(t1);
        }

        if dz.abs() < EPS {
            if oz < z_min || oz > z_max {
                return None;
            }
        } else {
            let inv = 1.0 / dz;
            let mut t0 = (z_min - oz) * inv;
            let mut t1 = (z_max - oz) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_enter = t_enter.max(t0);
            t_exit = t_exit.min(t1);
        }

        if t_enter > t_exit {
            return None;
        }

        (t_exit >= 0.0).then_some(t_exit)
    }

    fn roll_gun_damage(dist_tiles: i32, rng: &mut u32) -> Option<i32> {
        let effective_dist = ((dist_tiles.max(0) * 1) / 2).clamp(0, 20);
        // Treat 0 Damage as Miss so Hits Never Deal 0
        let damage = if effective_dist < 2 {
            rnd_byte(rng) / 4
        } else if effective_dist < 4 {
            rnd_byte(rng) / 6
        } else {
            if (rnd_byte(rng) / 12) < effective_dist {
                return None;
            }
            rnd_byte(rng) / 6
        };

        (damage > 0).then_some(damage)
    }

    fn roll_knife_damage(rng: &mut u32) -> i32 {
        (rnd_byte(rng) / 4).max(1)
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

        let boundary_dist = ray_exit_dist_xz(
            shot.origin,
            dir,
            grid.width,
            grid.height,
        ).unwrap_or(shot.max_dist);
        let max_dist = shot.max_dist.min(boundary_dist);

        let world_hit = raycast_grid(&grid, &solid, shot.origin, dir, max_dist);
        let world_dist = world_hit.as_ref().map(|h| h.dist).unwrap_or(max_dist);

        // Shooter Tile (Chebyshev Tile Distance like Wolf GunAttack)
        let ptx = (shot.origin.x + 0.5).floor() as i32;
        let ptz = (shot.origin.z + 0.5).floor() as i32;

        // Find Nearest Living Enemy Hit Before Wall
        let mut best: Option<(Entity, EnemyKind, f32, i32)> = None;

        for (e, kind, occ, gt) in q_alive.iter() {
            let p = gt.translation();
            let (radius, half_h, center_y) = hitbox(*kind);
            let center = Vec3::new(p.x, center_y, p.z);

            let Some(t) = ray_hit_vertical_cylinder(shot.origin, dir, center, radius, half_h) else {
                continue;
            };

            if t <= max_dist && t < world_dist {
                let etx = occ.0.x;
                let etz = occ.0.y;
                let dist_tiles = (ptx - etx).abs().max((ptz - etz).abs());

                match best {
                    None => best = Some((e, *kind, t, dist_tiles)),
                    Some((_, _, best_t, _)) if t < best_t => best = Some((e, *kind, t, dist_tiles)),
                    _ => {}
                }
            }
        }

        // Enemy Hit Consumes Shot
        if let Some((e, kind, _t, dist_tiles)) = best {
            let mut dmg_opt = match shot.weapon {
                WeaponSlot::Knife => Some(roll_knife_damage(&mut *rng)),
                WeaponSlot::Pistol | WeaponSlot::MachineGun | WeaponSlot::Chaingun => {
                    roll_gun_damage(dist_tiles, &mut *rng)
                }
            };

            // Surprise Double Damage if Not in Attack Mode Yet
            if let Ok(mut ai) = q_ai.get_mut(e) {
                if ai.state == davelib::ai::EnemyAiState::Stand {
                    if let Some(d) = dmg_opt.as_mut() {
                        *d *= 2;
                    }
                    ai.state = davelib::ai::EnemyAiState::Chase;
                }
            }

            let Some(dmg) = dmg_opt else {
                continue;
            };

            if let Ok(mut hp) = q_hp.get_mut(e) {
                hp.cur -= dmg;

                if hp.cur <= 0 {
                    hp.cur = 0;

                    // Kill Discovery Counts When Death is Latched
                    level_score.kills_found += 1;

                    if let Ok((_, _, _, gt)) = q_alive.get(e) {
                        let p = gt.translation();
                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyDeath(kind),
                            pos: Vec3::new(p.x, 0.6, p.z),
                        });
                    }

                    commands.entity(e).insert(Dead);

                    match kind {
                        EnemyKind::Guard => {
                            commands.entity(e).insert(GuardDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Mutant => {
                            commands.entity(e).insert(MutantDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Ss => {
                            commands.entity(e).insert(SsDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Officer => {
                            commands.entity(e).insert(OfficerDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Dog => {
                            commands.entity(e).insert(DogDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Hans => {
                            commands.entity(e).insert(HansDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Gretel => {
                            commands.entity(e).insert(GretelDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Hitler => {
                            commands.entity(e).insert(HitlerDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::MechaHitler => {
                            commands.entity(e).insert(MechaHitlerDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::GhostHitler => {
                            commands.entity(e).insert(GhostHitlerDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Schabbs => {
                            commands.entity(e).insert(SchabbsDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::Otto => {
                            commands.entity(e).insert(OttoDying { frame: 0, tics: 0 });
                        }
                        EnemyKind::General => {
                            commands.entity(e).insert(GeneralDying { frame: 0, tics: 0 });
                        }
                    }
                } else {
                    let timer = Timer::from_seconds(0.20, TimerMode::Once);
                    match kind {
                        EnemyKind::Guard => {
                            commands.entity(e).insert(GuardPain { timer });
                        }
                        EnemyKind::Mutant => {
                            commands.entity(e).insert(MutantPain { timer });
                        }
                        EnemyKind::Ss => {
                            commands.entity(e).insert(SsPain { timer });
                        }
                        EnemyKind::Officer => {
                            commands.entity(e).insert(OfficerPain { timer });
                        }
                        EnemyKind::Dog => {
                            commands.entity(e).insert(DogPain { timer });
                        }
                        EnemyKind::Hans => {
                            // Wolfenstein 3-D Bosses Do Not Flinch / Enter Pain
                        }
                        EnemyKind::Gretel => {
                            // Wolfenstein 3-D Bosses Do Not Flinch / Enter Pain
                        }
                        EnemyKind::Hitler => {
                            // Wolfenstein 3-D Bosses Do Not Flinch / Enter Pain
                        }
                        EnemyKind::MechaHitler => {
                            // Wolfenstein 3-D Bosses Do Not Flinch / Enter Pain
                        }
                        EnemyKind::GhostHitler => {
                            // Wolfenstein 3-D Fake Hitler Does Not Flinch / Enter Pain
                        }
                        EnemyKind::Schabbs => {
                            // Wolfenstein 3-D Bosses Do Not Flinch / Enter Pain
                        }
                        EnemyKind::Otto => {
                            // Wolfenstein 3-D Bosses Do Not Flinch / Enter Pain
                        }
                        EnemyKind::General => {
                            // Wolfenstein 3-D Bosses Do Not Flinch / Enter Pain
                        }
                    }
                }
            }

            continue;
        }
    }
}
