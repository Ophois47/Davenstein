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
    // Key Icons
    pub key_gold: bool,
    pub key_silver: bool,

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

            key_gold: false,
            key_silver: false,
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
        // Red Player Damage Flash
        let mut t = Timer::from_seconds(0.22, TimerMode::Once);
        // Start "Finished", Don't Show Anything Until Triggered
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
        // Ease Out
        let a = (1.0 - t).powf(2.2);
        (a * 0.65).clamp(0.0, 0.65)
    }
}


#[derive(Resource, Debug, Clone)]
pub struct PickupFlash {
    pub timer: Timer,
    pub color: Srgba,
}

impl Default for PickupFlash {
    fn default() -> Self {
        // Slightly faster than Wolf: 3 steps × 5 tics (instead of 6) at 70Hz.
        const TIC_HZ: f32 = 70.0;
        const NUM_STEPS: f32 = 3.0;
        const STEP_TICS: f32 = 3.0;

        let mut t = Timer::from_seconds((NUM_STEPS * STEP_TICS) / TIC_HZ, TimerMode::Once);
        t.set_elapsed(t.duration()); // start invisible until triggered

        // Straw-yellow target (64,62,0) mapped to 0..1.
        let color = Srgba::new(1.0, 62.0 / 64.0, 0.0, 1.0);

        Self { timer: t, color }
    }
}

impl PickupFlash {
    pub fn trigger(&mut self, _color: Srgba) {
        // Universal bonus flash (same for all pickups), only when consumed
        self.color = Srgba::new(1.0, 62.0 / 64.0, 0.0, 1.0);
        self.timer.reset();
    }

    pub fn alpha(&self) -> f32 {
        // 3 stepped levels, no fade-in. Only steps down then off
        const TIC_HZ: f32 = 70.0;
        const STEP_TICS: f32 = 3.0; // must match Default() above
        const STEP_SECS: f32 = STEP_TICS / TIC_HZ;

        // Strengths: 3/20, 2/20, 1/20 (subtle by design; palette-shift equivalent)
        const WHITESTEPS: f32 = 20.0;
        const A3: f32 = 3.0 / WHITESTEPS; // 0.15
        const A2: f32 = 2.0 / WHITESTEPS; // 0.10
        const A1: f32 = 1.0 / WHITESTEPS; // 0.05

        if self.timer.is_finished() {
            return 0.0;
        }

        let e = self.timer.elapsed_secs();
        if e < STEP_SECS {
            A3
        } else if e < STEP_SECS * 2.0 {
            A2
        } else if e < STEP_SECS * 3.0 {
            A1
        } else {
            0.0
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct DeathOverlay {
    pub active: bool,
    pub timer: Timer,
    /// Drives the Red-to-Black Fade Used on Game Over. Wolf3D's 'ex_died' Case
    /// Calls 'VW_FadeOut' Once the Last Life Is Gone, Taking the Solid Red Fizzle
    /// Screen Down to Black Before the High Scores and Main Menu. Held Separate
    /// From 'timer' so the Fade Cannot Rewind the Completed Dissolve
    pub black_fade: Timer,
}

impl Default for DeathOverlay {
    fn default() -> Self {
        // Wolf3D's Death Fizzle Ran 70 Frames at the Game's 70Hz Tic Rate, so
        // Roughly One Second. The Old 0.28s Was Tuned for an Alpha Ramp; the
        // Pixel Dissolve Needs the Full Second to Read as Scattering Pixels
        // Rather Than a Flash. Still Inside 'DeathDelay' (1.25s) Before Restart
        let mut t = Timer::from_seconds(1.0, TimerMode::Once);
        t.set_elapsed(t.duration());

        // Starts at Zero Elapsed (Not Faded). Only Ticks While Game Over Is Set
        let black_fade = Timer::from_seconds(0.35, TimerMode::Once);

        Self { active: false, timer: t, black_fade }
    }
}

impl DeathOverlay {
    const MAX_ALPHA: f32 = 0.80;

    /// Dissolve Resolution, in Pixels. The Original Fizzle Fade Ran Over the
    /// 320x200 VGA View, so Matching That Cell Count Reproduces the Chunky Red
    /// Pixels When the Canvas Is Upscaled to the Window
    pub const FIZZLE_W: u32 = 320;
    pub const FIZZLE_H: u32 = 200;

    pub fn alpha(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.timer.is_finished() {
            return Self::MAX_ALPHA;
        }
        let dur = self.timer.duration().as_secs_f32().max(0.0001);
        let t = (self.timer.elapsed_secs() / dur).clamp(0.0, 1.0);
        (t.powf(2.2) * Self::MAX_ALPHA).clamp(0.0, Self::MAX_ALPHA)
    }

    /// Fraction of the Death Screen That Should Be Filled With Solid Red, 0..=1.
    ///
    /// Wolf3D's 'Died' Fills the View With Palette Colour 4 and Then Reveals It
    /// One Pixel at a Time in Pseudorandom Order via 'FizzleFade', so Every Pixel
    /// Is Either Untouched or Fully Opaque - the Screen Is Never a Translucent
    /// Wash. This Drives That Reveal Instead of an Alpha Ramp. The Ramp Is Linear
    /// Because the Original Plotted a Fixed 64000/frames Pixels per Frame
    pub fn fizzle_progress(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.timer.is_finished() {
            return 1.0;
        }
        let dur = self.timer.duration().as_secs_f32().max(0.0001);
        (self.timer.elapsed_secs() / dur).clamp(0.0, 1.0)
    }

    /// Fraction of the Way From Red to Black, 0..=1. Only Advances While Game Over
    /// Is Set; 0 Means the Plain Red Death Screen
    pub fn black_progress(&self) -> f32 {
        let dur = self.black_fade.duration().as_secs_f32().max(0.0001);
        (self.black_fade.elapsed_secs() / dur).clamp(0.0, 1.0)
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct GameOver(pub bool);
