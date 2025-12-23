/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub cur: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { cur: max, max }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct OccupiesTile(pub IVec2);

#[derive(Component)]
pub struct Dead;
