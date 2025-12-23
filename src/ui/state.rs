/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

use crate::combat::WeaponSlot;

#[derive(Resource, Debug, Clone)]
pub struct HudState {
    pub hp: i32,
    pub ammo: i32,
    pub score: i32,
    pub lives: i32,

    // Weapon System (1–4)
    pub selected: WeaponSlot,
    // Bits For Owned Weapons
    pub owned_mask: u8,
}

impl HudState {
    #[inline]
    pub fn owns(&self, w: WeaponSlot) -> bool {
        let bit = 1u8 << (w as u8);
        (self.owned_mask & bit) != 0
    }

    #[inline]
    pub fn grant(&mut self, w: WeaponSlot) {
        let bit = 1u8 << (w as u8);
        self.owned_mask |= bit;
    }
}

impl Default for HudState {
    fn default() -> Self {
        let mut s = Self {
            hp: 100,
            ammo: 8,
            score: 0,
            lives: 3,
            selected: WeaponSlot::Pistol,
            owned_mask: 0,
        };

        // Start with Knife + Pistol
        s.grant(WeaponSlot::Knife);
        s.grant(WeaponSlot::Pistol);
        s
    }
}
