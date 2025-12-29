/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::HudState;
use davelib::audio::{PlaySfx, SfxKind};
use davelib::player::Player;

#[derive(Component)]
pub(super) struct DamageFlashOverlay;

#[derive(Component)]
pub(super) struct ViewModelImage;

#[derive(Resource, Clone)]
pub(crate) struct ViewModelSprites {
    pub knife: [Handle<Image>; 5],
    pub pistol: [Handle<Image>; 5],
    pub machinegun: [Handle<Image>; 5],
    pub chaingun: [Handle<Image>; 5],
}

#[derive(Component)]
pub(super) struct HudHpDigit(pub usize); // 0=hundreds, 1=tens, 2=ones

#[derive(Component)]
pub(super) struct HudAmmoDigit(pub usize); // 0=hundreds, 1=tens, 2=ones

#[derive(Component)]
pub(super) struct HudScoreDigit(pub usize); // 0..5 (six digits)

#[derive(Component)]
pub(super) struct HudLivesDigit(pub usize); // 0..1 (two digits)

fn split_score_6_blanks(n: i32) -> [Option<usize>; 6] {
    let mut n = n.max(0) as u32;
    if n > 999_999 {
        n = 999_999;
    }

    // First compute fixed-width digits (with zeros)
    let mut raw = [0usize; 6];
    for i in 0..6 {
        let idx = 5 - i;
        raw[idx] = (n % 10) as usize;
        n /= 10;
    }

    // Then convert leading zeros to blanks, but always show at least one digit.
    let mut out: [Option<usize>; 6] = [None; 6];
    let mut started = false;

    for i in 0..6 {
        if raw[i] != 0 || i == 5 {
            started = true;
        }
        if started {
            out[i] = Some(raw[i]);
        }
    }

    out
}

// Right-aligned with leading blanks (good for lives, ammo/hp style)
fn split_right_aligned_blanks(n: i32, width: usize) -> Vec<Option<usize>> {
    let mut n = n.max(0) as u32;
    let max = 10u32.saturating_pow(width as u32).saturating_sub(1);
    if n > max {
        n = max;
    }

    let mut out = vec![None; width];
    for idx in (0..width).rev() {
        out[idx] = Some((n % 10) as usize);
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out
}

#[derive(Resource, Clone)]
pub(crate) struct HudDigitSprites {
    pub digits: [Handle<Image>; 10],
    pub blank: Handle<Image>,
}

fn split_3_right_aligned(n: i32) -> [Option<usize>; 3] {
    let n = n.clamp(0, 999) as u32;
    let h = (n / 100) as usize;
    let t = ((n / 10) % 10) as usize;
    let o = (n % 10) as usize;

    // Right-aligned with blanks (Wolf-like)
    let hundreds = if n >= 100 { Some(h) } else { None };
    let tens = if n >= 10 { Some(t) } else { None };
    let ones = Some(o);

    [hundreds, tens, ones]
}

impl ViewModelSprites {
    pub fn idle(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            Knife => self.knife[0].clone(),
            Pistol => self.pistol[0].clone(),
            MachineGun => self.machinegun[0].clone(),
            Chaingun => self.chaingun[0].clone(),
        }
    }

    pub fn fire_simple(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            Pistol => self.pistol[2].clone(),
            Knife => self.knife[2].clone(),
            _ => self.idle(w),
        }
    }

    pub fn pistol_frame(&self, idx: usize) -> Handle<Image> {
        self.pistol[idx.min(4)].clone()
    }

    #[allow(dead_code)]
    pub fn knife_frame(&self, idx: usize) -> Handle<Image> {
        self.knife[idx.min(4)].clone()
    }

    // Direct Indexing
    #[allow(dead_code)]
    pub fn fire_frame(&self, w: crate::combat::WeaponSlot, idx: usize) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            MachineGun => self.machinegun[idx].clone(),
            Chaingun => self.chaingun[idx].clone(),
            _ => self.fire_simple(w),
        }
    }

    // Full-Auto Animation Frame Selection
    // Cycle is Counter (0, 1, 2, 3, ...) NOT Direct Sprite Index
    pub fn auto_fire(&self, w: crate::combat::WeaponSlot, cycle: usize) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;

        match w {
            // Machinegun: Bring up / Forward (1) <-> Flash (2)
            MachineGun => {
                // Bring up / Forward -> Flash -> Recover / Back
                // Choose "Back" Frame as 3 OR 4 Depending on Which Looks Like Recoil Recovery
                const SEQ: [usize; 3] = [1, 2, 3];
                self.machinegun[SEQ[cycle % SEQ.len()]].clone()
            }

            // Chaingun: Forward (1), Flash A (2), Forward (1), Flash B (3)
            Chaingun => {
                const SEQ: [usize; 4] = [1, 2, 1, 3];
                self.chaingun[SEQ[cycle % SEQ.len()]].clone()
            }

            _ => self.fire_simple(w),
        }
    }
}

#[derive(Resource)]
pub(crate) struct WeaponState {
    pub cooldown: Timer,
    pub flash: Timer,
    pub showing_fire: bool,
    pub fire_cycle: usize,
}

impl Default for WeaponState {
    fn default() -> Self {
        const TIC: f32 = 1.0 / 70.0;
        const PISTOL_COOLDOWN_TICS: f32 = 20.0;
        const PISTOL_FLASH_TICS: f32 = 12.0;

        let cooldown_secs = PISTOL_COOLDOWN_TICS * TIC;
        let flash_secs = PISTOL_FLASH_TICS * TIC;

        let mut cooldown = Timer::from_seconds(cooldown_secs, TimerMode::Once);
        cooldown.set_elapsed(std::time::Duration::from_secs_f32(cooldown_secs));

        Self {
            cooldown,
            flash: Timer::from_seconds(flash_secs, TimerMode::Once),
            showing_fire: false,
            fire_cycle: 0,
        }
    }
}

pub(crate) fn weapon_fire_and_viewmodel(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Single<&CursorOptions>,
    sprites: Option<Res<ViewModelSprites>>,
    mut weapon: ResMut<WeaponState>,
    mut hud: ResMut<HudState>,
    mut vm_q: Query<&mut ImageNode, With<ViewModelImage>>,
    q_player: Query<&Transform, With<Player>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut fire_ev: MessageWriter<crate::combat::FireShot>,
    mut armed: Local<bool>,
    mut fire_anim_accum: Local<f32>,
    mut last_weapon: Local<Option<crate::combat::WeaponSlot>>,
    mut auto_linger: Local<f32>,
) {
    use crate::combat::WeaponSlot;

    let Some(sprites) = sprites else { return; };

    let dt = time.delta();
    let dt_secs = dt.as_secs_f32();

    // Only Allow Weapon Selection / Firing While Mouse is Locked
    let locked = cursor.grab_mode == CursorGrabMode::Locked;
    if !locked {
        *armed = false;
        *fire_anim_accum = 0.0;
        *last_weapon = Some(hud.selected);

        // Hard Snap Viewmodel to Idle if Unlocked
        weapon.fire_cycle = 0;
        weapon.showing_fire = false;
        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.idle(hud.selected);
        }
        return;
    }

    // Prevent Very First Click (Used to Grab Cursor) From Also Firing
    if !*armed {
        *armed = true;
        *fire_anim_accum = 0.0;
        *last_weapon = Some(hud.selected);
        return;
    }

    // Weapon Selection (1–4)
    for code in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4] {
        if keys.just_pressed(code) {
            if let Some(slot) = WeaponSlot::from_digit_key(code) {
                if hud.owns(slot) {
                    hud.selected = slot;
                    weapon.showing_fire = false;
                    weapon.fire_cycle = 0;
                    weapon.flash.reset();
                    let dur = weapon.cooldown.duration();
                    weapon.cooldown.set_elapsed(dur);
                    *fire_anim_accum = 0.0;
                    *last_weapon = Some(hud.selected);
                    *auto_linger = 0.0;
                    if let Ok(mut img) = vm_q.single_mut() {
                        img.image = sprites.idle(hud.selected);
                    }
                }
            }
        }
    }

    // If Weapon Changed Externally Somehow, Reset Anim Accumulator
    if last_weapon.map(|w| w != hud.selected).unwrap_or(true) {
        *fire_anim_accum = 0.0;
        weapon.fire_cycle = 0;
        weapon.showing_fire = false;
        *last_weapon = Some(hud.selected);
        *auto_linger = 0.0;
        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.idle(hud.selected);
        }
    }

    // Per-Weapon Paramaters
    const TIC: f32 = 1.0 / 70.0;
    let (cooldown_secs, flash_secs, ammo_cost, max_dist) = match hud.selected {
        WeaponSlot::Knife => (10.0 * TIC, 12.0 * TIC, 0, 1.5),
        WeaponSlot::Pistol => (25.0 * TIC, 16.0 * TIC, 1, 64.0),
        WeaponSlot::MachineGun => (12.0 * TIC, 8.0 * TIC, 1, 64.0),
        WeaponSlot::Chaingun => (6.0 * TIC, 8.0 * TIC, 1, 64.0),
    };

    // Ensure Timers Match Current Weapon
    if (weapon.cooldown.duration().as_secs_f32() - cooldown_secs).abs() > f32::EPSILON {
        weapon.cooldown = Timer::from_seconds(cooldown_secs, TimerMode::Once);
        weapon.cooldown.set_elapsed(std::time::Duration::from_secs_f32(cooldown_secs));
    }
    if (weapon.flash.duration().as_secs_f32() - flash_secs).abs() > f32::EPSILON {
        weapon.flash = Timer::from_seconds(flash_secs, TimerMode::Once);
    }

    // Weapon Kind Flags (MG Handled Differently)
    let is_machinegun = hud.selected == WeaponSlot::MachineGun;
    let is_chaingun = hud.selected == WeaponSlot::Chaingun;
    let is_full_auto = is_machinegun || is_chaingun;

    let trigger_down = mouse.pressed(MouseButton::Left);
    let trigger_pressed = mouse.just_pressed(MouseButton::Left);

    // Tick Cooldown
    weapon.cooldown.tick(dt);

    // Ammo Check
    let mut has_ammo = ammo_cost == 0 || hud.ammo >= ammo_cost;

    // Flash Timer Handling
    // Knife + Pistol Keep Existing Behavior
    // MachineGun ALSO Uses Flash Timer, but Chaingun Does Not, Has Own Cycling
    if weapon.showing_fire && (!is_chaingun) {
        weapon.flash.tick(dt);

        // PISTOL: Advance Through 4-Frame Sequence Across Flash Timer
        if hud.selected == WeaponSlot::Pistol {
            let dur = weapon.flash.duration().as_secs_f32().max(0.0001);
            let t = (weapon.flash.elapsed_secs() / dur).clamp(0.0, 1.0);

            let frame = if t < 0.25 {
                1 // Raise
            } else if t < 0.50 {
                2 // Muzzle Flash
            } else if t < 0.75 {
                3 // Recover
            } else {
                4 // Settle
            };

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.pistol_frame(frame);
            }
        }

        // KNIFE: 3-Frame Swing Over the Flash Timer (Wind-Up -> Hit -> Recover)
        if hud.selected == WeaponSlot::Knife {
            let dur = weapon.flash.duration().as_secs_f32().max(0.0001);
            let t = (weapon.flash.elapsed_secs() / dur).clamp(0.0, 1.0);

            let frame = if t < 0.33 {
                1 // Wind-Up
            } else if t < 0.66 {
                2 // Hit
            } else {
                3 // Recover
            };

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.knife[frame].clone();
            }
        }

        if weapon.flash.is_finished() {
            weapon.showing_fire = false;

            if let Ok(mut img) = vm_q.single_mut() {
                if hud.selected == WeaponSlot::Pistol {
                    img.image = sprites.pistol_frame(0); // Idle
                } else if is_machinegun && trigger_down && has_ammo {
                    img.image = sprites.fire_frame(WeaponSlot::MachineGun, 1); // Forward
                } else {
                    img.image = sprites.idle(hud.selected);
                }
            }
        }
    }

    // CHAINGUN: Cycling Only 
    let firing_anim_tic_secs = 12.0 * TIC;

    if is_chaingun && trigger_down && has_ammo {
        *auto_linger = 0.0;

        if weapon.fire_cycle == 0 {
            weapon.showing_fire = true;
            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
            }
        }

        *fire_anim_accum += dt_secs;

        if *fire_anim_accum >= firing_anim_tic_secs {
            *fire_anim_accum -= firing_anim_tic_secs;

            weapon.fire_cycle = weapon.fire_cycle.wrapping_add(1);
            weapon.showing_fire = true;

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
            }
        }
    } else {
        *fire_anim_accum = 0.0;

        if is_chaingun && (!trigger_down || !has_ammo) {
            if *auto_linger > 0.0 {
                *auto_linger = (*auto_linger - dt_secs).max(0.0);

                if weapon.fire_cycle != 0 {
                    weapon.showing_fire = true;
                    if let Ok(mut img) = vm_q.single_mut() {
                        img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
                    }
                }
            } else {
                weapon.fire_cycle = 0;
                weapon.showing_fire = false;
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.idle(hud.selected);
                }
            }
        } else {
            *auto_linger = 0.0;
        }
    }

    // MACHINEGUN: While Holding (and Not Flashing), Keep Forward Pose
    if is_machinegun && trigger_down && has_ammo && !weapon.showing_fire {
        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.fire_frame(WeaponSlot::MachineGun, 1); // Forward
        }
    }

    // MACHINEGUN: ALWAYS Snap Back to Idle When Trigger Not Held
    // Prevents rare "stuck forward" posture after releasing the mouse
    if is_machinegun && !trigger_down {
        weapon.showing_fire = false;
        weapon.fire_cycle = 0;
        *auto_linger = 0.0;
        weapon.flash.reset();

        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.idle(WeaponSlot::MachineGun);
        }
    }

    // Fire Intent
    let wants_fire = if is_full_auto {
        trigger_down // HOLD to fire
    } else {
        trigger_pressed // Knife + Pistol click-to-fire
    };

    // Prevent ROF wobble: allow small catch-up under frame jitter
    let max_shots_per_frame = match hud.selected {
        WeaponSlot::Chaingun => 1,
        _ => 3,
    };
    let mut shots_fired_this_frame = 0usize;

    while wants_fire
        && weapon.cooldown.is_finished()
        && has_ammo
        && shots_fired_this_frame < max_shots_per_frame
    {
        shots_fired_this_frame += 1;

        // Spend ammo (knife is 0 cost)
        if ammo_cost > 0 {
            hud.ammo = hud.ammo.saturating_sub(ammo_cost);
        }

        weapon.cooldown.reset();
        weapon.flash.reset();

        // --- MachineGun: show muzzle flash EXACTLY on the shot moment (syncs with sound) ---
        if is_machinegun {
            weapon.showing_fire = true;
            weapon.flash.reset(); // flash timer starts NOW

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.fire_frame(WeaponSlot::MachineGun, 2); // muzzle flash
            }

            // optional: tiny linger makes taps readable (doesn't affect holding)
            *auto_linger = 0.10;
        }


        // --- Chaingun: keep your existing behavior (cycle advance on shot is OK for you) ---
        if is_chaingun {
            weapon.showing_fire = true;

            weapon.fire_cycle = weapon.fire_cycle.wrapping_add(1);
            *fire_anim_accum = 0.0;
            *auto_linger = 0.10;

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
            }
        }

        // For non-full-auto, show the simple fire sprite immediately
        // For non-full-auto, start the attack animation immediately
        if !is_full_auto {
            weapon.showing_fire = true;
            weapon.flash.reset();

            if hud.selected == WeaponSlot::Pistol {
                // Start pistol anim on "raise" frame, not flash
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.pistol_frame(1);
                }
            } else if hud.selected == WeaponSlot::Knife {
                // Start knife on wind-up (3-step swing)
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.knife[1].clone(); // wind-up
                }
            } else {
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.fire_simple(hud.selected);
                }
            }
        }

        // Emit SFX + FireShot (synced to each bullet)
        if let Ok(tf) = q_player.single() {
            let origin = tf.translation;
            let dir = (tf.rotation * Vec3::NEG_Z).normalize();
            let sfx_pos = Vec3::new(origin.x, 0.6, origin.z);

            match hud.selected {
                WeaponSlot::Knife => {
                    sfx.write(PlaySfx { kind: SfxKind::KnifeSwing, pos: sfx_pos });
                }
                WeaponSlot::Pistol => {
                    sfx.write(PlaySfx { kind: SfxKind::PistolFire, pos: sfx_pos });
                }
                WeaponSlot::MachineGun => {
                    sfx.write(PlaySfx { kind: SfxKind::MachineGunFire, pos: sfx_pos });
                }
                WeaponSlot::Chaingun => {
                    sfx.write(PlaySfx { kind: SfxKind::ChaingunFire, pos: sfx_pos });
                }
            }

            fire_ev.write(crate::combat::FireShot {
                weapon: hud.selected,
                origin,
                dir,
                max_dist,
            });
        }

        has_ammo = ammo_cost == 0 || hud.ammo >= ammo_cost;
    }
}

pub(crate) fn sync_hud_hp_digits(
    hud: Res<HudState>,
    digits: Option<Res<HudDigitSprites>>,
    mut q: Query<(&HudHpDigit, &mut ImageNode)>,
) {
    if !hud.is_changed() {
        return;
    }
    let Some(digits) = digits else { return; };

    let hp_digits = split_3_right_aligned(hud.hp);

    for (slot, mut img) in &mut q {
        let handle = match hp_digits.get(slot.0).copied().flatten() {
            Some(d) => digits.digits[d].clone(),
            None => digits.blank.clone(),
        };
        img.image = handle;
    }
}

pub(crate) fn sync_hud_ammo_digits(
    hud: Res<HudState>,
    digits: Option<Res<HudDigitSprites>>,
    mut q: Query<(&HudAmmoDigit, &mut ImageNode)>,
) {
    if !hud.is_changed() {
        return;
    }
    let Some(digits) = digits else { return; };

    let ammo_digits = split_3_right_aligned(hud.ammo);

    for (slot, mut img) in &mut q {
        let handle = match ammo_digits.get(slot.0).copied().flatten() {
            Some(d) => digits.digits[d].clone(),
            None => digits.blank.clone(),
        };
        img.image = handle;
    }
}

pub(crate) fn sync_hud_score_digits(
    hud: Res<HudState>,
    digits: Option<Res<HudDigitSprites>>,
    mut q: Query<(&HudScoreDigit, &mut ImageNode)>,
) {
    if !hud.is_changed() {
        return;
    }
    let Some(digits) = digits else { return; };

    let score_digits = split_score_6_blanks(hud.score);

    for (slot, mut img) in &mut q {
        let handle = match score_digits.get(slot.0).copied().flatten() {
            Some(d) => digits.digits[d].clone(),
            None => digits.blank.clone(),
        };
        img.image = handle;
    }
}

pub(crate) fn sync_hud_lives_digits(
    hud: Res<HudState>,
    digits: Option<Res<HudDigitSprites>>,
    mut q: Query<(&HudLivesDigit, &mut ImageNode)>,
) {
    if !hud.is_changed() {
        return;
    }
    let Some(digits) = digits else { return; };

    let lives_digits = split_right_aligned_blanks(hud.lives, 2);

    for (slot, mut img) in &mut q {
        let handle = match lives_digits.get(slot.0).copied().flatten() {
            Some(d) => digits.digits[d].clone(),
            None => digits.blank.clone(),
        };
        img.image = handle;
    }
}

pub(crate) fn flash_on_hp_drop(
    hud: Res<HudState>,
    mut flash: ResMut<super::DamageFlash>,
    mut last_hp: Local<Option<i32>>,
) {
    let Some(prev) = *last_hp else {
        *last_hp = Some(hud.hp);
        return;
    };

    if hud.hp < prev {
        flash.trigger();
    }

    *last_hp = Some(hud.hp);
}

pub(crate) fn tick_damage_flash(
    time: Res<Time>,
    mut flash: ResMut<super::DamageFlash>,
    mut q: Query<&mut BackgroundColor, With<DamageFlashOverlay>>,
) {
    flash.timer.tick(time.delta());

    let a = flash.alpha();
    for mut bg in q.iter_mut() {
        *bg = BackgroundColor(Srgba::new(1.0, 0.0, 0.0, a).into());
    }
}

pub(crate) fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    hud: Res<HudState>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    // Viewmodel sprites
    let sprites = ViewModelSprites {
        knife: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/knife_{i}.png"))),
        pistol: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/pistol_{i}.png"))),
        machinegun: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/machinegun_{i}.png"))),
        chaingun: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/chaingun_{i}.png"))),
    };
    commands.insert_resource(sprites.clone());

    // Starting viewmodel based on selected weapon
    let weapon_idle: Handle<Image> = sprites.idle(hud.selected);

    // HUD digit sprites
    let hud_digits = HudDigitSprites {
        digits: std::array::from_fn(|i| asset_server.load(format!("textures/hud/digits/digit_{i}.png"))),
        blank: asset_server.load("textures/hud/digits/digit_blank.png"),
    };
    commands.insert_resource(hud_digits.clone());

    // Boxed HUD strip background (320x44)
    let status_bar: Handle<Image> = asset_server.load("textures/hud/status_bar.png");

    // --- Native Wolf HUD sizing ---
    const HUD_W: f32 = 320.0;

    // IMPORTANT: for now, the HUD height is ONLY the strip height (44px),
    // so there is no meaningless blue area below it.
    const STATUS_H: f32 = 44.0;

    // Digit cell size (native)
    const DIGIT_W: f32 = 8.0;
    const DIGIT_H: f32 = 16.0;
    const DIGIT_TOP: f32 = 18.0;

    // Placement tweaks (native coords)
    const SCORE_X: f32 = 48.0;
    const LIVES_X: f32 = 108.0;
    const HP_X: f32 = 168.0;
    const AMMO_X: f32 = 208.0;

    // Pixel-perfect integer scale from window width
    let win = q_windows.iter().next().expect("PrimaryWindow");
    let win_w = win.resolution.width();
    let hud_scale_i = (win_w / HUD_W).floor().max(1.0) as i32;
    let hud_scale = hud_scale_i as f32;

    // Scaled sizes
    let hud_w_px = HUD_W * hud_scale;
    let status_h_px = STATUS_H * hud_scale;

    let digit_w_px = DIGIT_W * hud_scale;
    let digit_h_px = DIGIT_H * hud_scale;
    let digit_top_px = DIGIT_TOP * hud_scale;

    let score_x_px = SCORE_X * hud_scale;
    let lives_x_px = LIVES_X * hud_scale;
    let hp_x_px = HP_X * hud_scale;
    let ammo_x_px = AMMO_X * hud_scale;

    const GUN_SCALE: f32 = 7.5;
    const GUN_SRC_PX: f32 = 64.0;
    const GUN_PX: f32 = GUN_SRC_PX * GUN_SCALE;

    // Wolf HUD blue (0, 0, 164)
    const BACKGROUND_COLOR: bevy::prelude::Srgba = Srgba::rgb(0.0, 0.0, 164.0 / 255.0);

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|ui| {
            // View area
            ui.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                padding: UiRect::bottom(Val::Px(0.05)),
                ..default()
            })
            .with_children(|vm| {
                vm.spawn((
                    ViewModelImage,
                    ImageNode::new(weapon_idle),
                    Node {
                        width: Val::Px(GUN_PX),
                        height: Val::Px(GUN_PX),
                        ..default()
                    },
                ));

                vm.spawn((
                    DamageFlashOverlay,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    BackgroundColor(Srgba::new(1.0, 0.0, 0.0, 0.0).into()),
                ));
            });

            // Status bar container (NOW only 44px tall, scaled)
            ui.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(status_h_px),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BACKGROUND_COLOR.into()),
            ))
            .with_children(|bar| {
                // Inner HUD canvas (scaled)
                bar.spawn(Node {
                    width: Val::Px(hud_w_px),
                    height: Val::Px(status_h_px),
                    position_type: PositionType::Relative,
                    ..default()
                })
                .with_children(|inner| {
                    // Boxed strip (spawn first so it draws behind digits)
                    inner.spawn((
                        ImageNode::new(status_bar.clone()),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Px(hud_w_px),
                            height: Val::Px(status_h_px),
                            ..default()
                        },
                    ));

                    // SCORE
                    let score_digits = split_score_6_blanks(hud.score);
                    inner
                        .spawn(Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(score_x_px),
                            top: Val::Px(digit_top_px),
                            flex_direction: FlexDirection::Row,
                            ..default()
                        })
                        .with_children(|score| {
                            for (slot, dopt) in score_digits.iter().enumerate() {
                                let handle = match dopt {
                                    Some(d) => hud_digits.digits[*d].clone(),
                                    None => hud_digits.blank.clone(),
                                };
                                score.spawn((
                                    HudScoreDigit(slot),
                                    ImageNode::new(handle),
                                    Node {
                                        width: Val::Px(digit_w_px),
                                        height: Val::Px(digit_h_px),
                                        ..default()
                                    },
                                ));
                            }
                        });

                    // LIVES
                    let lives_digits = split_right_aligned_blanks(hud.lives, 2);
                    inner
                        .spawn(Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(lives_x_px),
                            top: Val::Px(digit_top_px),
                            flex_direction: FlexDirection::Row,
                            ..default()
                        })
                        .with_children(|lives| {
                            for (slot, dopt) in lives_digits.iter().enumerate() {
                                let handle = match dopt {
                                    Some(d) => hud_digits.digits[*d].clone(),
                                    None => hud_digits.blank.clone(),
                                };
                                lives.spawn((
                                    HudLivesDigit(slot),
                                    ImageNode::new(handle),
                                    Node {
                                        width: Val::Px(digit_w_px),
                                        height: Val::Px(digit_h_px),
                                        ..default()
                                    },
                                ));
                            }
                        });

                    // HEALTH
                    let hp_digits = split_3_right_aligned(hud.hp);
                    inner
                        .spawn(Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(hp_x_px),
                            top: Val::Px(digit_top_px),
                            flex_direction: FlexDirection::Row,
                            ..default()
                        })
                        .with_children(|hp| {
                            for (slot, dopt) in hp_digits.iter().enumerate() {
                                let handle = match dopt {
                                    Some(d) => hud_digits.digits[*d].clone(),
                                    None => hud_digits.blank.clone(),
                                };
                                hp.spawn((
                                    HudHpDigit(slot),
                                    ImageNode::new(handle),
                                    Node {
                                        width: Val::Px(digit_w_px),
                                        height: Val::Px(digit_h_px),
                                        ..default()
                                    },
                                ));
                            }
                        });

                    // AMMO
                    let ammo_digits = split_3_right_aligned(hud.ammo);
                    inner
                        .spawn(Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(ammo_x_px),
                            top: Val::Px(digit_top_px),
                            flex_direction: FlexDirection::Row,
                            ..default()
                        })
                        .with_children(|ammo| {
                            for (slot, dopt) in ammo_digits.iter().enumerate() {
                                let handle = match dopt {
                                    Some(d) => hud_digits.digits[*d].clone(),
                                    None => hud_digits.blank.clone(),
                                };
                                ammo.spawn((
                                    HudAmmoDigit(slot),
                                    ImageNode::new(handle),
                                    Node {
                                        width: Val::Px(digit_w_px),
                                        height: Val::Px(digit_h_px),
                                        ..default()
                                    },
                                ));
                            }
                        });
                });
            });
        });
}
