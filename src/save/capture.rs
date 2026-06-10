/*
Davenstein - by David Petnick

This Module is the Bridge Between Live ECS State and the Serializable Model
- capture_save_game(): Reads Live State -> SaveGame (SAVE Side, Pure Reads)
- The RESTORE Side Lives in restart::load_game_finish (in restart.rs), Because
  It Must Run Inside the Level-Rebuild "Finish" Stage After the World Exists
  The Helpers Here (apply_run_state / apply_player) Are Called From There
*/

use bevy::prelude::*;

use crate::save::model::*;
use davelib::enemies::{EnemyKind, SpawnIndex};
use davelib::level::{CurrentLevel, LevelId};
use davelib::level_score::LevelScore;
use davelib::player::{
    PlayerKeys,
    PlayerVitals,
};
use crate::ui::HudState;

// ---------- LevelId <-> LevelRef ----------
// Explicit Conversion (Not Enum-Cast Based) so It Cannot Silently Break if the
// LevelId Enum is Ever Reordered. Mirrors the Project's Existing Explicit Style

pub fn level_to_ref(level: LevelId) -> LevelRef {
    LevelRef {
        episode: level.episode(),
        floor: level.floor_number() as u8,
    }
}

/// Reverse of level_to_ref. Clamps Out-of-Range Input to Valid Bounds
pub fn level_from_ref(r: LevelRef) -> LevelId {
    level_id_from_episode_floor(r.episode, r.floor)
}

/// (Episode 1-6, Floor 1-10) -> LevelId, via Explicit Table. Clamps Garbage
fn level_id_from_episode_floor(episode: u8, floor: u8) -> LevelId {
    use LevelId::*;
    let e = episode.clamp(1, 6);
    let f = floor.clamp(1, 10);
    match (e, f) {
        (1, 1) => E1M1, (1, 2) => E1M2, (1, 3) => E1M3, (1, 4) => E1M4, (1, 5) => E1M5,
        (1, 6) => E1M6, (1, 7) => E1M7, (1, 8) => E1M8, (1, 9) => E1M9, (1, 10) => E1M10,
        (2, 1) => E2M1, (2, 2) => E2M2, (2, 3) => E2M3, (2, 4) => E2M4, (2, 5) => E2M5,
        (2, 6) => E2M6, (2, 7) => E2M7, (2, 8) => E2M8, (2, 9) => E2M9, (2, 10) => E2M10,
        (3, 1) => E3M1, (3, 2) => E3M2, (3, 3) => E3M3, (3, 4) => E3M4, (3, 5) => E3M5,
        (3, 6) => E3M6, (3, 7) => E3M7, (3, 8) => E3M8, (3, 9) => E3M9, (3, 10) => E3M10,
        (4, 1) => E4M1, (4, 2) => E4M2, (4, 3) => E4M3, (4, 4) => E4M4, (4, 5) => E4M5,
        (4, 6) => E4M6, (4, 7) => E4M7, (4, 8) => E4M8, (4, 9) => E4M9, (4, 10) => E4M10,
        (5, 1) => E5M1, (5, 2) => E5M2, (5, 3) => E5M3, (5, 4) => E5M4, (5, 5) => E5M5,
        (5, 6) => E5M6, (5, 7) => E5M7, (5, 8) => E5M8, (5, 9) => E5M9, (5, 10) => E5M10,
        (6, 1) => E6M1, (6, 2) => E6M2, (6, 3) => E6M3, (6, 4) => E6M4, (6, 5) => E6M5,
        (6, 6) => E6M6, (6, 7) => E6M7, (6, 8) => E6M8, (6, 9) => E6M9, (6, 10) => E6M10,
        // Clamp Guarantees We Never Reach Here, but Match Must Be Exhaustive
        _ => E1M1,
    }
}

// ---------- EnemyKind -> u8 ----------
// Explicit, Stable Numbering for the Save Format. Do Not Rely on Enum Cast
// Order so the Format Survives Any Future Reordering of EnemyKind

pub fn enemy_kind_to_u8(kind: EnemyKind) -> u8 {
    match kind {
        EnemyKind::Guard => 0,
        EnemyKind::Ss => 1,
        EnemyKind::Officer => 2,
        EnemyKind::Mutant => 3,
        EnemyKind::Dog => 4,
        EnemyKind::Hans => 5,
        EnemyKind::Gretel => 6,
        EnemyKind::Hitler => 7,
        EnemyKind::MechaHitler => 8,
        EnemyKind::GhostHitler => 9,
        EnemyKind::Schabbs => 10,
        EnemyKind::Otto => 11,
        EnemyKind::General => 12,
    }
}

/// Build the Dead-Enemy List From the Dead Enemies Currently in the World
/// Each is Identified by Its Kind and Stable Per-Kind Spawn Index, Which is All
/// Restore Needs to Put It Back as a Corpse on Load
pub fn capture_dead_enemies(
    dead: &[(EnemyKind, SpawnIndex)],
) -> Vec<DeadEnemy> {
    dead.iter()
        .map(|(kind, idx)| DeadEnemy {
            kind: enemy_kind_to_u8(*kind),
            index: idx.0,
        })
        .collect()
}

/// Build a Bucket-1 SaveGame From Current Live State. Pure Reads, No Mutation
/// Call This From a Save Trigger System That Has These Params Available
pub fn capture_save_game(
    name: String,
    hud: &HudState,
    player_tf: &Transform,
    player_vitals: &PlayerVitals,
    current_level: &CurrentLevel,
    level_score: &LevelScore,
    dead_enemies: Vec<DeadEnemy>,
    present_pickups: Vec<[i32; 2]>,
    open_doors: Vec<[i32; 2]>,
) -> SaveGame {
    // Facing: Derive Yaw/Pitch From the Player's Transform Rotation. The Camera
    // Rotation is the Source of Truth (mouse_look Writes It via
    // Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)), so Decompose the Same Way
    let (yaw, pitch, _roll) = player_tf.rotation.to_euler(EulerRot::YXZ);

    let run_state = RunState {
        hp: hud.hp,
        ammo: hud.ammo,
        score: hud.score,
        lives: hud.lives,
        key_gold: hud.key_gold,
        key_silver: hud.key_silver,
        selected_weapon: hud.selected as u8,
        owned_mask: hud.owned_mask,
    };

    let player = PlayerSnapshot {
        pos: [
            player_tf.translation.x,
            player_tf.translation.y,
            player_tf.translation.z,
        ],
        yaw,
        pitch,
        hp: player_vitals.hp,
        hp_max: player_vitals.hp_max,
    };

    let level = level_to_ref(current_level.0);

    let level_score_snap = LevelScoreSnapshot {
        kills_found: level_score.kills_found,
        kills_total: level_score.kills_total,
        secrets_found: level_score.secrets_found,
        secrets_total: level_score.secrets_total,
        treasure_found: level_score.treasure_found,
        treasure_total: level_score.treasure_total,
        time_secs: level_score.time_secs,
    };

    let mut game = SaveGame::new_bucket1(name, run_state, player, level, level_score_snap);
    game.world = Some(WorldSnapshot { dead_enemies, present_pickups, open_doors });
    game
}

// ---------- RESTORE Side Helpers ----------
// These Are Called From restart::load_game_finish After the World is Rebuilt

/// Stamp Saved Run State Onto the HUD Resource (Overrides the Fresh Defaults)
pub fn apply_run_state(hud: &mut HudState, rs: &RunState) {
    hud.hp = rs.hp;
    hud.ammo = rs.ammo;
    hud.score = rs.score;
    hud.lives = rs.lives;
    hud.key_gold = rs.key_gold;
    hud.key_silver = rs.key_silver;
    hud.owned_mask = rs.owned_mask;
    hud.selected = weapon_from_u8(rs.selected_weapon);
}

/// Stamp Saved Player Position/Facing/Vitals Onto the Freshly Spawned Player
/// Returns the (Yaw, Pitch) so the Caller Can Also Reset LookAngles, Keeping
/// Mouse-Look Continuous From the Restored Angle (Otherwise the Next Mouse Move
/// Snaps the Camera Back to the Default-Spawn Angle)
pub fn apply_player(
    tf: &mut Transform,
    vitals: &mut PlayerVitals,
    keys: &mut PlayerKeys,
    snap: &PlayerSnapshot,
    rs: &RunState,
) -> (f32, f32) {
    tf.translation = Vec3::new(snap.pos[0], snap.pos[1], snap.pos[2]);
    tf.rotation = Quat::from_euler(EulerRot::YXZ, snap.yaw, snap.pitch, 0.0);

    vitals.hp = snap.hp;
    vitals.hp_max = snap.hp_max;

    // Keys Live on Both HudState and the Player's PlayerKeys Component, Keep Them
    // Consistent so the Door-Use Logic (Which Reads PlayerKeys) Matches the HUD
    keys.gold = rs.key_gold;
    keys.silver = rs.key_silver;

    (snap.yaw, snap.pitch)
}

/// Stamp Saved Per-Level Score Back Onto the LevelScore Resource
pub fn apply_level_score(score: &mut LevelScore, snap: &LevelScoreSnapshot) {
    score.kills_found = snap.kills_found;
    score.kills_total = snap.kills_total;
    score.secrets_found = snap.secrets_found;
    score.secrets_total = snap.secrets_total;
    score.treasure_found = snap.treasure_found;
    score.treasure_total = snap.treasure_total;
    score.time_secs = snap.time_secs;
}

fn weapon_from_u8(v: u8) -> crate::combat::WeaponSlot {
    use crate::combat::WeaponSlot::*;
    match v {
        0 => Knife,
        1 => Pistol,
        2 => MachineGun,
        3 => Chaingun,
        _ => Pistol, // Safe Fallback
    }
}
