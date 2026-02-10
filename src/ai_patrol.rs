/*
Davenstein - by David Petnick
*/

use bevy::prelude::*;

use crate::enemies::{Dir8, EnemyKind};

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Patrol;

/// Plane1 Path Arrow Codes (90..=97) to Dir8 Convention
/// Wolf Arrow Meanings (Map-Space): N, E, S, W, NE, SE, SW, NW
/// - +Y is "South", so N is (0,-1) => Dir8(4)
/// - E is (+1,0) => Dir8(2)
/// - S is (0,+1) => Dir8(0)
/// - W is (-1,0) => Dir8(6)
pub fn patrol_dir_from_plane1(code: u16) -> Option<Dir8> {
    match code {
        90 => Some(Dir8(4)),
        91 => Some(Dir8(2)),
        92 => Some(Dir8(0)),
        93 => Some(Dir8(6)),
        94 => Some(Dir8(3)),
        95 => Some(Dir8(1)),
        96 => Some(Dir8(7)),
        97 => Some(Dir8(5)),
        _ => None,
    }
}

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
        _ => IVec2::ZERO,
    }
}

fn wolf_dir4_to_dir8(dir4: u8) -> Dir8 {
    // 4 Directions N/E/S/W
    match dir4 & 3 {
        0 => Dir8(4), // N => -Y / -Z
        1 => Dir8(2), // E => +X
        2 => Dir8(0), // S => +Y / +Z
        3 => Dir8(6), // W => -X
        _ => Dir8(0),
    }
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

/// For Spawned Enemy, Derive From Raw Wolfenstein 3D Plane1 Code
pub fn spawn_dir_and_patrol_for_kind(kind: EnemyKind, code: u16) -> Option<(Dir8, bool)> {
    match kind {
        EnemyKind::Guard => spawn_dir_and_patrol_from_bands(code, 108),
        EnemyKind::Ss => spawn_dir_and_patrol_from_bands(code, 126),
        EnemyKind::Dog => spawn_dir_and_patrol_from_bands(code, 134),
        _ => None,
    }
}
