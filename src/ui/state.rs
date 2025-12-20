use bevy::prelude::*;
use crate::combat::WeaponSlot;

#[derive(Resource, Debug, Clone)]
pub struct HudState {
    pub hp: i32,
    pub ammo: i32,

    // Weapon system (Wolf 1–4)
    pub selected: WeaponSlot,
    pub owned_mask: u8, // bits for weapons you own
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
            ammo: 25,
            selected: WeaponSlot::Pistol,
            owned_mask: 0,
        };

        // Start with Knife + Pistol (Wolf-style)
        s.grant(WeaponSlot::Knife);
        s.grant(WeaponSlot::Pistol);
        s
    }
}
