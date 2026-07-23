/*
Davenstein - by David Petnick
*/

use bevy::prelude::*;

use crate::enemies::{Dir8, EnemyKind};

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Patrol;

/// Plane1 Path-Arrow Codes (ICONARROWS = 90 ..= 97) to Dir8.
///
/// In the Original, `arrowtile - 90` is a dirtype Directly, in the Compass Order
/// E, NE, N, NW, W, SW, S, SE (WOLFSRC/WL_STATE.C SelectPathDir). Davenstein's
/// Dir8 is That Same Compass Rotated by +2 (Dir8 0 = South, 2 = East, 4 = North,
/// 6 = West), so the Conversion is Just `(dirtype + 2) & 7`.
pub fn patrol_dir_from_plane1(code: u16) -> Option<Dir8> {
    if (90..=97).contains(&code) {
        let dirtype = (code - 90) as u8;
        Some(Dir8((dirtype + 2) & 7))
    } else {
        None
    }
}

// The & 7 on the match input means only 0 – 7 are possible,
// all of which are covered. unreachable!() is more correct than
// silently returning ZERO for an impossible case
pub fn patrol_step_8way(dir: Dir8) -> IVec2 {
    match dir.0 & 7 {
        0 => IVec2::new(0, 1),
        1 => IVec2::new(1, 1),
        2 => IVec2::new(1, 0),
        3 => IVec2::new(1, -1),
        4 => IVec2::new(0, -1),
        5 => IVec2::new(-1, -1),
        6 => IVec2::new(-1, 0),
        7 => IVec2::new(-1, 1),
        _ => unreachable!(),
    }
}

fn wolf_dir4_to_dir8(dir4: u8) -> Dir8 {
    // Original SpawnStand / SpawnPatrol Set `new->dir = dir * 2`, so the Map's
    // dir4 (0..3) Selects the dirtype East, North, West, South. Rotate Into
    // Davenstein's Dir8 (+2) the Same Way patrol_dir_from_plane1 Does.
    let dirtype = (dir4 & 3) * 2; // 0, 2, 4, 6 = East, North, West, South
    Dir8((dirtype + 2) & 7)
}

fn spawn_dir_and_patrol_from_bands(code: u16, base: u16) -> Option<(Dir8, bool)> {
    // Difficulty Bands: Base, Base+36, Base+72
    for off in [0u16, 36u16, 72u16] {
        let start = base + off;
        if (start..=start + 7).contains(&code) {
            let i = (code - start) as u8; // 0..7
            let is_patrol = i >= 4;
            let dir4 = i & 3; // 0..3
            return Some((wolf_dir4_to_dir8(dir4), is_patrol));
        }
    }
    None
}

/// For a Spawned Enemy, Derive Facing + Patrol From the Raw Wolfenstein 3D
/// Plane1 Code. Bases Match id's ScanInfoPlane (WOLFSRC/WL_GAME.C): Stand Codes
/// at `base .. base+3`, Patrol at `base+4 .. base+7`, Repeated per Difficulty
/// at +36 and +72.
pub fn spawn_dir_and_patrol_for_kind(kind: EnemyKind, code: u16) -> Option<(Dir8, bool)> {
    match kind {
        EnemyKind::Guard => spawn_dir_and_patrol_from_bands(code, 108),
        EnemyKind::Officer => spawn_dir_and_patrol_from_bands(code, 116),
        EnemyKind::Ss => spawn_dir_and_patrol_from_bands(code, 126),
        EnemyKind::Dog => spawn_dir_and_patrol_from_bands(code, 134),
        EnemyKind::Mutant => spawn_dir_and_patrol_from_bands(code, 216),
        _ => None,
    }
}
