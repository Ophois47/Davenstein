use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct HudState {
    pub hp: i32,
    pub ammo: i32,
}

impl Default for HudState {
    fn default() -> Self {
        Self { hp: 100, ammo: 8 }
    }
}
