/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelId {
    E1M1,
    E1M2,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct CurrentLevel(pub LevelId);

impl Default for CurrentLevel {
    fn default() -> Self {
        Self(LevelId::E1M1)
    }
}

/// Wolf plane1 for the *currently loaded* level.
/// This becomes the single source of truth for decorations/pickups later.
#[derive(Resource, Debug, Clone, Default)]
pub struct WolfPlane1(pub Vec<u16>);
