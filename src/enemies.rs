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
use crate::episode_end::DeathCamBoss;
use crate::player::Player;

const GUARD_MAX_HP: i32 = 25;
const MUTANT_MAX_HP: i32 = 45;
const SS_MAX_HP: i32 = 100;
const OFFICER_MAX_HP: i32 = 50;
const DOG_MAX_HP: i32 = 1;

pub(crate) const SS_SHOOT_SECS: f32 = 0.35;
pub(crate) const OFFICER_SHOOT_SECS: f32 = 0.35;
pub(crate) const DOG_BITE_SECS: f32 = 0.35;

/// Boss HP by skill level (1992 MS-DOS Wolfenstein 3-D / WL6)
/// Source: Wolf3D wiki authentic values
/// skill.0: 0=Easy, 1=Medium, 2=Hard, 3=Nightmare
pub(crate) fn boss_health(kind: EnemyKind, skill: &crate::skill::SkillLevel) -> i32 {
    let difficulty = skill.0.min(3); // Clamp to valid range
    match (kind, difficulty) {
        // Hans Grosse
        (EnemyKind::Hans, 0) => 850,
        (EnemyKind::Hans, 1) => 950,
        (EnemyKind::Hans, 2) => 1050,
        (EnemyKind::Hans, 3) => 1200,
        
        // Dr. Schabbs
        (EnemyKind::Schabbs, 0) => 850,
        (EnemyKind::Schabbs, 1) => 950,
        (EnemyKind::Schabbs, 2) => 1550,
        (EnemyKind::Schabbs, 3) => 2400,
        
        // Fake Hitler (phase 1)
        (EnemyKind::Hitler, 0) => 200,
        (EnemyKind::Hitler, 1) => 300,
        (EnemyKind::Hitler, 2) => 400,
        (EnemyKind::Hitler, 3) => 500,
        
        // Mecha Hitler (phase 2)
        (EnemyKind::MechaHitler, 0) => 800,
        (EnemyKind::MechaHitler, 1) => 950,
        (EnemyKind::MechaHitler, 2) => 1050,
        (EnemyKind::MechaHitler, 3) => 1200,
        
        // Gretel Grosse
        (EnemyKind::Gretel, 0) => 850,
        (EnemyKind::Gretel, 1) => 950,
        (EnemyKind::Gretel, 2) => 1050,
        (EnemyKind::Gretel, 3) => 1200,
        
        // Otto Giftmacher
        (EnemyKind::Otto, 0) => 850,
        (EnemyKind::Otto, 1) => 950,
        (EnemyKind::Otto, 2) => 1050,
        (EnemyKind::Otto, 3) => 1200,
        
        // General Fettgesicht
        (EnemyKind::General, 0) => 850,
        (EnemyKind::General, 1) => 950,
        (EnemyKind::General, 2) => 1050,
        (EnemyKind::General, 3) => 1200,
        
        // Ghost Hitler
        (EnemyKind::GhostHitler, 0) => 850,
        (EnemyKind::GhostHitler, 1) => 950,
        (EnemyKind::GhostHitler, 2) => 1050,
        (EnemyKind::GhostHitler, 3) => 1200,
        
        // Fallback
        _ => 850,
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DeathCamReplaySlow(pub u8);

#[derive(Component, Debug, Clone, Copy)]
pub struct Dir8(pub u8);

// Cached to Avoid Redundant Texture Swaps
#[derive(Component, Clone, Copy)]
pub struct View8(pub u8);

#[derive(Clone, Copy, Debug)]
pub enum AttackMode {
    Hitscan,
    Melee,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    Guard,
    Ss,
    Officer,
    Mutant,
    Dog,
    Hans,
    Gretel,
    Hitler,
    MechaHitler,
    GhostHitler,
    Schabbs,
    Otto,
    General,
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
    pub gretel: EnemyTuning,
    pub hitler: EnemyTuning,
    pub mecha_hitler: EnemyTuning,
    pub ghost_hitler: EnemyTuning,
    pub schabbs: EnemyTuning,
    pub otto: EnemyTuning,
    pub general: EnemyTuning,
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
                max_hp: boss_health(EnemyKind::Hans, &crate::skill::SkillLevel(1)),
                // Wolfenstein 3-D Bosses Typically Stand Still Until Alerted
                wander_speed_tps: 0.0,
                chase_speed_tps: 1.3,
                can_shoot: true,
                attack_damage: 30,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 8.0,
                reaction_time_secs: 0.30,
            },
            gretel: EnemyTuning {
                max_hp: boss_health(EnemyKind::Gretel, &crate::skill::SkillLevel(1)),
                // Wolfenstein 3-D Bosses Typically Stand Still Until Alerted
                wander_speed_tps: 0.0,
                chase_speed_tps: 1.3,
                can_shoot: true,
                attack_damage: 30,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 8.0,
                reaction_time_secs: 0.30,
            },
            hitler: EnemyTuning {
                max_hp: boss_health(EnemyKind::Hitler, &crate::skill::SkillLevel(1)),
                wander_speed_tps: 0.0,
                chase_speed_tps: 1.3,
                can_shoot: true,
                attack_damage: 30,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 8.0,
                reaction_time_secs: 0.30,
            },
            mecha_hitler: EnemyTuning {
                max_hp: boss_health(EnemyKind::MechaHitler, &crate::skill::SkillLevel(1)),
                wander_speed_tps: 0.0,
                chase_speed_tps: 1.3,
                can_shoot: true,
                attack_damage: 30,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 8.0,
                reaction_time_secs: 0.30,
            },
            ghost_hitler: EnemyTuning {
                max_hp: boss_health(EnemyKind::GhostHitler, &crate::skill::SkillLevel(1)),
                wander_speed_tps: 0.0,
                chase_speed_tps: 0.6,
                can_shoot: true,
                attack_damage: 60,
                attack_cooldown_secs: 0.85,
                attack_range_tiles: 12.0,
                reaction_time_secs: 0.30,
            },
            schabbs: EnemyTuning {
                max_hp: boss_health(EnemyKind::Schabbs, &crate::skill::SkillLevel(1)),
                wander_speed_tps: 0.0,
                chase_speed_tps: 1.3,
                can_shoot: true,
                attack_damage: 30,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 8.0,
                reaction_time_secs: 0.30,
            },
            otto: EnemyTuning {
                max_hp: boss_health(EnemyKind::Otto, &crate::skill::SkillLevel(1)),
                wander_speed_tps: 0.0,
                chase_speed_tps: 1.3,
                can_shoot: true,
                attack_damage: 30,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 8.0,
                reaction_time_secs: 0.30,
            },
            general: EnemyTuning {
                max_hp: boss_health(EnemyKind::General, &crate::skill::SkillLevel(1)),
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
            EnemyKind::Gretel => self.gretel,
            EnemyKind::Hitler => self.hitler,
            EnemyKind::MechaHitler => self.mecha_hitler,
            EnemyKind::GhostHitler => self.ghost_hitler,
            EnemyKind::Schabbs => self.schabbs,
            EnemyKind::Otto => self.otto,
            EnemyKind::General => self.general,
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

    pub pain: Handle<Image>,
    pub dying: [Handle<Image>; 4],
    pub corpse: Handle<Image>,
}

#[derive(Resource)]
pub struct MutantSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],

    pub shoot_front_aim: Handle<Image>,
    pub shoot_front_fire_0: Handle<Image>,
    pub shoot_front_fire_1: Handle<Image>,

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

        Self {
            idle,
            walk,
            shoot_front_aim,
            shoot_front_fire,
            pain,
            dying,
            corpse,
        }
    }
}

impl FromWorld for MutantSprites {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let idle: [Handle<Image>; 8] = std::array::from_fn(|dir| {
            asset_server.load(format!("enemies/mutant/mutant_idle_a{}.png", dir))
        });

        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            std::array::from_fn(|dir| {
                asset_server.load(format!(
                    "enemies/mutant/mutant_walk_r{}_dir{}.png",
                    row, dir,
                ))
            })
        });

        let pain: Handle<Image> = asset_server.load("enemies/mutant/mutant_pain.png");

        let dying: [Handle<Image>; 4] = std::array::from_fn(|i| {
            asset_server.load(format!("enemies/mutant/mutant_death_{}.png", i))
        });

        let corpse: Handle<Image> = asset_server.load("enemies/mutant/mutant_corpse.png");

        let shoot_front_aim: Handle<Image> = asset_server.load(
            "enemies/mutant/mutant_shoot_front_aim.png",
        );
        let shoot_front_fire_0: Handle<Image> = asset_server.load(
            "enemies/mutant/mutant_shoot_front_fire_0.png",
        );
        let shoot_front_fire_1: Handle<Image> = asset_server.load(
            "enemies/mutant/mutant_shoot_front_fire_1.png",
        );

        Self {
            idle,
            walk,
            shoot_front_aim,
            shoot_front_fire_0,
            shoot_front_fire_1,
            pain,
            dying,
            corpse,
        }
    }
}

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

// --- BOSSES ---
pub(crate) const HANS_SHOOT_SECS: f32 = 0.25;
pub(crate) const GRETEL_SHOOT_SECS: f32 = 0.25;
pub(crate) const HITLER_SHOOT_SECS: f32 = 0.25;
pub(crate) const MECHA_HITLER_SHOOT_SECS: f32 = 0.25;
pub(crate) const GHOST_HITLER_SHOOT_SECS: f32 = 0.25;
pub(crate) const SCHABBS_SHOOT_SECS: f32 = 0.45;
pub(crate) const SCHABBS_THROW_SECS: f32 = 0.25;
pub(crate) const OTTO_SHOOT_SECS: f32 = 0.25;
pub(crate) const GENERAL_SHOOT_SECS: f32 = 0.25;

#[derive(Component)]
pub struct Hans;

#[derive(Component)]
pub struct Gretel;

#[derive(Component)]
pub struct Hitler;

#[derive(Component)]
pub struct MechaHitler;

#[derive(Component)]
pub struct GhostHitler;

#[derive(Component)]
pub struct Schabbs;

#[derive(Component)]
pub struct Otto;

#[derive(Component)]
pub struct General;

#[derive(Component)]
pub struct HansCorpse;

#[derive(Component)]
pub struct GretelCorpse;

#[derive(Component)]
pub struct HitlerCorpse;

#[derive(Component)]
pub struct MechaHitlerCorpse;

#[derive(Component)]
pub struct GhostHitlerCorpse;

#[derive(Component)]
pub struct SchabbsCorpse;

#[derive(Component)]
pub struct OttoCorpse;

#[derive(Component)]
pub struct GeneralCorpse;

#[derive(Component, Default)]
pub struct HansWalk {
    pub phase: f32,
}

#[derive(Component, Default)]
pub struct GretelWalk {
    pub phase: f32,
}

#[derive(Component, Debug, Default)]
pub struct HitlerWalk {
    pub phase: f32,
}

#[derive(Component, Debug, Default)]
pub struct MechaHitlerWalk {
    pub phase: f32,
}

#[derive(Component, Debug, Default)]
pub struct GhostHitlerWalk {
    pub phase: f32,
}

#[derive(Component, Debug, Default)]
pub struct SchabbsWalk {
    pub phase: f32,
}

#[derive(Component, Debug, Default)]
pub struct OttoWalk {
    pub phase: f32,
}

#[derive(Component, Debug, Default)]
pub struct GeneralWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct HansShoot {
    pub t: Timer,
}

#[derive(Component)]
pub struct GretelShoot {
    pub t: Timer,
}

#[derive(Component, Debug)]
pub struct HitlerShoot {
    pub t: Timer,
}

#[derive(Component, Debug)]
pub struct MechaHitlerShoot {
    pub t: Timer,
}

#[derive(Component, Debug)]
pub struct GhostHitlerShoot {
    pub t: Timer,
}

#[derive(Component, Debug)]
pub struct SchabbsShoot {
    pub t: Timer,
}

#[derive(Component, Debug)]
pub struct OttoShoot {
    pub t: Timer,
}

#[derive(Component, Debug)]
pub struct GeneralShoot {
    pub t: Timer,
}

#[derive(Component, Debug)]
pub struct GeneralChaingunVolley {
    pub shots_remaining: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct HansDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GretelDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Debug, Clone, Copy)]
pub struct HitlerDying {
    pub frame: u8, // 0..8
    pub tics: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct MechaHitlerDying {
    pub frame: u8, // 0..4
    pub tics: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GhostHitlerDying {
    pub frame: u8, // 0..4
    pub tics: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SchabbsDying {
    pub frame: u8, // 0..4
    pub tics: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct OttoDying {
    pub frame: u8, // 0..4
    pub tics: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GeneralDying {
    pub frame: u8, // 0..4
    pub tics: u8,
}

#[derive(Resource)]
pub struct HansSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct GretelSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct HitlerSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 8],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct MechaHitlerSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct GhostHitlerSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: Handle<Image>,
    pub fireball: [Handle<Image>; 2],
    pub dying: [Handle<Image>; 4],
    pub corpse: Handle<Image>,
}

#[derive(Resource)]
pub struct SchabbsSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct OttoSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct GeneralSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot_rocket: [Handle<Image>; 2],
    pub shoot_chaingun: [Handle<Image>; 2],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
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

impl FromWorld for GretelSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle = std::array::from_fn(|i| server.load(format!("enemies/gretel/gretel_idle_a{i}.png")));
        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| server.load(format!("enemies/gretel/gretel_walk_r{row}_dir{dir}.png")))
        });

        let shoot: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/gretel/gretel_shoot_{f}.png")));

        let d0: Handle<Image> = server.load("enemies/gretel/gretel_death_0.png");
        let d1: Handle<Image> = server.load("enemies/gretel/gretel_death_1.png");
        let d2: Handle<Image> = server.load("enemies/gretel/gretel_death_2.png");
        let d3: Handle<Image> = server.load("enemies/gretel/gretel_death_3.png");

        let dying = [
            std::array::from_fn(|_| d0.clone()),
            std::array::from_fn(|_| d1.clone()),
            std::array::from_fn(|_| d2.clone()),
            std::array::from_fn(|_| d3.clone()),
        ];

        let corpse0: Handle<Image> = server.load("enemies/gretel/gretel_corpse.png");
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

impl FromWorld for HitlerSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle = std::array::from_fn(|i| server.load(format!("enemies/hitler/hitler_idle_a{i}.png")));
        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| server.load(format!("enemies/hitler/hitler_walk_r{row}_dir{dir}.png")))
        });

        let shoot: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/hitler/hitler_shoot_{f}.png")));

        let d: [Handle<Image>; 8] = std::array::from_fn(|i| server.load(format!("enemies/hitler/hitler_death_{i}.png")));

        let dying: [[Handle<Image>; 8]; 8] = std::array::from_fn(|row| {
            let h = d[row].clone();
            std::array::from_fn(|_| h.clone())
        });

        let corpse0: Handle<Image> = server.load("enemies/hitler/hitler_corpse.png");
        let corpse = std::array::from_fn(|_| corpse0.clone());

        Self { idle, walk, shoot, dying, corpse }
    }
}

impl FromWorld for MechaHitlerSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle = std::array::from_fn(|i| server.load(format!("enemies/mecha_hitler/mecha_hitler_idle_a{i}.png")));
        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| server.load(format!("enemies/mecha_hitler/mecha_hitler_walk_r{row}_dir{dir}.png")))
        });

        let shoot: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/mecha_hitler/mecha_hitler_shoot_{f}.png")));

        let d0: Handle<Image> = server.load("enemies/mecha_hitler/mecha_hitler_death_0.png");
        let d1: Handle<Image> = server.load("enemies/mecha_hitler/mecha_hitler_death_1.png");
        let d2: Handle<Image> = server.load("enemies/mecha_hitler/mecha_hitler_death_2.png");
        let d3: Handle<Image> = server.load("enemies/mecha_hitler/mecha_hitler_death_3.png");

        let dying = [
            std::array::from_fn(|_| d0.clone()),
            std::array::from_fn(|_| d1.clone()),
            std::array::from_fn(|_| d2.clone()),
            std::array::from_fn(|_| d3.clone()),
        ];

        let corpse0: Handle<Image> = server.load("enemies/mecha_hitler/mecha_hitler_corpse.png");
        let corpse = std::array::from_fn(|_| corpse0.clone());

        Self { idle, walk, shoot, dying, corpse }
    }
}

impl FromWorld for GhostHitlerSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle: [Handle<Image>; 8] = std::array::from_fn(|i| {
            server.load(format!("enemies/ghost_hitler/fake_hitler_idle_a{i}.png"))
        });

        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            std::array::from_fn(|dir| {
                server.load(format!(
                    "enemies/ghost_hitler/fake_hitler_walk_r{row}_dir{dir}.png"
                ))
            })
        });

        let shoot: Handle<Image> =
            server.load("enemies/ghost_hitler/fake_hitler_shoot.png");

        let fireball: [Handle<Image>; 2] = [
            server.load("enemies/ghost_hitler/fake_hitler_fireball_0.png"),
            server.load("enemies/ghost_hitler/fake_hitler_fireball_1.png"),
        ];

        let dying: [Handle<Image>; 4] = std::array::from_fn(|i| {
            server.load(format!("enemies/ghost_hitler/fake_hitler_death_{i}.png"))
        });

        let corpse: Handle<Image> =
            server.load("enemies/ghost_hitler/fake_hitler_corpse.png");

        Self {
            idle,
            walk,
            shoot,
            fireball,
            dying,
            corpse,
        }
    }
}

impl FromWorld for SchabbsSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        // Your current Schabbs export only has these four frames
        // Treat them as walk frames 0..3
        let walk_frames: [Handle<Image>; 4] = std::array::from_fn(|i| {
            server.load(format!("enemies/schabbs/schabbs_idle_a{i}.png"))
        });

        // Schabbs has no true idle set, so reuse a mid-walk frame as the stand frame
        let idle: [Handle<Image>; 8] = std::array::from_fn(|_| walk_frames[1].clone());

        // Provide 4 walk frames, duplicated across all 8 views
        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            let h = walk_frames[row].clone();
            std::array::from_fn(|_| h.clone())
        });

        // Your assets are schabbs_shoot_0..1
        // Keep the existing [3] shape by duplicating the last frame
        let shoot0: Handle<Image> = server.load("enemies/schabbs/schabbs_shoot_0.png");
        let shoot1: Handle<Image> = server.load("enemies/schabbs/schabbs_shoot_1.png");
        let shoot: [Handle<Image>; 3] = [shoot0, shoot1.clone(), shoot1];

        // Your assets are schabbs_death_0..2
        // Keep the existing [4] shape by duplicating the last frame
        let d0: Handle<Image> = server.load("enemies/schabbs/schabbs_death_0.png");
        let d1: Handle<Image> = server.load("enemies/schabbs/schabbs_death_1.png");
        let d2: Handle<Image> = server.load("enemies/schabbs/schabbs_death_2.png");

        let dying: [[Handle<Image>; 8]; 4] = [
            std::array::from_fn(|_| d0.clone()),
            std::array::from_fn(|_| d1.clone()),
            std::array::from_fn(|_| d2.clone()),
            std::array::from_fn(|_| d2.clone()),
        ];

        let corpse_one: Handle<Image> = server.load("enemies/schabbs/schabbs_corpse.png");
        let corpse: [Handle<Image>; 8] = std::array::from_fn(|_| corpse_one.clone());

        Self {
            idle,
            walk,
            shoot,
            dying,
            corpse,
        }
    }
}

impl FromWorld for OttoSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        // Walk frames 0..3
        let walk_frames: [Handle<Image>; 4] = std::array::from_fn(|i| {
            server.load(format!("enemies/otto/otto_walk_{i}.png"))
        });

        // Reuse a mid-walk frame as the stand frame
        let idle: [Handle<Image>; 8] = std::array::from_fn(|_| walk_frames[1].clone());

        // Provide 4 walk frames, duplicated across all 8 views
        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            let h = walk_frames[row].clone();
            std::array::from_fn(|_| h.clone())
        });

        let shoot0: Handle<Image> = server.load("enemies/otto/otto_shoot_0.png");
        let shoot1: Handle<Image> = server.load("enemies/otto/otto_shoot_1.png");
        let shoot: [Handle<Image>; 3] = [shoot0, shoot1.clone(), shoot1];

        let d0: Handle<Image> = server.load("enemies/otto/otto_death_0.png");
        let d1: Handle<Image> = server.load("enemies/otto/otto_death_1.png");
        let d2: Handle<Image> = server.load("enemies/otto/otto_death_2.png");

        let dying: [[Handle<Image>; 8]; 4] = [
            std::array::from_fn(|_| d0.clone()),
            std::array::from_fn(|_| d1.clone()),
            std::array::from_fn(|_| d2.clone()),
            std::array::from_fn(|_| d2.clone()),
        ];

        let corpse_one: Handle<Image> = server.load("enemies/otto/otto_corpse.png");
        let corpse: [Handle<Image>; 8] = std::array::from_fn(|_| corpse_one.clone());

        Self {
            idle,
            walk,
            shoot,
            dying,
            corpse,
        }
    }
}

impl FromWorld for GeneralSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        // Walk frames 0..3
        let walk_frames: [Handle<Image>; 4] = std::array::from_fn(|i| {
            server.load(format!("enemies/general/general_walk_{i}.png"))
        });

        // Reuse a mid-walk frame as the stand frame
        let idle: [Handle<Image>; 8] = std::array::from_fn(|_| walk_frames[1].clone());

        // Provide 4 walk frames, duplicated across all 8 views
        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            let h = walk_frames[row].clone();
            std::array::from_fn(|_| h.clone())
        });

        // Load rocket launcher shoot sprites
        let shoot_rocket: [Handle<Image>; 2] = std::array::from_fn(|i| {
            server.load(format!("enemies/general/general_shoot_rocket_{i}.png"))
        });

        // Load chaingun shoot sprites
        let shoot_chaingun: [Handle<Image>; 2] = std::array::from_fn(|i| {
            server.load(format!("enemies/general/general_shoot_chaingun_{i}.png"))
        });

        let d0: Handle<Image> = server.load("enemies/general/general_death_0.png");
        let d1: Handle<Image> = server.load("enemies/general/general_death_1.png");
        let d2: Handle<Image> = server.load("enemies/general/general_death_2.png");

        let dying: [[Handle<Image>; 8]; 4] = [
            std::array::from_fn(|_| d0.clone()),
            std::array::from_fn(|_| d1.clone()),
            std::array::from_fn(|_| d2.clone()),
            std::array::from_fn(|_| d2.clone()),
        ];

        let corpse_one: Handle<Image> = server.load("enemies/general/general_corpse.png");
        let corpse: [Handle<Image>; 8] = std::array::from_fn(|_| corpse_one.clone());

        Self {
            idle,
            walk,
            shoot_rocket,
            shoot_chaingun,
            dying,
            corpse,
        }
    }
}

// SystemParam to group all enemy sprites into a single parameter
// This reduces the setup() function parameter count from 18 to 8
use bevy::ecs::system::SystemParam;

#[derive(SystemParam)]
pub struct AllEnemySprites<'w> {
    pub guards: Res<'w, GuardSprites>,
    pub mutants: Res<'w, MutantSprites>,
    pub ss: Res<'w, SsSprites>,
    pub officers: Res<'w, OfficerSprites>,
    pub dogs: Res<'w, DogSprites>,
    pub hans: Res<'w, HansSprites>,
    pub gretel: Res<'w, GretelSprites>,
    pub mecha_hitler: Res<'w, MechaHitlerSprites>,
    pub ghost_hitler: Res<'w, GhostHitlerSprites>,
    pub schabbs: Res<'w, SchabbsSprites>,
    pub otto: Res<'w, OttoSprites>,
    pub general: Res<'w, GeneralSprites>,
}

fn attach_hans_walk(mut commands: Commands, q: Query<Entity, Added<Hans>>) {
    for e in q.iter() {
        commands.entity(e).insert(HansWalk::default());
    }
}

fn attach_gretel_walk(mut commands: Commands, q: Query<Entity, Added<Gretel>>) {
    for e in q.iter() {
        commands.entity(e).insert(GretelWalk::default());
    }
}

fn attach_hitler_walk(mut commands: Commands, q: Query<Entity, Added<Hitler>>) {
    for e in q.iter() {
        commands.entity(e).insert(HitlerWalk::default());
    }
}

fn attach_mecha_hitler_walk(mut commands: Commands, q: Query<Entity, Added<MechaHitler>>) {
    for e in q.iter() {
        commands.entity(e).insert(MechaHitlerWalk::default());
    }
}

fn attach_ghost_hitler_walk(mut commands: Commands, q: Query<Entity, Added<GhostHitler>>) {
    for e in q.iter() {
        commands.entity(e).insert(GhostHitlerWalk::default());
    }
}

fn attach_schabbs_walk(mut commands: Commands, q: Query<Entity, Added<Schabbs>>) {
    for e in q.iter() {
        commands.entity(e).insert(SchabbsWalk::default());
    }
}

fn attach_otto_walk(mut commands: Commands, q: Query<Entity, Added<Otto>>) {
    for e in q.iter() {
        commands.entity(e).insert(OttoWalk::default());
    }
}

fn attach_general_walk(mut commands: Commands, q: Query<Entity, Added<General>>) {
    for e in q.iter() {
        commands.entity(e).insert(GeneralWalk::default());
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

fn tick_gretel_walk(
    time: Res<Time>,
    mut q: Query<(&mut GretelWalk, Option<&EnemyMove>), (With<Gretel>, Without<GretelDying>)>,
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

fn tick_hitler_walk(
    time: Res<Time>,
    mut q: Query<(&mut HitlerWalk, Option<&EnemyMove>), (With<Hitler>, Without<Dead>)>,
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

fn tick_mecha_hitler_walk(
    time: Res<Time>,
    mut q: Query<(&mut MechaHitlerWalk, Option<&EnemyMove>), (With<MechaHitler>, Without<Dead>)>,
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

fn tick_ghost_hitler_walk(
    time: Res<Time>,
    mut q: Query<(&mut GhostHitlerWalk, Option<&EnemyMove>), (With<GhostHitler>, Without<Dead>)>,
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

fn tick_schabbs_walk(
    time: Res<Time>,
    mut q: Query<(&mut SchabbsWalk, Option<&EnemyMove>), (With<Schabbs>, Without<SchabbsDying>)>,
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

fn tick_otto_walk(
    time: Res<Time>,
    mut q: Query<(&mut OttoWalk, Option<&EnemyMove>), (With<Otto>, Without<OttoDying>)>,
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

fn tick_general_walk(
    time: Res<Time>,
    mut q: Query<(&mut GeneralWalk, Option<&EnemyMove>), (With<General>, Without<GeneralDying>)>,
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

fn tick_gretel_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GretelShoot), With<Gretel>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<GretelShoot>();
        }
    }
}

fn tick_hitler_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut HitlerShoot), With<Hitler>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<HitlerShoot>();
        }
    }
}

fn tick_mecha_hitler_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut MechaHitlerShoot), With<MechaHitler>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<MechaHitlerShoot>();
        }
    }
}

fn tick_ghost_hitler_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GhostHitlerShoot), With<GhostHitler>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<GhostHitlerShoot>();
        }
    }
}

fn tick_schabbs_throw(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SchabbsShoot), With<Schabbs>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<SchabbsShoot>();
        }
    }
}

fn tick_otto_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut OttoShoot), With<Otto>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<OttoShoot>();
        }
    }
}

fn tick_general_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GeneralShoot), With<General>>,
) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<GeneralShoot>();
        }
    }
}

pub fn spawn_hans(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &HansSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        Health::new(boss_health(EnemyKind::Hans, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_gretel(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &GretelSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        Gretel,
        EnemyKind::Gretel,
        Dir8(0),
        View8(0),
        Health::new(boss_health(EnemyKind::Gretel, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_hitler(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &HitlerSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        Hitler,
        DeathCamBoss,
        EnemyKind::Hitler,
        Dir8(0),
        View8(0),
        Health::new(boss_health(EnemyKind::Hitler, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_mecha_hitler(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &MechaHitlerSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        MechaHitler,
        EnemyKind::MechaHitler,
        Dir8(0),
        View8(0),
        Health::new(boss_health(EnemyKind::MechaHitler, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_ghost_hitler(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &GhostHitlerSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        GhostHitler,
        EnemyKind::GhostHitler,
        Dir8(0),
        View8(0),
        Health::new(boss_health(EnemyKind::GhostHitler, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_schabbs(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &SchabbsSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        Schabbs,
        DeathCamBoss,
        EnemyKind::Schabbs,
        Dir8(0),
        View8(0),
        Health::new(boss_health(EnemyKind::Schabbs, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_otto(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &OttoSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        Otto,
        DeathCamBoss,
        EnemyKind::Otto,
        Dir8(0),
        View8(0),
        Health::new(boss_health(EnemyKind::Otto, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_general(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &GeneralSprites,
    tile: IVec2,
    skill: &crate::skill::SkillLevel,
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
        General,
        DeathCamBoss,
        EnemyKind::General,
        Dir8(0),
        View8(0),
        Health::new(boss_health(EnemyKind::General, skill)),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
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

pub fn update_gretel_views(
    sprites: Res<GretelSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&GretelCorpse>,
            Option<&GretelDying>,
            Option<&GretelShoot>,
            Option<&GretelWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Gretel>, Without<Player>),
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

pub fn update_hitler_views(
    sprites: Res<HitlerSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&HitlerCorpse>,
            Option<&HitlerDying>,
            Option<&HitlerShoot>,
            Option<&HitlerWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Hitler>, Without<Player>),
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
            let f = (d.frame as usize).min(7);
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

pub fn update_mecha_hitler_views(
    sprites: Res<MechaHitlerSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&MechaHitlerCorpse>,
            Option<&MechaHitlerDying>,
            Option<&MechaHitlerShoot>,
            Option<&MechaHitlerWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<MechaHitler>, Without<Player>),
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

pub fn update_ghost_hitler_views(
    sprites: Res<GhostHitlerSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&GhostHitlerCorpse>,
            Option<&GhostHitlerDying>,
            Option<&GhostHitlerShoot>,
            Option<&GhostHitlerWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<GhostHitler>, Without<Player>),
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
            sprites.corpse.clone()
        } else if let Some(d) = dying {
            let f = (d.frame as usize).min(3);
            sprites.dying[f].clone()
        } else if shoot.is_some() {
            sprites.shoot.clone()
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

pub fn update_schabbs_views(
    sprites: Res<SchabbsSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&SchabbsCorpse>,
            Option<&SchabbsDying>,
            Option<&SchabbsShoot>,
            Option<&SchabbsWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Schabbs>, Without<Player>),
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
            let fi = ((t / dur) * 2.0).floor() as usize;
            sprites.shoot[fi.min(1)].clone()
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

pub fn update_otto_views(
    sprites: Res<OttoSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&OttoCorpse>,
            Option<&OttoDying>,
            Option<&OttoShoot>,
            Option<&OttoWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Otto>, Without<Player>),
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
            let fi = ((t / dur) * 2.0).floor() as usize;
            sprites.shoot[fi.min(1)].clone()
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

pub fn update_general_views(
    sprites: Res<GeneralSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&GeneralCorpse>,
            Option<&GeneralDying>,
            Option<&GeneralShoot>,
            Option<&GeneralChaingunVolley>,
            Option<&GeneralWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<General>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (corpse, dying, shoot, chaingun, walk, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
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
        } else if chaingun.is_some() {
            // Chaingun volley - show chaingun sprites
            // Alternate between frames 0 and 1 based on shots remaining (creates firing effect)
            let frame = if chaingun.unwrap().shots_remaining % 2 == 0 { 1 } else { 0 };
            sprites.shoot_chaingun[frame].clone()
        } else if let Some(s) = shoot {
            // Rocket shoot - show rocket sprites
            let dur = s.t.duration().as_secs_f32().max(1e-6);
            let t = s.t.elapsed().as_secs_f32();
            let fi = ((t / dur) * 2.0).floor() as usize;
            sprites.shoot_rocket[fi.min(1)].clone()
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

fn tick_gretel_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GretelDying)>,
) {
    for (e, mut d) in q.iter_mut() {
        // Advance animation by tics, not by a Timer
        d.tics = d.tics.saturating_add(1);

        if d.tics >= 8 {
            d.tics = 0;
            d.frame = d.frame.saturating_add(1);

            if d.frame >= 4 {
                commands.entity(e).remove::<GretelDying>();
                commands.entity(e).insert(GretelCorpse);
            }
        }
    }
}

fn tick_hitler_dying(
	mut commands: Commands,
	mut q: Query<(Entity, &mut HitlerDying, Option<&DeathCamReplaySlow>), With<Hitler>>,
) {
	const BASE_TICS_PER_FRAME: u8 = 8;
	const FRAMES: u8 = 8;

	for (e, mut d, slow) in q.iter_mut() {
		let mul = slow.map(|s| s.0.max(1)).unwrap_or(1);
		let tics_per_frame = BASE_TICS_PER_FRAME.saturating_mul(mul);

		d.tics = d.tics.saturating_add(1);
		if d.tics < tics_per_frame {
			continue;
		}

		d.tics = 0;
		d.frame = d.frame.saturating_add(1);

		if d.frame >= FRAMES {
			commands.entity(e).remove::<HitlerDying>();
			commands.entity(e).remove::<DeathCamReplaySlow>();
			commands.entity(e).insert(HitlerCorpse);
		}
	}
}

fn tick_mecha_hitler_dying(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    hitler_sprites: Res<HitlerSprites>,
    skill_level: Res<crate::skill::SkillLevel>,
    mut q: Query<(Entity, &mut MechaHitlerDying, &OccupiesTile), With<MechaHitler>>,
) {
    const TICS_PER_FRAME: u8 = 8;
    const FRAMES: u8 = 4;

    for (e, mut d, occ) in q.iter_mut() {
        d.tics += 1;
        if d.tics < TICS_PER_FRAME {
            continue;
        }

        d.tics = 0;
        d.frame += 1;

        if d.frame >= FRAMES {
            commands.entity(e).remove::<MechaHitlerDying>();
            commands.entity(e).insert(MechaHitlerCorpse);

            // Spawn Hitler phase2 at same tile
            spawn_hitler(&mut commands, &mut meshes, &mut materials, &hitler_sprites, occ.0, &skill_level);
        }
    }
}

fn tick_ghost_hitler_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GhostHitlerDying), With<GhostHitler>>,
) {
    const TICS_PER_FRAME: u8 = 8;
    const FRAMES: u8 = 4;

    for (e, mut d) in q.iter_mut() {
        d.tics += 1;
        if d.tics < TICS_PER_FRAME {
            continue;
        }

        d.tics = 0;
        d.frame += 1;

        if d.frame >= FRAMES {
            commands.entity(e).remove::<GhostHitlerDying>();
            commands.entity(e).insert(GhostHitlerCorpse);
        }
    }
}

fn tick_schabbs_dying(
	mut commands: Commands,
	mut q: Query<(Entity, &mut SchabbsDying, Option<&DeathCamReplaySlow>)>,
) {
	const BASE_TICS_PER_FRAME: u8 = 8;
	const FRAMES: u8 = 3;

	for (e, mut d, slow) in q.iter_mut() {
		let mul = slow.map(|s| s.0.max(1)).unwrap_or(1);
		let tics_per_frame = BASE_TICS_PER_FRAME.saturating_mul(mul);

		d.tics = d.tics.saturating_add(1);

		if d.tics >= tics_per_frame {
			d.tics = 0;
			d.frame = d.frame.saturating_add(1);

			if d.frame >= FRAMES {
				commands.entity(e).remove::<SchabbsDying>();
				commands.entity(e).remove::<DeathCamReplaySlow>();
				commands.entity(e).insert(SchabbsCorpse);
			}
		}
	}
}

fn tick_otto_dying(
	mut commands: Commands,
	mut q: Query<(Entity, &mut OttoDying, Option<&DeathCamReplaySlow>)>,
) {
	const BASE_TICS_PER_FRAME: u8 = 8;
	const FRAMES: u8 = 3;

	for (e, mut d, slow) in q.iter_mut() {
		let mul = slow.map(|s| s.0.max(1)).unwrap_or(1);
		let tics_per_frame = BASE_TICS_PER_FRAME.saturating_mul(mul);

		d.tics = d.tics.saturating_add(1);

		if d.tics >= tics_per_frame {
			d.tics = 0;
			d.frame = d.frame.saturating_add(1);

			if d.frame >= FRAMES {
				commands.entity(e).remove::<OttoDying>();
				commands.entity(e).remove::<DeathCamReplaySlow>();
				commands.entity(e).insert(OttoCorpse);
			}
		}
	}
}

fn tick_general_dying(
	mut commands: Commands,
	mut q: Query<(Entity, &mut GeneralDying, Option<&DeathCamReplaySlow>)>,
) {
	const BASE_TICS_PER_FRAME: u8 = 8;
	const FRAMES: u8 = 3;

	for (e, mut d, slow) in q.iter_mut() {
		let mul = slow.map(|s| s.0.max(1)).unwrap_or(1);
		let tics_per_frame = BASE_TICS_PER_FRAME.saturating_mul(mul);

		d.tics = d.tics.saturating_add(1);

		if d.tics >= tics_per_frame {
			d.tics = 0;
			d.frame = d.frame.saturating_add(1);

			if d.frame >= FRAMES {
				commands.entity(e).remove::<GeneralDying>();
				commands.entity(e).remove::<DeathCamReplaySlow>();
				commands.entity(e).insert(GeneralCorpse);
			}
		}
	}
}

// --- FUNCTIONS ---
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

fn attach_ss_walk(mut commands: Commands, q: Query<Entity, Added<Ss>>) {
    for e in q.iter() {
        commands.entity(e).insert(SsWalk::default());
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

// enemies.rs
fn tick_ss_shoot(
    time: Res<Time>,
    mut commands: Commands,
    q_active_shoot: Query<(Entity, &crate::audio::ActiveEnemyShootSfx)>,
    mut q: Query<(Entity, &mut SsShoot)>,
) {
    let mut any_ss_still_shooting = false;
    let mut any_ss_finished = false;

    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            any_ss_finished = true;
            commands.entity(e).remove::<SsShoot>();
        } else {
            any_ss_still_shooting = true;
        }
    }

    if any_ss_finished && !any_ss_still_shooting {
        for (e, a) in q_active_shoot.iter() {
            if a.kind == EnemyKind::Ss {
                commands.entity(e).despawn();
            }
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
            // GuardShoot Has Only Timer, Pick Aim vs Fire Based on Timer Progress
            let dur = s.timer.duration().as_secs_f32().max(1e-6);
            let t = s.timer.elapsed().as_secs_f32();
            let fire_phase = t >= (dur * 0.5);

            if fire_phase {
                sprites.shoot_front_fire.clone()
            } else {
                sprites.shoot_front_aim.clone()
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

fn update_mutant_views(
    sprites: Res<MutantSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Entity,
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
    mut last_fire_alt: Local<std::collections::HashMap<Entity, bool>>,
    mut shooting_now: Local<std::collections::HashSet<Entity>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    shooting_now.retain(|e| q.get(*e).is_ok());
    last_fire_alt.retain(|e, _| q.get(*e).is_ok());

    for (e, _dead, corpse, dying, pain, walk, shoot, mv, gt, dir8, mut view, mat3d, mut tf) in
        q.iter_mut()
    {
        let enemy_pos = gt.translation();

        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        if shoot.is_some() {
            if !shooting_now.contains(&e) {
                let prev = last_fire_alt.get(&e).copied().unwrap_or(true);
                last_fire_alt.insert(e, !prev);
                shooting_now.insert(e);
            }
        } else {
            shooting_now.remove(&e);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse.clone()
        } else if let Some(d) = dying {
            let i = (d.frame as usize).min(sprites.dying.len().saturating_sub(1));
            sprites.dying[i].clone()
        } else if pain.is_some() {
            sprites.pain.clone()
        } else if let Some(s) = shoot {
            let dur = s.timer.duration().as_secs_f32().max(1e-6);
            let t = s.timer.elapsed().as_secs_f32();
            let fire_phase = t >= (dur * 0.5);

            if fire_phase {
                let alt = last_fire_alt.get(&e).copied().unwrap_or(false);
                if alt {
                    sprites.shoot_front_fire_1.clone()
                } else {
                    sprites.shoot_front_fire_0.clone()
                }
            } else {
                sprites.shoot_front_aim.clone()
            }
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

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuardSprites>()
            .init_resource::<MutantSprites>()
            .init_resource::<SsSprites>()
            .init_resource::<OfficerSprites>()
            .init_resource::<DogSprites>()
            .init_resource::<HansSprites>()
            .init_resource::<GretelSprites>()
            .init_resource::<HitlerSprites>()
            .init_resource::<MechaHitlerSprites>()
            .init_resource::<GhostHitlerSprites>()
            .init_resource::<SchabbsSprites>()
            .init_resource::<OttoSprites>()
            .init_resource::<GeneralSprites>()
            // Update Systems: Attach Walk Components
            .add_systems(Update, attach_guard_walk)
            .add_systems(Update, attach_mutant_walk)
            .add_systems(Update, attach_ss_walk)
            .add_systems(Update, attach_officer_walk)
            .add_systems(Update, attach_dog_walk)
            .add_systems(Update, attach_hans_walk)
            .add_systems(Update, attach_gretel_walk)
            .add_systems(Update, attach_hitler_walk)
            .add_systems(Update, attach_mecha_hitler_walk)
            .add_systems(Update, attach_ghost_hitler_walk)
            .add_systems(Update, attach_schabbs_walk)
            .add_systems(Update, attach_otto_walk)
            .add_systems(Update, attach_general_walk)
            // Update Systems: Update Views
            .add_systems(Update, update_guard_views)
            .add_systems(Update, update_mutant_views)
            .add_systems(Update, update_ss_views)
            .add_systems(Update, update_officer_views)
            .add_systems(Update, update_dog_views)
            .add_systems(Update, update_hans_views)
            .add_systems(Update, update_gretel_views)
            .add_systems(Update, update_hitler_views)
            .add_systems(Update, update_mecha_hitler_views)
            .add_systems(Update, update_ghost_hitler_views)
            .add_systems(Update, update_schabbs_views)
            .add_systems(Update, update_otto_views)
            .add_systems(Update, update_general_views)
            // FixedUpdate Systems: Guards
            .add_systems(FixedUpdate, tick_guard_walk)
            .add_systems(FixedUpdate, tick_guard_pain)
            .add_systems(FixedUpdate, tick_guard_shoot)
            .add_systems(FixedUpdate, tick_guard_dying)
            // FixedUpdate Systems: Mutants
            .add_systems(FixedUpdate, tick_mutant_walk)
            .add_systems(FixedUpdate, tick_mutant_pain)
            .add_systems(FixedUpdate, tick_mutant_shoot)
            .add_systems(FixedUpdate, tick_mutant_dying)
            // FixedUpdate Systems: SS
            .add_systems(FixedUpdate, tick_ss_walk)
            .add_systems(FixedUpdate, tick_ss_pain)
            .add_systems(FixedUpdate, tick_ss_shoot)
            .add_systems(FixedUpdate, tick_ss_dying)
            // FixedUpdate Systems: Officers
            .add_systems(FixedUpdate, tick_officer_walk)
            .add_systems(FixedUpdate, tick_officer_pain)
            .add_systems(FixedUpdate, tick_officer_shoot)
            .add_systems(FixedUpdate, tick_officer_dying)
            // FixedUpdate Systems: Dogs
            .add_systems(FixedUpdate, tick_dog_walk)
            .add_systems(FixedUpdate, tick_dog_pain)
            .add_systems(FixedUpdate, tick_dog_bite_cooldown)
            .add_systems(FixedUpdate, tick_dog_bite)
            .add_systems(FixedUpdate, tick_dog_dying)
            // FixedUpdate Systems: Hans
            .add_systems(FixedUpdate, tick_hans_walk)
            .add_systems(FixedUpdate, tick_hans_shoot)
            .add_systems(FixedUpdate, tick_hans_dying)
            // FixedUpdate Systems: Gretel
            .add_systems(FixedUpdate, tick_gretel_walk)
            .add_systems(FixedUpdate, tick_gretel_shoot)
            .add_systems(FixedUpdate, tick_gretel_dying)
            // FixedUpdate Systems: Mecha Hitler
            .add_systems(FixedUpdate, tick_mecha_hitler_walk)
            .add_systems(FixedUpdate, tick_mecha_hitler_shoot)
            .add_systems(FixedUpdate, tick_mecha_hitler_dying)
            // FixedUpdate Systems: Hitler
            .add_systems(FixedUpdate, tick_hitler_walk)
            .add_systems(FixedUpdate, tick_hitler_shoot)
            .add_systems(FixedUpdate, tick_hitler_dying)
            // FixedUpdate Systems: Ghost Hitler
            .add_systems(FixedUpdate, tick_ghost_hitler_walk)
            .add_systems(FixedUpdate, tick_ghost_hitler_shoot)
            .add_systems(FixedUpdate, tick_ghost_hitler_dying)
            // FixedUpdate Systems: Schabbs
            .add_systems(FixedUpdate, tick_schabbs_walk)
            .add_systems(FixedUpdate, tick_schabbs_throw)
            .add_systems(FixedUpdate, tick_schabbs_dying)
            // FixedUpdate Systems: Otto
            .add_systems(FixedUpdate, tick_otto_walk)
            .add_systems(FixedUpdate, tick_otto_shoot)
            .add_systems(FixedUpdate, tick_otto_dying)
            // FixedUpdate Systems: General
            .add_systems(FixedUpdate, tick_general_walk)
            .add_systems(FixedUpdate, tick_general_shoot)
            .add_systems(FixedUpdate, tick_general_dying);
    }
}
