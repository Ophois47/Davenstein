/*
Davenstein - by David Petnick

Wolf-style patrol helpers.

Coordinate conventions in this project:
- Map tiles use IVec2(x, y) where y corresponds to world +Z
- In Wolf's plane dumps, rows increase "south"
- Our Dir8 convention (see enemies.rs / ai.rs) is:
    Dir8(0)=+Z (south)
    Dir8(2)=+X (east)
    Dir8(4)=-Z (north)
    Dir8(6)=-X (west)
  with diagonals in between.
*/

use bevy::prelude::*;

use crate::enemies::{Dir8, EnemyKind};

/// Marker/state for an actor that should patrol along Wolf path arrows
///
/// `diag_phase` is used to emulate Wolf's diagonal stair-stepping:
/// diagonal directions alternate between their X and Y components
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Patrol {
    pub diag_phase: bool,
}

/// Wolf plane1 path arrow codes (90..=97) to our Dir8 convention
///
/// Wolf arrow meanings (map-space): N, E, S, W, NE, SE, SW, NW
///
/// In our map:
/// - +Y is "south", so N is (0,-1) => Dir8(4)
/// - E is (+1,0) => Dir8(2)
/// - S is (0,+1) => Dir8(0)
/// - W is (-1,0) => Dir8(6)
pub fn patrol_dir_from_plane1(code: u16) -> Option<Dir8> {
    match code {
        90 => Some(Dir8(4)), // N
        91 => Some(Dir8(2)), // E
        92 => Some(Dir8(0)), // S
        93 => Some(Dir8(6)), // W
        94 => Some(Dir8(3)), // NE
        95 => Some(Dir8(1)), // SE
        96 => Some(Dir8(7)), // SW
        97 => Some(Dir8(5)), // NW
        _ => None,
    }
}

/// Decide the next patrol step (tile delta) given the current facing
/// For diagonal facings, alternate between the two axis steps
pub fn patrol_step_4way(dir: Dir8, diag_phase: bool) -> (IVec2, bool) {
    match dir.0 & 7 {
        // Cardinals
        0 => (IVec2::new(0, 1), diag_phase),   // +Y / +Z (south)
        2 => (IVec2::new(1, 0), diag_phase),   // +X (east)
        4 => (IVec2::new(0, -1), diag_phase),  // -Y / -Z (north)
        6 => (IVec2::new(-1, 0), diag_phase),  // -X (west)

        // Diagonals (stair-step)
        1 => {
            // SE: (1, 1)
            if diag_phase {
                (IVec2::new(1, 0), false)
            } else {
                (IVec2::new(0, 1), true)
            }
        }
        3 => {
            // NE: (1, -1)
            if diag_phase {
                (IVec2::new(1, 0), false)
            } else {
                (IVec2::new(0, -1), true)
            }
        }
        5 => {
            // NW: (-1, -1)
            if diag_phase {
                (IVec2::new(-1, 0), false)
            } else {
                (IVec2::new(0, -1), true)
            }
        }
        7 => {
            // SW: (-1, 1)
            if diag_phase {
                (IVec2::new(-1, 0), false)
            } else {
                (IVec2::new(0, 1), true)
            }
        }
        _ => (IVec2::ZERO, diag_phase),
    }
}

fn wolf_dir4_to_dir8(dir4: u8) -> Dir8 {
    // Wolf's 4 directions are N/E/S/W
    match dir4 & 3 {
        0 => Dir8(4), // N => -Y / -Z
        1 => Dir8(2), // E => +X
        2 => Dir8(0), // S => +Y / +Z
        3 => Dir8(6), // W => -X
        _ => Dir8(0),
    }
}

fn spawn_dir_and_patrol_from_bands(code: u16, base: u16) -> Option<(Dir8, bool)> {
    // Wolf difficulty bands: base, base+36, base+72
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

/// For a spawned enemy, derive (initial facing, patrol?) from the raw Wolf plane1 code
pub fn spawn_dir_and_patrol_for_kind(kind: EnemyKind, code: u16) -> Option<(Dir8, bool)> {
    match kind {
        EnemyKind::Guard => spawn_dir_and_patrol_from_bands(code, 108),
        EnemyKind::Ss => spawn_dir_and_patrol_from_bands(code, 126),
        EnemyKind::Dog => spawn_dir_and_patrol_from_bands(code, 134),
        _ => None,
    }
}
