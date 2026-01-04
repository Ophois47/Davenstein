/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelId {
    E1M1,
    E1M2,
    E1M3,
    E1M4,
    E1M5,
    E1M6,
    E1M7,
    E1M8,
    E1M9,
    E1M10,
}

impl LevelId {
    pub const fn floor_number(self) -> i32 {
        match self {
            LevelId::E1M1 => 1,
            LevelId::E1M2 => 2,
            LevelId::E1M3 => 3,
            LevelId::E1M4 => 4,
            LevelId::E1M5 => 5,
            LevelId::E1M6 => 6,
            LevelId::E1M7 => 7,
            LevelId::E1M8 => 8,
            LevelId::E1M9 => 9,
            LevelId::E1M10 => 10,
        }
    }

    /// Temporary Episode 1 progression table
    /// This advances sequentially and wraps back to E1M1 after E1M10
    /// Secret exits can be added later once we decide how to identify them in your map data
    pub const fn next_e1_normal(self) -> Self {
        match self {
            LevelId::E1M1 => LevelId::E1M2,
            LevelId::E1M2 => LevelId::E1M3,
            LevelId::E1M3 => LevelId::E1M4,
            LevelId::E1M4 => LevelId::E1M5,
            LevelId::E1M5 => LevelId::E1M6,
            LevelId::E1M6 => LevelId::E1M7,
            LevelId::E1M7 => LevelId::E1M8,
            LevelId::E1M8 => LevelId::E1M9,
            LevelId::E1M9 => LevelId::E1M10,
            LevelId::E1M10 => LevelId::E1M1,
        }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct CurrentLevel(pub LevelId);

impl Default for CurrentLevel {
    fn default() -> Self {
        Self(LevelId::E1M1)
    }
}

/// Wolf plane1 for the currently loaded level
/// This becomes the single source of truth for decorations / pickups later
#[derive(Resource, Debug, Clone, Default)]
pub struct WolfPlane1(pub Vec<u16>);
