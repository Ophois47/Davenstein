/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::time::Timer;

use crate::actors::{
    Dead,
    Health,
    OccupiesTile,
};
use crate::ai::EnemyMove;
use crate::audio::{
    PlaySfx,
    SfxKind,
    ActiveEnemyVoiceSfx,
};
use crate::player::Player;

// TODO: Health Often Varied by Skill Level
const GUARD_MAX_HP: i32 = 25;
const SS_MAX_HP: i32 = 100;
const OFFICER_MAX_HP: i32 = 50;
const MUTANT_MAX_HP: i32 = 45;
const DOG_MAX_HP: i32 = 1;
const HANS_MAX_HP: i32 = 850;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    Guard,
    Ss,
    Officer,
    Mutant,
    Dog,
    Hans,
    // TODO: Other Bosses
}

#[derive(Clone, Copy, Debug)]
pub enum AttackMode {
    Hitscan,
    Melee,
}

#[derive(Clone, Copy, Debug)]
pub struct EnemyTuning {
    pub max_hp: i32,
    pub wander_speed_tps: f32,
    pub chase_speed_tps: f32,
    pub can_shoot: bool,
    pub attack_damage: i32,
    pub attack_cooldown_secs: f32,
    pub attack_range_tiles: f32,
    pub reaction_time_secs: f32,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct EnemyTunings {
    pub guard: EnemyTuning,
    pub ss: EnemyTuning,
    pub officer: EnemyTuning,
    pub mutant: EnemyTuning,
    pub dog: EnemyTuning,
    pub hans: EnemyTuning,
}

impl EnemyTunings {
    /// Single Source of Truth for Defaults
    /// Without Relying on Default / Derive(Default)
    pub fn baseline() -> Self {
        Self {
            guard: EnemyTuning {
                max_hp: 25,
                wander_speed_tps: 0.9,
                chase_speed_tps: 1.6,
                can_shoot: true,
                attack_damage: 8,
                attack_cooldown_secs: 0.6,
                attack_range_tiles: 6.0,
                reaction_time_secs: 0.80,
            },
            ss: EnemyTuning {
                max_hp: 100,
                wander_speed_tps: 1.0,
                chase_speed_tps: 1.8,
                can_shoot: true,
                attack_damage: 10,
                attack_cooldown_secs: 0.55,
                attack_range_tiles: 7.0,
                reaction_time_secs: 0.50,
            },
            officer: EnemyTuning {
                max_hp: 50,
                wander_speed_tps: 0.9,
                chase_speed_tps: 1.8,
                can_shoot: true,
                attack_damage: 8,
                attack_cooldown_secs: 0.6,
                attack_range_tiles: 6.0,
                reaction_time_secs: 0.30,
            },
            mutant: EnemyTuning {
                max_hp: 45,
                wander_speed_tps: 0.9,
                chase_speed_tps: 1.9,
                can_shoot: true,
                attack_damage: 8,
                attack_cooldown_secs: 0.6,
                attack_range_tiles: 6.0,
                reaction_time_secs: 0.30,
            },
            dog: EnemyTuning {
                max_hp: 1,
                wander_speed_tps: 1.2,
                chase_speed_tps: 2.8,
                can_shoot: false,
                attack_damage: 8,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 1.1,
                reaction_time_secs: 0.20,
            },
            hans: EnemyTuning {
                max_hp: HANS_MAX_HP,
                // Wolfenstein 3-D Bosses Typically Stand Still Until Alerted
                wander_speed_tps: 0.0,
                chase_speed_tps: 1.3,
                can_shoot: true,
                attack_damage: 30,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 8.0,
                reaction_time_secs: 0.30,
            },
        }
    }

    pub fn for_kind(&self, kind: EnemyKind) -> EnemyTuning {
        match kind {
            EnemyKind::Guard => self.guard,
            EnemyKind::Ss => self.ss,
            EnemyKind::Officer => self.officer,
            EnemyKind::Mutant => self.mutant,
            EnemyKind::Dog => self.dog,
            EnemyKind::Hans => self.hans,
        }
    }
}

#[derive(Component)]
pub struct Guard;

#[derive(Component)]
pub struct Mutant;

#[derive(Component)]
pub struct GuardCorpse;

#[derive(Component)]
pub struct MutantCorpse;

#[derive(Component, Debug, Default)]
pub struct GuardWalk {
    // Progress in "Tiles Moved"
    // Frame = Floor(Phase*4) & 3
    pub phase: f32,
}

#[derive(Component, Debug, Default)]
pub struct MutantWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct GuardPain {
    pub timer: Timer,
}

#[derive(Component)]
pub struct MutantPain {
    pub timer: Timer,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GuardDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Debug, Clone, Copy)]
pub struct MutantDying {
    pub frame: u8,
    pub tics: u8,
}

#[derive(Component, Debug)]
pub struct GuardShoot {
    pub timer: Timer,
}

#[derive(Component, Debug)]
pub struct MutantShoot {
    pub timer: Timer,
}

#[derive(Resource)]
pub struct GuardSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],

    pub shoot_front_aim: Handle<Image>,
    pub shoot_front_fire: Handle<Image>,
    pub shoot_side_fire: Handle<Image>,

    pub pain: Handle<Image>,
    pub dying: [Handle<Image>; 4],
    pub corpse: Handle<Image>,
}

#[derive(Resource)]
pub struct MutantSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],

    pub shoot_front_aim: Handle<Image>,
    pub shoot_front_fire: Handle<Image>,
    pub shoot_side_fire: Handle<Image>,

    pub pain: Handle<Image>,
    pub dying: [Handle<Image>; 4],
    pub corpse: Handle<Image>,
}

impl FromWorld for GuardSprites {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        // 8-dir idle frames (your files: guard_idle_a0..a7.png)
        let idle: [Handle<Image>; 8] = std::array::from_fn(|dir| {
            asset_server.load(format!("enemies/guard/guard_idle_a{}.png", dir))
        });

        // 4 walk frames x 8 directions (your files: guard_walk_r{row}_dir{dir}.png)
        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            std::array::from_fn(|dir| {
                asset_server.load(format!(
                    "enemies/guard/guard_walk_r{}_dir{}.png",
                    row,
                    dir,
                ))
            })
        });

        // Single-frame states
        let pain: Handle<Image> = asset_server.load("enemies/guard/guard_pain.png");

        // Dying
        let dying: [Handle<Image>; 4] = std::array::from_fn(|i| {
            asset_server.load(format!("enemies/guard/guard_death_{}.png", i))
        });

        let corpse: Handle<Image> = asset_server.load("enemies/guard/guard_corpse.png");

        // Shooting
        let shoot_front_aim: Handle<Image> =
            asset_server.load("enemies/guard/guard_shoot_front_aim.png");
        let shoot_front_fire: Handle<Image> =
            asset_server.load("enemies/guard/guard_shoot_front_fire.png");
        let shoot_side_fire: Handle<Image> =
            asset_server.load("enemies/guard/guard_shoot_side_fire.png");

        Self {
            idle,
            walk,
            shoot_front_aim,
            shoot_front_fire,
            shoot_side_fire,
            pain,
            dying,
            corpse,
        }
    }
}

impl FromWorld for MutantSprites {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        // 8-dir idle frames (your files: guard_idle_a0..a7.png)
        let idle: [Handle<Image>; 8] = std::array::from_fn(|dir| {
            asset_server.load(format!("enemies/mutant/mutant_idle_a{}.png", dir))
        });

        // 4 walk frames x 8 directions (your files: guard_walk_r{row}_dir{dir}.png)
        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            std::array::from_fn(|dir| {
                asset_server.load(format!(
                    "enemies/mutant/mutant_walk_r{}_dir{}.png",
                    row,
                    dir,
                ))
            })
        });

        // Single-frame states
        let pain: Handle<Image> = asset_server.load("enemies/mutant/mutant_pain.png");

        // Dying
        let dying: [Handle<Image>; 4] = std::array::from_fn(|i| {
            asset_server.load(format!("enemies/mutant/mutant_death_{}.png", i))
        });

        let corpse: Handle<Image> = asset_server.load("enemies/mutant/mutant_corpse.png");

        // Shooting
        let shoot_front_aim: Handle<Image> =
            asset_server.load("enemies/mutant/mutant_shoot_front_aim.png");
        let shoot_front_fire: Handle<Image> =
            asset_server.load("enemies/mutant/mutant_shoot_front_fire.png");
        let shoot_side_fire: Handle<Image> =
            asset_server.load("enemies/mutant/mutant_shoot_side_fire.png");

        Self {
            idle,
            walk,
            shoot_front_aim,
            shoot_front_fire,
            shoot_side_fire,
            pain,
            dying,
            corpse,
        }
    }
}

pub(crate) const OFFICER_SHOOT_SECS: f32 = 0.35;

#[derive(Component)]
pub struct Officer;

#[derive(Component)]
pub struct OfficerCorpse;

#[derive(Component, Default)]
pub struct OfficerWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct OfficerShoot {
    pub t: Timer,
}

#[derive(Component)]
pub struct OfficerPain {
    pub timer: Timer,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct OfficerDying {
    pub frame: u8,
    pub tics: u8,
}

#[derive(Component)]
pub struct Ss;

#[derive(Component)]
pub struct Dog;

#[derive(Component)]
pub struct SsCorpse;

#[derive(Component)]
pub struct DogCorpse;

#[derive(Component)]
pub struct Hans;

#[derive(Component)]
pub struct HansCorpse;

#[derive(Component)]
pub struct SsPain {
    pub timer: Timer,
}

#[derive(Component)]
pub struct DogPain {
    pub timer: Timer,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SsDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Debug, Clone, Copy)]
pub struct HansDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DogDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Default)]
pub struct SsWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct SsShoot {
    pub t: Timer,
}

#[derive(Component, Default)]
pub struct HansWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct HansShoot {
    pub t: Timer,
}

#[derive(Component, Default)]
pub struct DogWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct DogBite {
    pub t: Timer,
}

impl DogBite {
    pub fn new() -> Self {
        Self {
            t: Timer::from_seconds(DOG_BITE_SECS, TimerMode::Once),
        }
    }
}

#[derive(Component, Debug)]
pub struct DogBiteCooldown {
    pub t: Timer,
}

impl DogBiteCooldown {
    pub fn new(secs: f32) -> Self {
        Self {
            t: Timer::from_seconds(secs.max(0.0), TimerMode::Once),
        }
    }
}

#[derive(Resource)]
pub struct OfficerSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub pain: [[Handle<Image>; 8]; 2],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

impl FromWorld for OfficerSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle = std::array::from_fn(|i| server.load(format!("enemies/officer/officer_idle_a{i}.png")));

        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| {
                server.load(format!("enemies/officer/officer_walk_r{row}_dir{dir}.png"))
            })
        });

        let shoot: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/officer/officer_shoot_{f}.png")));

        // Wolf note on your sheet: first two death frames are hurt frames
        // Reuse them as pain frames to avoid needing separate exports
        let pain0: Handle<Image> = server.load("enemies/officer/officer_death_0.png");
        let pain1: Handle<Image> = server.load("enemies/officer/officer_death_1.png");
        let pain = [
            std::array::from_fn(|_| pain0.clone()),
            std::array::from_fn(|_| pain1.clone()),
        ];

        let dying = std::array::from_fn(|i| {
            let h: Handle<Image> = server.load(format!("enemies/officer/officer_death_{i}.png"));
            std::array::from_fn(|_| h.clone())
        });

        let corpse_one: Handle<Image> = server.load("enemies/officer/officer_corpse.png");
        let corpse = std::array::from_fn(|_| corpse_one.clone());

        Self { idle, walk, shoot, pain, dying, corpse }
    }
}

pub(crate) const SS_SHOOT_SECS: f32 = 0.35;
pub(crate) const HANS_SHOOT_SECS: f32 = 0.35;
const DOG_BITE_SECS: f32 = 0.35;

fn attach_ss_walk(mut commands: Commands, q: Query<Entity, Added<Ss>>) {
    for e in q.iter() {
        commands.entity(e).insert(SsWalk::default());
    }
}

fn attach_hans_walk(mut commands: Commands, q: Query<Entity, Added<Hans>>) {
    for e in q.iter() {
        commands.entity(e).insert(HansWalk::default());
    }
}

fn attach_dog_walk(mut commands: Commands, q: Query<Entity, Added<Dog>>) {
    for e in q.iter() {
        commands.entity(e).insert(DogWalk::default());
    }
}

fn tick_ss_walk(
    time: Res<Time>,
    mut q: Query<(&mut SsWalk, Option<&EnemyMove>), (With<Ss>, Without<SsDying>)>,
) {
    let dt = time.delta_secs();
    for (mut w, moving) in q.iter_mut() {
        if let Some(m) = moving {
            // Drive Animation by Distance Traveled (Tiles)
            w.phase += m.speed_tps * dt;
        } else {
            w.phase = 0.0;
        }
    }
}

fn tick_hans_walk(
    time: Res<Time>,
    mut q: Query<(&mut HansWalk, Option<&EnemyMove>), (With<Hans>, Without<HansDying>)>,
) {
    let dt = time.delta_secs();
    for (mut w, moving) in q.iter_mut() {
        if let Some(m) = moving {
            // Drive Animation by Distance Traveled (Tiles)
            w.phase += m.speed_tps * dt;
        } else {
            w.phase = 0.0;
        }
    }
}

fn tick_dog_walk(
    time: Res<Time>,
    mut q: Query<(&mut DogWalk, Option<&EnemyMove>), (With<Dog>, Without<DogDying>)>,
) {
    let dt = time.delta_secs();
    for (mut w, moving) in q.iter_mut() {
        if let Some(m) = moving {
            // Drive animation by distance traveled (tiles).
            w.phase += m.speed_tps * dt;
        } else {
            w.phase = 0.0;
        }
    }
}

fn tick_ss_shoot(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut SsShoot)>) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<SsShoot>();
        }
    }
}

fn tick_hans_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut HansShoot), With<Hans>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<HansShoot>();
        }
    }
}

fn tick_dog_bite(
    time: Res<Time>,
    mut commands: Commands,
    tunings: Res<EnemyTunings>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut enemy_fire: MessageWriter<crate::ai::EnemyFire>,
    mut q: Query<(Entity, &GlobalTransform, &mut DogBite, Option<&DogPain>), With<Dog>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();
    let player_tile = IVec2::new(player_pos.x.round() as i32, player_pos.z.round() as i32);

    // Wolf3D (1992): dog bite hits ~70% of the time and does (US_RndT() >> 4) damage (0..15)
    // NOTE: This can yield 0 damage; if you decide you never want 0 from a *landed* bite
    // we can clamp it to at least 1
    const BITE_HIT_CHANCE: f32 = 0.70;

    for (e, gt, mut bite, pain) in q.iter_mut() {
        // Pain interrupts the bite immediately (no damage, no cooldown)
        if pain.is_some() {
            commands.entity(e).remove::<DogBite>();
            continue;
        }

        bite.t.tick(time.delta());
        if !bite.t.is_finished() {
            continue;
        }

        let dog_pos = gt.translation();
        let dog_tile = IVec2::new(dog_pos.x.round() as i32, dog_pos.z.round() as i32);

        let dx = (player_tile.x - dog_tile.x).abs();
        let dy = (player_tile.y - dog_tile.y).abs();
        let dist_tiles = dx.max(dy) as f32;

        if dist_tiles <= tunings.dog.attack_range_tiles && rand::random::<f32>() < BITE_HIT_CHANCE {
            let dmg = (rand::random::<u8>() >> 4) as i32;
            enemy_fire.write(crate::ai::EnemyFire {
                kind: EnemyKind::Dog,
                damage: dmg,
            });
        }

        let mut ec = commands.entity(e);
        ec.remove::<DogBite>();
        ec.insert(DogBiteCooldown::new(tunings.dog.attack_cooldown_secs));
    }
}

fn tick_dog_bite_cooldown(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut DogBiteCooldown), With<Dog>>,
) {
    for (e, mut cd) in q.iter_mut() {
        cd.t.tick(time.delta());
        if cd.t.is_finished() {
            commands.entity(e).remove::<DogBiteCooldown>();
        }
    }
}

fn tick_ss_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SsPain)>,
    mut started: Local<std::collections::HashMap<Entity, f32>>,
) {
    let now = time.elapsed_secs();

    // Track live entities so the Local map doesn't grow if something despawns mid-pain
    let mut live: Vec<Entity> = Vec::new();

    for (e, mut p) in q.iter_mut() {
        live.push(e);

        // Latch the *first* time this entity entered pain
        let start = started.entry(e).or_insert(now);

        // Keep ticking (harmless), but drive the elapsed from the latched start time
        // so repeated inserts won't reset the animation back to frame 0
        p.timer.tick(time.delta());

        let dur = p.timer.duration().as_secs_f32().max(1e-6);
        let elapsed = (now - *start).clamp(0.0, dur);

        p.timer
            .set_elapsed(std::time::Duration::from_secs_f32(elapsed));

        if (now - *start) >= dur {
            commands.entity(e).remove::<SsPain>();
            started.remove(&e);
        }
    }

    started.retain(|e, _| live.iter().any(|x| x == e));
}

fn tick_dog_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut DogPain, Option<&DogBite>), With<Dog>>,
) {
    for (e, mut p, bite) in q.iter_mut() {
        // Pain Interrupts In-Progress Bite
        if bite.is_some() {
            commands.entity(e).remove::<DogBite>();
        }

        p.timer.tick(time.delta());
        if p.timer.is_finished() {
            commands.entity(e).remove::<DogPain>();
        }
    }
}

fn tick_ss_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut SsDying)>,
) {
    for (e, mut d) in q.iter_mut() {
        // Wolf-style: advance animation by tics, not by a Timer
        d.tics = d.tics.saturating_add(1);

        // Every N tics, advance one frame.
        // (We'll tune N later; 8 is a sane starting point)
        if d.tics >= 8 {
            d.tics = 0;
            d.frame = d.frame.saturating_add(1);

            // 4 death frames: 0,1,2,3. After that, become a corpse
            if d.frame >= 4 {
                commands.entity(e).remove::<SsDying>();
                commands.entity(e).insert(SsCorpse);
                // Keep OccupiesTile on corpses so doors won't close on them
                // Player/enemy collision queries already ignore Dead
            }
        }
    }
}

fn tick_hans_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut HansDying)>,
) {
    for (e, mut d) in q.iter_mut() {
        // Advance animation by tics, not by a Timer
        d.tics = d.tics.saturating_add(1);

        if d.tics >= 8 {
            d.tics = 0;
            d.frame = d.frame.saturating_add(1);

            if d.frame >= 4 {
                commands.entity(e).remove::<HansDying>();
                commands.entity(e).insert(HansCorpse);
            }
        }
    }
}

fn tick_dog_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut DogDying)>,
) {
    const DEATH_FRAMES: u8 = 3;

    for (e, mut d) in q.iter_mut() {
        d.tics = d.tics.saturating_add(1);

        if d.tics >= 8 {
            d.tics = 0;
            d.frame = d.frame.saturating_add(1);

            if d.frame >= DEATH_FRAMES {
                commands.entity(e).remove::<DogDying>();
                commands.entity(e).insert(DogCorpse);
                // Keep OccupiesTile on corpses so doors won't close on them
                // Player/enemy collision queries already ignore Dead
            }
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct Dir8(pub u8);

// Cached to Avoid Redundant Texture Swaps
#[derive(Component, Clone, Copy)]
pub struct View8(pub u8);

#[derive(Resource)]
pub struct SsSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub pain: [[Handle<Image>; 8]; 2],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct DogSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub bite: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 3],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct HansSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

impl FromWorld for SsSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        const SS_PAIN_FILES: [&str; 2] = [
            "enemies/ss/ss_pain.png",
            "enemies/ss/ss_pain.png",
        ];

        const SS_DYING_FILES: [&str; 4] = [
            "enemies/ss/ss_death_0.png",
            "enemies/ss/ss_death_1.png",
            "enemies/ss/ss_death_2.png",
            "enemies/ss/ss_death_3.png",
        ];

        let idle = std::array::from_fn(|i| server.load(format!("enemies/ss/ss_idle_a{i}.png")));
        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| server.load(format!("enemies/ss/ss_walk_r{row}_dir{dir}.png")))
        });

        let shoot: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/ss/ss_shoot_{f}.png")));

        // Pain: load exactly what we want, duplicated across dirs
        let pain0: Handle<Image> = server.load(SS_PAIN_FILES[0]);
        let pain1: Handle<Image> = server.load(SS_PAIN_FILES[1]);
        let pain = [
            std::array::from_fn(|_| pain0.clone()),
            std::array::from_fn(|_| pain1.clone()),
        ];

        // Dying: load death frames in explicit order, duplicated across dirs
        let dying = std::array::from_fn(|i| {
            let h: Handle<Image> = server.load(SS_DYING_FILES[i]);
            std::array::from_fn(|_| h.clone())
        });

        let corpse_one: Handle<Image> = server.load("enemies/ss/ss_corpse.png");
        let corpse = std::array::from_fn(|_| corpse_one.clone());

        Self { idle, walk, shoot, pain, dying, corpse }
    }
}

impl FromWorld for HansSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle = std::array::from_fn(|i| server.load(format!("enemies/hans/hans_idle_a{i}.png")));
        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| server.load(format!("enemies/hans/hans_walk_r{row}_dir{dir}.png")))
        });

        let shoot: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/hans/hans_shoot_{f}.png")));

        let d0: Handle<Image> = server.load("enemies/hans/hans_death_0.png");
        let d1: Handle<Image> = server.load("enemies/hans/hans_death_1.png");
        let d2: Handle<Image> = server.load("enemies/hans/hans_death_2.png");
        let d3: Handle<Image> = server.load("enemies/hans/hans_death_3.png");

        let dying = [
            std::array::from_fn(|_| d0.clone()),
            std::array::from_fn(|_| d1.clone()),
            std::array::from_fn(|_| d2.clone()),
            std::array::from_fn(|_| d3.clone()),
        ];

        let corpse0: Handle<Image> = server.load("enemies/hans/hans_corpse.png");
        let corpse = std::array::from_fn(|_| corpse0.clone());

        Self {
            idle,
            walk,
            shoot,
            dying,
            corpse,
        }
    }
}

impl FromWorld for DogSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        // Walk frames (4) x directions (8)
        // Uses your exported naming: dog_walk_d{dir}_f{frame}.png
        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|frame| {
            std::array::from_fn(|dir| {
                server.load(format!("enemies/dog/dog_walk_d{dir}_f{frame}.png"))
            })
        });

        // Idle: use the "stand-looking" walk frame f2 for each direction
        let idle: [Handle<Image>; 8] = std::array::from_fn(|dir| walk[2][dir].clone());

        // Bite: 3 frames (not directional)
        let bite: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/dog/dog_bite_{f}.png")));

        // Dying: 3 frames (0..2), duplicated across dirs
        let dying: [[Handle<Image>; 8]; 3] = std::array::from_fn(|f| {
            let h: Handle<Image> = server.load(format!("enemies/dog/dog_death_{f}.png"));
            std::array::from_fn(|_| h.clone())
        });

        // Corpse: duplicated across dirs
        let corpse_one: Handle<Image> = server.load("enemies/dog/dog_corpse.png");
        let corpse = std::array::from_fn(|_| corpse_one.clone());

        Self { idle, walk, bite, dying, corpse }
    }
}

fn attach_guard_walk(mut commands: Commands, q: Query<Entity, (Added<Guard>, Without<GuardWalk>)>) {
    for e in q.iter() {
        commands.entity(e).insert(GuardWalk::default());
    }
}

fn attach_mutant_walk(mut commands: Commands, q: Query<Entity, (Added<Mutant>, Without<MutantWalk>)>) {
    for e in q.iter() {
        commands.entity(e).insert(MutantWalk::default());
    }
}

fn tick_guard_walk(
    time: Res<Time>,
    mut q: Query<(&mut GuardWalk, Option<&crate::ai::EnemyMove>), (With<Guard>, Without<Dead>, Without<GuardDying>)>,
) {
    let dt = time.delta_secs();
    for (mut walk, mv) in q.iter_mut() {
        if let Some(mv) = mv {
            // 1.0 phase per tile; 4 frames per tile
            walk.phase += dt * mv.speed_tps;
        } else {
            walk.phase = 0.0;
        }
    }
}

fn tick_mutant_walk(
    time: Res<Time>,
    mut q: Query<(&mut MutantWalk, Option<&crate::ai::EnemyMove>), (With<Mutant>, Without<Dead>, Without<MutantDying>)>,
) {
    let dt = time.delta_secs();
    for (mut walk, mv) in q.iter_mut() {
        if let Some(mv) = mv {
            // 1.0 phase per tile; 4 frames per tile
            walk.phase += dt * mv.speed_tps;
        } else {
            walk.phase = 0.0;
        }
    }
}

pub fn tick_guard_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardPain), With<Guard>>,
    mut started: Local<std::collections::HashMap<Entity, f32>>,
) {
    const PAIN_FLASH_SECS: f32 = 0.08;

    let now = time.elapsed_secs();

    // Track Entities Currently in Pain, Prune Stale Map Entries
    let mut live: Vec<Entity> = Vec::new();

    for (e, mut pain) in q.iter_mut() {
        live.push(e);

        // IMPORTANT: Do NOT Reset on Subsequent Hits
        // Stops Sustained Fire From Freezing Pain Sprite
        let start = started.entry(e).or_insert(now);

        // Tick Timer in Case Anything Relies on it
        // Clamp Visual Pain Duration Based on 'started'
        pain.timer.tick(time.delta());

        if now - *start >= PAIN_FLASH_SECS {
            commands.entity(e).remove::<GuardPain>();
            started.remove(&e);
        }
    }

    // Prevent Local<HashMap> from Growing if Entities Despawn While in Pain
    started.retain(|e, _| live.iter().any(|x| x == e));
}

pub fn tick_mutant_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut MutantPain), With<Mutant>>,
    mut started: Local<std::collections::HashMap<Entity, f32>>,
) {
    const PAIN_FLASH_SECS: f32 = 0.08;

    let now = time.elapsed_secs();

    // Track Entities Currently in Pain, Prune Stale Map Entries
    let mut live: Vec<Entity> = Vec::new();

    for (e, mut pain) in q.iter_mut() {
        live.push(e);

        // IMPORTANT: Do NOT Reset on Subsequent Hits
        // Stops Sustained Fire From Freezing Pain Sprite
        let start = started.entry(e).or_insert(now);

        // Tick Timer in Case Anything Relies on it
        // Clamp Visual Pain Duration Based on 'started'
        pain.timer.tick(time.delta());

        if now - *start >= PAIN_FLASH_SECS {
            commands.entity(e).remove::<MutantPain>();
            started.remove(&e);
        }
    }

    // Prevent Local<HashMap> from Growing if Entities Despawn While in Pain
    started.retain(|e, _| live.iter().any(|x| x == e));
}

fn tick_guard_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardShoot), With<Guard>>,
) {
    for (e, mut shoot) in q.iter_mut() {
        shoot.timer.tick(time.delta());
        if shoot.timer.is_finished() {
            commands.entity(e).remove::<GuardShoot>();
        }
    }
}

fn tick_mutant_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut MutantShoot), With<Mutant>>,
) {
    for (e, mut shoot) in q.iter_mut() {
        shoot.timer.tick(time.delta());
        if shoot.timer.is_finished() {
            commands.entity(e).remove::<MutantShoot>();
        }
    }
}

pub fn play_enemy_death_sfx(
    mut commands: Commands,
    mut sfx: MessageWriter<PlaySfx>,
    q_active_voice: Query<Entity, With<ActiveEnemyVoiceSfx>>,
    q: Query<(&GlobalTransform, &EnemyKind), Added<Dead>>,
) {
    // Immediately kill all enemy voice SFX when ANY enemy dies
    for e in q_active_voice.iter() {
        commands.entity(e).try_despawn();
    }

    for (gt, kind) in q.iter() {
        let p = gt.translation();
        let pos = Vec3::new(p.x, 0.6, p.z);

        sfx.write(PlaySfx {
            kind: SfxKind::EnemyDeath(*kind),
            pos,
        });
    }
}

pub fn spawn_guard(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &GuardSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    // A Vertical Quad in XY Plane (Normal +Z), UVs "Upright"
    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,       // No Lighting on Sprites
        cull_mode: None,   // Safe for Billboards
        ..default()
    });

    commands.spawn((
        Guard,
        EnemyKind::Guard,
        Dir8(0),
        View8(0),
        Health::new(GUARD_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_mutant(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &MutantSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    // A Vertical Quad in XY Plane (Normal +Z), UVs "Upright"
    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,       // No Lighting on Sprites
        cull_mode: None,   // Safe for Billboards
        ..default()
    });

    commands.spawn((
        Mutant,
        EnemyKind::Mutant,
        Dir8(0),
        View8(0),
        Health::new(MUTANT_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_ss(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &SsSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Ss,
        EnemyKind::Ss,
        Dir8(0),
        View8(0),
        Health::new(SS_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_officer(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &OfficerSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Officer,
        EnemyKind::Officer,
        Dir8(0),
        View8(0),
        Health::new(OFFICER_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

fn attach_officer_walk(mut commands: Commands, q: Query<Entity, Added<Officer>>) {
    for e in q.iter() {
        commands.entity(e).insert(OfficerWalk::default());
    }
}

fn tick_officer_walk(
    time: Res<Time>,
    mut q: Query<(&mut OfficerWalk, Option<&EnemyMove>), (With<Officer>, Without<OfficerDying>)>,
) {
    let dt = time.delta_secs();
    for (mut w, moving) in q.iter_mut() {
        if let Some(m) = moving {
            w.phase += m.speed_tps * dt;
        } else {
            w.phase = 0.0;
        }
    }
}

fn tick_officer_shoot(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut OfficerShoot)>) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<OfficerShoot>();
        }
    }
}

fn tick_officer_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut OfficerPain)>,
    mut started: Local<std::collections::HashMap<Entity, f32>>,
) {
    let now = time.elapsed_secs();

    let mut live: Vec<Entity> = Vec::new();

    for (e, mut p) in q.iter_mut() {
        live.push(e);

        let start = started.entry(e).or_insert(now);

        p.timer.tick(time.delta());

        let dur = p.timer.duration().as_secs_f32().max(1e-6);
        let elapsed = (now - *start).clamp(0.0, dur);

        p.timer.set_elapsed(std::time::Duration::from_secs_f32(elapsed));

        if (now - *start) >= dur {
            commands.entity(e).remove::<OfficerPain>();
            started.remove(&e);
        }
    }

    started.retain(|e, _| live.iter().any(|x| x == e));
}

fn tick_officer_dying(mut commands: Commands, mut q: Query<(Entity, &mut OfficerDying)>) {
    for (e, mut d) in q.iter_mut() {
        d.tics = d.tics.saturating_add(1);

        if d.tics >= 8 {
            d.tics = 0;
            d.frame = d.frame.saturating_add(1);

            if d.frame >= 4 {
                commands.entity(e).remove::<OfficerDying>();
                commands.entity(e).insert(OfficerCorpse);
            }
        }
    }
}

pub fn update_officer_views(
    sprites: Res<OfficerSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&OfficerCorpse>,
            Option<&OfficerDying>,
            Option<&OfficerPain>,
            Option<&OfficerShoot>,
            Option<&OfficerWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Officer>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (corpse, dying, pain, shoot, walk, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse[v as usize].clone()
        } else if let Some(d) = dying {
            let f = (d.frame as usize).min(3);
            sprites.dying[f][v as usize].clone()
        } else if let Some(p) = pain {
            let dur = p.timer.duration().as_secs_f32().max(1e-6);
            let t = p.timer.elapsed().as_secs_f32();
            let fi = ((t / dur) * 2.0).floor() as usize;
            sprites.pain[fi.min(1)][v as usize].clone()
        } else if let Some(s) = shoot {
            let dur = s.t.duration().as_secs_f32().max(1e-6);
            let t = s.t.elapsed().as_secs_f32();
            let fi = ((t / dur) * 3.0).floor() as usize;
            sprites.shoot[fi.min(2)].clone()
        } else if mv.is_some() {
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

pub fn spawn_hans(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &HansSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Hans,
        EnemyKind::Hans,
        Dir8(0),
        View8(0),
        Health::new(HANS_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_dog(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &DogSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Dog,
        EnemyKind::Dog,
        Dir8(0),
        View8(0),
        Health::new(DOG_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn update_ss_views(
    sprites: Res<SsSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&SsCorpse>,
            Option<&SsDying>,
            Option<&SsPain>,
            Option<&SsShoot>,
            Option<&SsWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Ss>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (corpse, dying, pain, shoot, walk, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse[v as usize].clone()
        } else if let Some(d) = dying {
            let f = (d.frame as usize).min(2);
            sprites.dying[f][v as usize].clone()
        } else if let Some(p) = pain {
            let dur = p.timer.duration().as_secs_f32().max(1e-6);
            let t = p.timer.elapsed().as_secs_f32();
            let fi = ((t / dur) * 2.0).floor() as usize;
            let fi = fi.min(1);
            sprites.pain[fi][v as usize].clone()
        } else if let Some(s) = shoot {
            let dur = s.t.duration().as_secs_f32().max(1e-6);
            let t = s.t.elapsed().as_secs_f32();
            let fi = ((t / dur) * 3.0).floor() as usize;
            let fi = fi.min(2);
            sprites.shoot[fi].clone()
        } else if mv.is_some() {
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

pub fn update_hans_views(
    sprites: Res<HansSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&HansCorpse>,
            Option<&HansDying>,
            Option<&HansShoot>,
            Option<&HansWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Hans>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (corpse, dying, shoot, walk, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse[v as usize].clone()
        } else if let Some(d) = dying {
            let f = (d.frame as usize).min(3);
            sprites.dying[f][v as usize].clone()
        } else if let Some(s) = shoot {
            let dur = s.t.duration().as_secs_f32().max(1e-6);
            let t = s.t.elapsed().as_secs_f32();
            let fi = ((t / dur) * 3.0).floor() as usize;
            let fi = fi.min(2);
            sprites.shoot[fi].clone()
        } else if mv.is_some() {
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

pub fn update_dog_views(
    sprites: Res<DogSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&DogCorpse>,
            Option<&DogDying>,
            Option<&DogPain>,
            Option<&DogBite>,
            Option<&DogWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Dog>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (corpse, dying, pain, bite, walk, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse[v as usize].clone()
        } else if let Some(d) = dying {
            let f = (d.frame as usize).min(3);
            sprites.dying[f][v as usize].clone()
        } else if pain.is_some() {
            // dog sheet has no dedicated pain frames in your zip; keep them “flinch-less” for now
            sprites.idle[v as usize].clone()
        } else if let Some(b) = bite {
            let dur = b.t.duration().as_secs_f32().max(1e-6);
            let t = b.t.elapsed().as_secs_f32();
            let frac = (t / dur).clamp(0.0, 0.999_9);

            let frame = (frac * 3.0).floor() as usize;
            sprites.bite[frame.min(2)].clone()
        } else if mv.is_some() {
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

fn quantize_view8(enemy_dir8: u8, enemy_pos: Vec3, player_pos: Vec3) -> u8 {
    use std::f32::consts::TAU;

    let to_player = player_pos - enemy_pos;
    let flat = Vec3::new(to_player.x, 0.0, to_player.z);
    if flat.length_squared() < 1e-6 {
        return 0;
    }

    let step = TAU / 8.0;
    let angle_to_player = flat.x.atan2(flat.z).rem_euclid(TAU);
    // Define Dir8(0) as Facing +Z, Dir8(2)=+X, Dir8(4)=-Z, Dir8(6)=-X
    let enemy_yaw = (enemy_dir8 as f32) * step;
    let rel = (angle_to_player - enemy_yaw).rem_euclid(TAU);

    (((rel + step * 0.5) / step).floor() as i32 & 7) as u8
}

pub fn tick_guard_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardDying), With<Guard>>,
) {
    const DEATH_FRAMES: u8 = 4;
    const TICS_PER_FRAME: u8 = 6;

    for (e, mut dying) in q.iter_mut() {
        dying.tics = dying.tics.saturating_add(1);

        if dying.tics >= TICS_PER_FRAME {
            dying.tics = 0;
            dying.frame = dying.frame.saturating_add(1);

            if dying.frame >= DEATH_FRAMES {
                // End of Animation -> Permanent Corpse (and non-blocking)
                commands.entity(e).remove::<GuardDying>();
                commands.entity(e).insert(GuardCorpse);
                // Keep OccupiesTile on corpses so doors won't close on them.
                // Player/enemy collision queries already ignore Dead.
            }
        }
    }
}

pub fn tick_mutant_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut MutantDying), With<Mutant>>,
) {
    const DEATH_FRAMES: u8 = 4;
    const TICS_PER_FRAME: u8 = 6;

    for (e, mut dying) in q.iter_mut() {
        dying.tics = dying.tics.saturating_add(1);

        if dying.tics >= TICS_PER_FRAME {
            dying.tics = 0;
            dying.frame = dying.frame.saturating_add(1);

            if dying.frame >= DEATH_FRAMES {
                // End of Animation -> Permanent Corpse (and non-blocking)
                commands.entity(e).remove::<MutantDying>();
                commands.entity(e).insert(MutantCorpse);
                // Keep OccupiesTile on corpses so doors won't close on them.
                // Player/enemy collision queries already ignore Dead.
            }
        }
    }
}

pub fn apply_guard_corpses(
    sprites: Res<GuardSprites>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(
        &MeshMaterial3d<StandardMaterial>,
        &mut Transform,
        Option<&mut Visibility>,
    ), (With<Guard>, Added<GuardCorpse>)>,
) {
    // Push Corpses Slightly "Back" so Item Drops
    // at Same Tile Can Win Depth Ties
    const CORPSE_DEPTH_BIAS: f32 = 250.0;

    for (mat3d, mut tf, vis) in q.iter_mut() {
        if let Some(mat) = materials.get_mut(&mat3d.0) {
            mat.base_color_texture = Some(sprites.corpse.clone());

            // Corpses Should NOT be Blend, or They'll Fight / Cover Drops
            mat.alpha_mode = AlphaMode::Mask(0.5);

            mat.unlit = true;
            mat.cull_mode = None;

            // Make Corpse Slightly Farther in Depth Than Drops
            mat.depth_bias = CORPSE_DEPTH_BIAS;
        }

        if let Some(mut v) = vis {
            *v = Visibility::Visible;
        }

        tf.translation.y = 0.5;
    }
}

pub fn update_guard_views(
    sprites: Res<GuardSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&Dead>,
            Option<&GuardCorpse>,
            Option<&GuardDying>,
            Option<&GuardPain>,
            Option<&GuardWalk>,
            Option<&GuardShoot>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Guard>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (_dead, corpse, dying, pain, walk, shoot, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        // Compute View Index (0..7) Relative to Enemy's Facing + Player Position
        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        // Rotate Quad to Face Player
        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        // Choose Texture in Priority Order:
        // Corpse > Dying > Pain > Shooting > Moving (Walk) > Idle
        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse.clone()
        } else if let Some(d) = dying {
            let i = (d.frame as usize).min(sprites.dying.len().saturating_sub(1));
            sprites.dying[i].clone()
        } else if pain.is_some() {
            sprites.pain.clone()
        } else if let Some(s) = shoot {
            let frontish = matches!(v, 0 | 1 | 7);

            // GuardShoot Has Only Timer', Pick Aim vs Fire Based on Timer Progress
            let dur = s.timer.duration().as_secs_f32().max(1e-6);
            let t = s.timer.elapsed().as_secs_f32();
            let fire_phase = t >= (dur * 0.5);

            if frontish {
                if fire_phase {
                    sprites.shoot_front_fire.clone()
                } else {
                    sprites.shoot_front_aim.clone()
                }
            } else {
                sprites.shoot_side_fire.clone()
            }
        } else if mv.is_some() {
            // Walk Frame Index From GuardWalk.phase (4 Frames Per Tile)
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

pub fn update_mutant_views(
    sprites: Res<MutantSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&Dead>,
            Option<&MutantCorpse>,
            Option<&MutantDying>,
            Option<&MutantPain>,
            Option<&MutantWalk>,
            Option<&MutantShoot>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Mutant>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (_dead, corpse, dying, pain, walk, shoot, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        // Compute View Index (0..7) Relative to Enemy's Facing + Player Position
        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        // Rotate Quad to Face Player
        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        // Choose Texture in Priority Order:
        // Corpse > Dying > Pain > Shooting > Moving (Walk) > Idle
        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse.clone()
        } else if let Some(d) = dying {
            let i = (d.frame as usize).min(sprites.dying.len().saturating_sub(1));
            sprites.dying[i].clone()
        } else if pain.is_some() {
            sprites.pain.clone()
        } else if let Some(s) = shoot {
            let frontish = matches!(v, 0 | 1 | 7);

            // GuardShoot Has Only Timer', Pick Aim vs Fire Based on Timer Progress
            let dur = s.timer.duration().as_secs_f32().max(1e-6);
            let t = s.timer.elapsed().as_secs_f32();
            let fire_phase = t >= (dur * 0.5);

            if frontish {
                if fire_phase {
                    sprites.shoot_front_fire.clone()
                } else {
                    sprites.shoot_front_aim.clone()
                }
            } else {
                sprites.shoot_side_fire.clone()
            }
        } else if mv.is_some() {
            // Walk Frame Index From MutantWalk.phase (4 Frames Per Tile)
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuardSprites>()
            .init_resource::<MutantSprites>()
            .init_resource::<SsSprites>()
            .init_resource::<OfficerSprites>()
            .init_resource::<DogSprites>()
            .init_resource::<HansSprites>()
            .add_systems(
                Update,
                (
                    (
                        attach_guard_walk,
                        attach_mutant_walk,
                        attach_ss_walk,
                        attach_officer_walk,
                        attach_dog_walk,
                        attach_hans_walk,
                    )
                        .chain(),
                    (
                        update_guard_views,
                        update_mutant_views,
                        update_ss_views,
                        update_officer_views,
                        update_dog_views,
                        update_hans_views,
                    )
                        .chain(),
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    (
                        tick_guard_walk,
                        tick_guard_pain,
                        tick_guard_shoot,
                        tick_guard_dying,
                    )
                        .chain(),
                    (
                        tick_mutant_walk,
                        tick_mutant_pain,
                        tick_mutant_shoot,
                        tick_mutant_dying,
                    )
                        .chain(),
                    (
                        tick_ss_walk,
                        tick_ss_pain,
                        tick_ss_shoot,
                        tick_ss_dying,
                    )
                        .chain(),
                    (
                        tick_officer_walk,
                        tick_officer_pain,
                        tick_officer_shoot,
                        tick_officer_dying,
                    )
                        .chain(),
                    (
                        tick_dog_walk,
                        tick_dog_pain,
                        tick_dog_bite_cooldown,
                        tick_dog_bite,
                        tick_dog_dying,
                    )
                        .chain(),
                    (
                        tick_hans_walk,
                        tick_hans_shoot,
                        tick_hans_dying,
                    )
                        .chain(),
                )
                    .chain(),
            );
    }
}
