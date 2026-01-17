/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelId {
    // Episode 1
    E1M1, E1M2, E1M3, E1M4, E1M5, E1M6, E1M7, E1M8, E1M9, E1M10,
    // Episode 2
    E2M1, E2M2, E2M3, E2M4, E2M5, E2M6, E2M7, E2M8, E2M9, E2M10,
    // Episode 3
    E3M1, E3M2, E3M3, E3M4, E3M5, E3M6, E3M7, E3M8, E3M9, E3M10,
    // Episode 4
    E4M1, E4M2, E4M3, E4M4, E4M5, E4M6, E4M7, E4M8, E4M9, E4M10,
    // Episode 5
    E5M1, E5M2, E5M3, E5M4, E5M5, E5M6, E5M7, E5M8, E5M9, E5M10,
    // Episode 6
    E6M1, E6M2, E6M3, E6M4, E6M5, E6M6, E6M7, E6M8, E6M9, E6M10,
}

impl LevelId {
    pub const fn episode(self) -> u8 {
        match self {
            LevelId::E1M1 | LevelId::E1M2 | LevelId::E1M3 | LevelId::E1M4 | LevelId::E1M5
            | LevelId::E1M6 | LevelId::E1M7 | LevelId::E1M8 | LevelId::E1M9 | LevelId::E1M10 => 1,
            
            LevelId::E2M1 | LevelId::E2M2 | LevelId::E2M3 | LevelId::E2M4 | LevelId::E2M5
            | LevelId::E2M6 | LevelId::E2M7 | LevelId::E2M8 | LevelId::E2M9 | LevelId::E2M10 => 2,
            
            LevelId::E3M1 | LevelId::E3M2 | LevelId::E3M3 | LevelId::E3M4 | LevelId::E3M5
            | LevelId::E3M6 | LevelId::E3M7 | LevelId::E3M8 | LevelId::E3M9 | LevelId::E3M10 => 3,
            
            LevelId::E4M1 | LevelId::E4M2 | LevelId::E4M3 | LevelId::E4M4 | LevelId::E4M5
            | LevelId::E4M6 | LevelId::E4M7 | LevelId::E4M8 | LevelId::E4M9 | LevelId::E4M10 => 4,
            
            LevelId::E5M1 | LevelId::E5M2 | LevelId::E5M3 | LevelId::E5M4 | LevelId::E5M5
            | LevelId::E5M6 | LevelId::E5M7 | LevelId::E5M8 | LevelId::E5M9 | LevelId::E5M10 => 5,
            
            LevelId::E6M1 | LevelId::E6M2 | LevelId::E6M3 | LevelId::E6M4 | LevelId::E6M5
            | LevelId::E6M6 | LevelId::E6M7 | LevelId::E6M8 | LevelId::E6M9 | LevelId::E6M10 => 6,
        }
    }

    pub const fn floor_number(self) -> i32 {
        match self {
            LevelId::E1M1 | LevelId::E2M1 | LevelId::E3M1 | LevelId::E4M1 | LevelId::E5M1 | LevelId::E6M1 => 1,
            LevelId::E1M2 | LevelId::E2M2 | LevelId::E3M2 | LevelId::E4M2 | LevelId::E5M2 | LevelId::E6M2 => 2,
            LevelId::E1M3 | LevelId::E2M3 | LevelId::E3M3 | LevelId::E4M3 | LevelId::E5M3 | LevelId::E6M3 => 3,
            LevelId::E1M4 | LevelId::E2M4 | LevelId::E3M4 | LevelId::E4M4 | LevelId::E5M4 | LevelId::E6M4 => 4,
            LevelId::E1M5 | LevelId::E2M5 | LevelId::E3M5 | LevelId::E4M5 | LevelId::E5M5 | LevelId::E6M5 => 5,
            LevelId::E1M6 | LevelId::E2M6 | LevelId::E3M6 | LevelId::E4M6 | LevelId::E5M6 | LevelId::E6M6 => 6,
            LevelId::E1M7 | LevelId::E2M7 | LevelId::E3M7 | LevelId::E4M7 | LevelId::E5M7 | LevelId::E6M7 => 7,
            LevelId::E1M8 | LevelId::E2M8 | LevelId::E3M8 | LevelId::E4M8 | LevelId::E5M8 | LevelId::E6M8 => 8,
            LevelId::E1M9 | LevelId::E2M9 | LevelId::E3M9 | LevelId::E4M9 | LevelId::E5M9 | LevelId::E6M9 => 9,
            LevelId::E1M10 | LevelId::E2M10 | LevelId::E3M10 | LevelId::E4M10 | LevelId::E5M10 | LevelId::E6M10 => 10,
        }
    }

    pub const fn first_level_of_episode(episode: u8) -> Self {
        match episode {
            1 => LevelId::E1M1,
            2 => LevelId::E2M1,
            3 => LevelId::E3M1,
            4 => LevelId::E4M1,
            5 => LevelId::E5M1,
            6 => LevelId::E6M1,
            _ => LevelId::E1M1,
        }
    }

    /// Episode progression (normal exits, not secret)
    pub const fn next_normal(self) -> Self {
        match self {
            // Episode 1 (has secret level E1M10)
            LevelId::E1M1 => LevelId::E1M2,
            LevelId::E1M2 => LevelId::E1M3,
            LevelId::E1M3 => LevelId::E1M4,
            LevelId::E1M4 => LevelId::E1M5,
            LevelId::E1M5 => LevelId::E1M6,
            LevelId::E1M6 => LevelId::E1M7,
            LevelId::E1M7 => LevelId::E1M8,
            LevelId::E1M8 => LevelId::E1M9,
            LevelId::E1M9 => LevelId::E1M1,
            LevelId::E1M10 => LevelId::E1M2,
            
            // Episode 2
            LevelId::E2M1 => LevelId::E2M2,
            LevelId::E2M2 => LevelId::E2M3,
            LevelId::E2M3 => LevelId::E2M4,
            LevelId::E2M4 => LevelId::E2M5,
            LevelId::E2M5 => LevelId::E2M6,
            LevelId::E2M6 => LevelId::E2M7,
            LevelId::E2M7 => LevelId::E2M8,
            LevelId::E2M8 => LevelId::E2M9,
            LevelId::E2M9 => LevelId::E2M1,
            LevelId::E2M10 => LevelId::E2M2,
            
            // Episode 3
            LevelId::E3M1 => LevelId::E3M2,
            LevelId::E3M2 => LevelId::E3M3,
            LevelId::E3M3 => LevelId::E3M4,
            LevelId::E3M4 => LevelId::E3M5,
            LevelId::E3M5 => LevelId::E3M6,
            LevelId::E3M6 => LevelId::E3M7,
            LevelId::E3M7 => LevelId::E3M8,
            LevelId::E3M8 => LevelId::E3M9,
            LevelId::E3M9 => LevelId::E3M1,
            LevelId::E3M10 => LevelId::E3M8,
            
            // Episode 4
            LevelId::E4M1 => LevelId::E4M2,
            LevelId::E4M2 => LevelId::E4M3,
            LevelId::E4M3 => LevelId::E4M4,
            LevelId::E4M4 => LevelId::E4M5,
            LevelId::E4M5 => LevelId::E4M6,
            LevelId::E4M6 => LevelId::E4M7,
            LevelId::E4M7 => LevelId::E4M8,
            LevelId::E4M8 => LevelId::E4M9,
            LevelId::E4M9 => LevelId::E4M1,
            LevelId::E4M10 => LevelId::E4M4,
            
            // Episode 5
            LevelId::E5M1 => LevelId::E5M2,
            LevelId::E5M2 => LevelId::E5M3,
            LevelId::E5M3 => LevelId::E5M4,
            LevelId::E5M4 => LevelId::E5M5,
            LevelId::E5M5 => LevelId::E5M6,
            LevelId::E5M6 => LevelId::E5M7,
            LevelId::E5M7 => LevelId::E5M8,
            LevelId::E5M8 => LevelId::E5M9,
            LevelId::E5M9 => LevelId::E5M1,
            LevelId::E5M10 => LevelId::E5M6,
            
            // Episode 6
            LevelId::E6M1 => LevelId::E6M2,
            LevelId::E6M2 => LevelId::E6M3,
            LevelId::E6M3 => LevelId::E6M4,
            LevelId::E6M4 => LevelId::E6M5,
            LevelId::E6M5 => LevelId::E6M6,
            LevelId::E6M6 => LevelId::E6M7,
            LevelId::E6M7 => LevelId::E6M8,
            LevelId::E6M8 => LevelId::E6M9,
            LevelId::E6M9 => LevelId::E6M1,
            LevelId::E6M10 => LevelId::E6M4,
        }
    }

    // Keep old function name for compatibility
    pub const fn next_e1_normal(self) -> Self {
        self.next_normal()
    }
}

pub const fn next_secret(from: LevelId) -> LevelId {
    match from {
        LevelId::E1M1 => LevelId::E1M10,
        LevelId::E2M1 => LevelId::E2M10,
        LevelId::E3M7 => LevelId::E3M10,
        LevelId::E4M3 => LevelId::E4M10,
        LevelId::E5M5 => LevelId::E5M10,
        LevelId::E6M3 => LevelId::E6M10,
        _ => from.next_normal(),
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct CurrentLevel(pub LevelId);

impl Default for CurrentLevel {
    fn default() -> Self {
        Self(LevelId::E1M1)
    }
}

/// Wolf plane1 for Currently Loaded Level
/// Single Source of Truth for Decorations / Pickups Later
#[derive(Resource, Debug, Clone, Default)]
pub struct WolfPlane1(pub Vec<u16>);
