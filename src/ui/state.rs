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

#[derive(Resource, Debug, Clone)]
pub struct DamageFlash {
    pub timer: Timer,
}

impl Default for DamageFlash {
    fn default() -> Self {
        // Wolf-ish quick flash
        let mut t = Timer::from_seconds(0.22, TimerMode::Once);
        // Start "finished" so we don't show anything until triggered
        t.set_elapsed(t.duration());
        Self { timer: t }
    }
}

impl DamageFlash {
    pub fn trigger(&mut self) {
        self.timer.reset();
    }

    pub fn alpha(&self) -> f32 {
        if self.timer.is_finished() {
            return 0.0;
        }
        let dur = self.timer.duration().as_secs_f32().max(0.0001);
        let t = (self.timer.elapsed_secs() / dur).clamp(0.0, 1.0);
        // Ease-out
        let a = (1.0 - t).powf(2.2);
        (a * 0.65).clamp(0.0, 0.65)
    }
}
