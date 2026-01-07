/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::window::{
    CursorGrabMode,
    CursorOptions,
    PrimaryWindow,
};

use super::{
    HudState,
    DeathOverlay,
    GameOver,
};
use davelib::audio::{PlaySfx, SfxKind};
use davelib::player::{
    GodMode,
    Player,
    PlayerControlLock,
};
use davelib::level::CurrentLevel;

#[derive(Component)]
pub(super) struct DamageFlashOverlay;

#[derive(Component)]
pub(super) struct PickupFlashOverlay;

#[derive(Component)]
pub(super) struct DeathOverlayOverlay;

#[derive(Component)]
pub(super) struct GameOverOverlay;

#[derive(Component)]
pub(super) struct ViewModelImage;

#[derive(Resource, Clone)]
pub(crate) struct ViewModelSprites {
    pub knife: [Handle<Image>; 5],
    pub pistol: [Handle<Image>; 5],
    pub machinegun: [Handle<Image>; 5],
    pub chaingun: [Handle<Image>; 5],
}

#[derive(Resource, Clone)]
pub(crate) struct HudIconSprites {
    pub weapon_knife: Handle<Image>,
    pub weapon_pistol: Handle<Image>,
    pub weapon_machinegun: Handle<Image>,
    pub weapon_chaingun: Handle<Image>,
    pub key_gold: Handle<Image>,
    pub key_silver: Handle<Image>,
}

impl HudIconSprites {
    #[inline]
    pub fn weapon(&self, slot: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot;
        match slot {
            WeaponSlot::Knife => self.weapon_knife.clone(),
            WeaponSlot::Pistol => self.weapon_pistol.clone(),
            WeaponSlot::MachineGun => self.weapon_machinegun.clone(),
            WeaponSlot::Chaingun => self.weapon_chaingun.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FaceDir {
    Forward,
    Right,
    Left,
}

#[derive(Resource, Debug, Clone)]
pub(crate) struct HudFaceLook {
    pub dir: FaceDir,
    pub timer: Timer,
    pub tick: u32,
}

impl Default for HudFaceLook {
    fn default() -> Self {
        const FACE_LOOK_PERIOD_SECS: f32 = 1.20;

        Self {
            dir: FaceDir::Forward,
            timer: Timer::from_seconds(
                FACE_LOOK_PERIOD_SECS,
                TimerMode::Repeating,
            ),
            tick: 0,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct HudFacePrevHp(pub i32);

#[derive(Resource, Debug, Clone)]
pub(crate) struct HudFaceOverride {
    pub active: bool,
    pub timer: Timer,
}

impl Default for HudFaceOverride {
    fn default() -> Self {
        // How Long Grin Stays Up
        const GRIN_SECS: f32 = 1.80;

        let mut t = Timer::from_seconds(GRIN_SECS, TimerMode::Once);
        t.set_elapsed(t.duration());
        Self { active: false, timer: t }
    }
}

/// Marker on ImageNode that is BJ's Face in HUD
#[derive(Component)]
pub(crate) struct HudFaceImage;

#[derive(Resource, Clone)]
pub(crate) struct HudFaceSprites {
    pub bands: [[Handle<Image>; 3]; 7],
    pub grin: Handle<Image>,
    pub dead: Handle<Image>,
    stone: Handle<Image>,
}

impl HudFaceSprites {
    pub fn bands(&self, band: usize, dir: usize) -> Handle<Image> {
        self.bands[band.min(6)][dir.min(2)].clone()
    }

    pub fn grin(&self) -> Handle<Image> {
        self.grin.clone()
    }

    pub fn dead(&self) -> Handle<Image> {
        self.dead.clone()
    }

    pub fn stone(&self) -> Handle<Image> {
        self.stone.clone()
    }
}

#[derive(Component)]
pub(super) struct HudWeaponIcon;

#[derive(Component)]
pub(super) struct HudGoldKeyIcon;

#[derive(Component)]
pub(super) struct HudSilverKeyIcon;

#[derive(Component)]
pub(super) struct HudHpDigit(pub usize); // 0 = Hundreds, 1 = Tens, 2 = Ones

#[derive(Component)]
pub(super) struct HudAmmoDigit(pub usize); // 0 = Hundreds, 1 = Tens, 2 = Ones

#[derive(Component)]
pub(super) struct HudScoreDigit(pub usize); // 0..5 (Six Digits)

#[derive(Component)]
pub(super) struct HudLivesDigit(pub usize); // 0..1 (Two Digits)

#[derive(Component)]
pub(super) struct HudFloorDigit(pub usize); // 0..1 (Two Digits)

#[derive(Component)]
pub(crate) struct MissionBjCardImage;

#[derive(Resource, Clone)]
pub(crate) struct MissionBjCardSprites {
    pub cards: [Handle<Image>; 3],
}

pub(crate) struct MissionBjCardAnim {
    timer: Timer,
    idx: usize,
    active: bool,
}

impl Default for MissionBjCardAnim {
    fn default() -> Self {
        const TIC: f32 = 1.0 / 70.0;
        const BJ_FRAME_TICS: f32 = 10.0;

        Self {
            timer: Timer::from_seconds(BJ_FRAME_TICS * TIC, TimerMode::Repeating),
            idx: 0,
            active: false,
        }
    }
}

pub(crate) fn tick_mission_bj_card(
    time: Res<Time>,
    win: Res<crate::level_complete::LevelComplete>,
    sprites: Option<Res<MissionBjCardSprites>>,
    mut q: Query<&mut ImageNode, With<MissionBjCardImage>>,
    mut anim: Local<MissionBjCardAnim>,
) {
    let Some(sprites) = sprites else { return; };
    let Some(mut img) = q.iter_mut().next() else { return; };

    if !win.0 {
        if anim.active {
            anim.active = false;
            anim.idx = 0;
            anim.timer.reset();
            img.image = sprites.cards[0].clone();
        }
        return;
    }

    if !anim.active {
        anim.active = true;
        anim.idx = 0;
        anim.timer.reset();
        img.image = sprites.cards[0].clone();
    }

    anim.timer.tick(time.delta());
    if anim.timer.just_finished() {
        anim.idx = (anim.idx + 1) % 3;
        img.image = sprites.cards[anim.idx].clone();
    }
}

fn split_score_6_blanks(n: i32) -> [Option<usize>; 6] {
    let mut n = n.max(0) as u32;
    if n > 999_999 {
        n = 999_999;
    }

    // First Compute Fixed-Width Digits (With Zeros)
    let mut raw = [0usize; 6];
    for i in 0..6 {
        let idx = 5 - i;
        raw[idx] = (n % 10) as usize;
        n /= 10;
    }

    // Then Convert Leading Zeros to Blanks,
    // Always Show at Least One Digit
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

// Right-Aligned with Leading Blanks (Good for Lives, Ammo / HP Style)
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

fn split_3_right_aligned(n: i32) -> [Option<usize>; 3] {
    let n = n.clamp(0, 999) as u32;
    let h = (n / 100) as usize;
    let t = ((n / 10) % 10) as usize;
    let o = (n % 10) as usize;

    let hundreds = if n >= 100 { Some(h) } else { None };
    let tens = if n >= 10 { Some(t) } else { None };
    let ones = Some(o);

    [hundreds, tens, ones]
}

pub(crate) fn sync_hud_floor_digits(
    level: Res<CurrentLevel>,
    digits: Option<Res<HudDigitSprites>>,
    mut q: Query<(&HudFloorDigit, &mut ImageNode)>,
) {
    if !level.is_changed() {
        return;
    }
    let Some(digits) = digits else { return; };

    let floor_num: i32 = level.0.floor_number();
    let floor_digits = split_right_aligned_blanks(floor_num, 2);

    for (slot, mut img) in &mut q {
        let handle = match floor_digits.get(slot.0).copied().flatten() {
            Some(d) => digits.digits[d].clone(),
            None => digits.blank.clone(),
        };
        img.image = handle;
    }
}

pub(crate) fn sync_viewmodel_size(
    q_win: Query<&Window, With<PrimaryWindow>>,
    mut q_vm: Query<&mut Node, With<ViewModelImage>>,
) {
    let Some(win) = q_win.iter().next() else { return; };
    let Some(mut node) = q_vm.iter_mut().next() else { return; };

    const STATUS_BAR_H: f32 = 64.0;
    const VIEWMODEL_HEIGHT_FRAC: f32 = 0.62;

    let view_h = (win.resolution.height() - STATUS_BAR_H).max(1.0);
    let gun_px = view_h * VIEWMODEL_HEIGHT_FRAC;

    node.width = Val::Px(gun_px);
    node.height = Val::Px(gun_px);
}

pub(crate) fn weapon_fire_and_viewmodel(
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Single<&CursorOptions>,
    lock: Res<PlayerControlLock>,
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

    // Block Selection / Firing While Dead (Input Lock)
    if lock.0 {
        *fire_anim_accum = 0.0;
        *last_weapon = Some(hud.selected);
        *auto_linger = 0.0;

        weapon.fire_cycle = 0;
        weapon.showing_fire = false;
        weapon.flash.reset();

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
    const BULLET_MAX_DIST: f32 = 10_000.0;

    let (cooldown_secs, flash_secs, ammo_cost, max_dist) = match hud.selected {
        WeaponSlot::Knife => (10.0 * TIC, 12.0 * TIC, 0, 1.5),
        WeaponSlot::Pistol => (25.0 * TIC, 16.0 * TIC, 1, BULLET_MAX_DIST),
        WeaponSlot::MachineGun => (12.0 * TIC, 8.0 * TIC, 1, BULLET_MAX_DIST),
        WeaponSlot::Chaingun => (6.0 * TIC, 8.0 * TIC, 1, BULLET_MAX_DIST),
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
            // Forward
            img.image = sprites.fire_frame(WeaponSlot::MachineGun, 1);
        }
    }

    // MACHINEGUN: ALWAYS Snap Back to Idle When Trigger Not Held
    // Prevents "Stuck Forward" Posture After Releasing Button
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
        // Hold to Fire
        trigger_down
    } else {
        // Knife + Pistol Click to Fire
        trigger_pressed
    };

    // Prevent ROF Wobble: Allow Small Catch up Under Frame Jitter
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

        // Spend ammo (Knife is 0)
        if ammo_cost > 0 {
            hud.ammo = hud.ammo.saturating_sub(ammo_cost);
        }

        weapon.cooldown.reset();
        weapon.flash.reset();

        // --- MachineGun: Show Muzzle Flash EXACTLY on the Shot Moment (Syncs With Sound) ---
        if is_machinegun {
            weapon.showing_fire = true;
            weapon.flash.reset(); // Flash Timer Starts Here

            if let Ok(mut img) = vm_q.single_mut() {
                // Muzzle Flash
                img.image = sprites.fire_frame(WeaponSlot::MachineGun, 2);
            }

            *auto_linger = 0.10;
        }

        // --- Chaingun ---
        if is_chaingun {
            weapon.showing_fire = true;

            weapon.fire_cycle = weapon.fire_cycle.wrapping_add(1);
            *fire_anim_accum = 0.0;
            *auto_linger = 0.10;

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
            }
        }

        // For Semi Auto, Start Attack Animation Immediately
        if !is_full_auto {
            weapon.showing_fire = true;
            weapon.flash.reset();

            if hud.selected == WeaponSlot::Pistol {
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.pistol_frame(1);
                }
            } else if hud.selected == WeaponSlot::Knife {
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.knife[1].clone();
                }
            } else if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.fire_simple(hud.selected);
            }
        }

        // Emit SFX + FireShot (Synced to Each Bullet)
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

pub fn tick_hud_face_timers(
    time: Res<Time>,
    hud: Res<HudState>,
    mut prev_hp: ResMut<HudFacePrevHp>,
    mut look: ResMut<HudFaceLook>,
    mut face_ov: ResMut<HudFaceOverride>,
) {
    // Timed grin override (separate from "stone" god-mode face).
    if face_ov.active {
        face_ov.timer.tick(time.delta());
        if face_ov.timer.is_finished() {
            face_ov.active = false;
        }
    }

    let hp = hud.hp as i32;
    let dropped = hp < prev_hp.0;

    if dropped {
        // On damage, cancel any "grin" and reset look to forward.
        face_ov.active = false;
        look.dir = FaceDir::Forward;
        look.timer.reset();
        look.tick = 0;
    }

    prev_hp.0 = hp;

    // Re-drive the classic "look left/right" behavior.
    //
    // Simple rule:
    // - Every FACE_LOOK_PERIOD_SECS, BJ alternates: Forward -> (Right/Left) -> Forward -> ...
    // - Damage resets to Forward immediately (handled above).
    look.timer.tick(time.delta());
    if look.timer.just_finished() {
        look.tick = look.tick.wrapping_add(1);

        look.dir = match look.dir {
            FaceDir::Forward => {
                // Alternate which side we glance toward.
                if (look.tick & 1) == 0 { FaceDir::Right } else { FaceDir::Left }
            }
            FaceDir::Right | FaceDir::Left => FaceDir::Forward,
        };
    }
}


pub fn sync_hud_face(
    faces: Res<HudFaceSprites>,
    hud: Res<HudState>,
    look: Res<HudFaceLook>,
    face_ov: Res<HudFaceOverride>,
    god_mode: Res<GodMode>,
    mut q_face: Query<&mut ImageNode, With<HudFaceImage>>,
) {
    let Ok(mut node) = q_face.single_mut() else {
        return;
    };

    // 1) Timed grin override (e.g. pickups / healing)
    if face_ov.active {
        node.image = faces.grin();
        return;
    }

    // 2) God mode face (stone)
    if god_mode.0 {
        node.image = faces.stone();
        return;
    }

    // 3) Normal face selection (HP band + look direction)
    let hp = hud.hp as i32;

    // coords_for() already returns (Dead) when hp <= 0.
    let (row, col) = coords_for(hp, look.dir);

    node.image = match (row, col) {
        // Special faces on row 1
        (1, 9) => faces.grin(),
        (1, 10) => faces.dead(),
        (1, 11) => faces.stone(),

        // Row 0: stages 0..3, each stage is 3 columns (F/R/L)
        (0, c @ 0..=11) => {
            let band = (c / 3) as usize;      // 0..3
            let dir = (c % 3) as usize;       // 0..2
            faces.bands(band, dir)
        }

        // Row 1: stages 4..6 live in columns 0..8 (3 stages * 3 dirs)
        (1, c @ 0..=8) => {
            let band = 4usize + (c / 3) as usize; // 4..6
            let dir = (c % 3) as usize;           // 0..2
            faces.bands(band, dir)
        }

        _ => faces.bands(0, 0),
    };
}


pub(crate) fn sync_hud_icons(
    hud: Res<HudState>,
    icons: Res<HudIconSprites>,
    mut q_weapon: Query<&mut ImageNode, With<HudWeaponIcon>>,
    mut q_keys: Query<
        (&mut Visibility, Option<&HudGoldKeyIcon>, Option<&HudSilverKeyIcon>),
        Or<(With<HudGoldKeyIcon>, With<HudSilverKeyIcon>)>,
    >,
) {
    // Keys
    for (mut vis, is_gold, is_silver) in &mut q_keys {
        if is_gold.is_some() {
            *vis = if hud.key_gold {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else if is_silver.is_some() {
            *vis = if hud.key_silver {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    // Weapon Icon: Update Only When HUD Changed
    if !hud.is_changed() {
        return;
    }

    if let Some(mut img) = q_weapon.iter_mut().next() {
        img.image = icons.weapon(hud.selected);
    }
}

pub(crate) fn ensure_pickup_flash_overlay(
    mut commands: Commands,
    q_existing: Query<Entity, With<PickupFlashOverlay>>,
    q_damage: Query<Entity, With<DamageFlashOverlay>>,
    q_parent: Query<&ChildOf>,
) {
    if !q_existing.is_empty() {
        return;
    }

    let Some(damage_overlay) = q_damage.iter().next() else {
        // HUD Not Built Yet (or Damage Overlay Missing)
        // Nothing to Attach to
        return;
    };

    // Walk Up to Top-Most HUD Node (Root Spawned by setup_hud())
    let mut root = damage_overlay;
    while let Ok(child_of) = q_parent.get(root) {
        root = child_of.0;
    }

    // Spawn as the LAST child so it draws on top of view + status bar
    commands.entity(root).with_children(|p| {
        p.spawn((
            PickupFlashOverlay,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ));
    });
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

pub(crate) fn tick_pickup_flash(
    time: Res<Time>,
    mut flash: ResMut<super::PickupFlash>,
    damage: Res<super::DamageFlash>,
    death: Res<super::DeathOverlay>,
    mut q: Query<&mut BackgroundColor, With<PickupFlashOverlay>>,
) {
    if !flash.timer.is_finished() {
        flash.timer.tick(time.delta());
    }
    let mut a = flash.alpha();

    if damage.alpha() > 0.0 || death.alpha() > 0.0 {
        a = 0.0;
    }

    let c = flash.color;
    for mut bg in q.iter_mut() {
        *bg = BackgroundColor(Srgba::new(c.red, c.green, c.blue, a).into());
    }
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

pub(crate) fn tick_death_overlay(
    time: Res<Time>,
    mut death: ResMut<DeathOverlay>,
    mut q: Query<&mut BackgroundColor, With<DeathOverlayOverlay>>,
) {
    if death.active && !death.timer.is_finished() {
        death.timer.tick(time.delta());
    }

    let a = death.alpha();
    for mut bg in q.iter_mut() {
        *bg = BackgroundColor(Srgba::new(1.0, 0.0, 0.0, a).into());
    }
}

pub(crate) fn sync_game_over_overlay_visibility(
    game_over: Res<GameOver>,
    mut q: Query<&mut Visibility, With<GameOverOverlay>>,
) {
    for mut vis in q.iter_mut() {
        *vis = if game_over.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Maps HP (1..100) to Stage 0..6. HP<=0 Handled Separately as Dead
/// 7 Injury Stages + Dead = 8 Total States
fn stage_from_hp(hp: i32) -> usize {
    let hp = hp.clamp(1, 100);
    let dmg = 100 - hp;              // 0..99
    ((dmg * 7) / 100) as usize       // 0..6
}

/// Returns (Row, Col) for Base Face Given HP + Dir
/// Column Order Within a Group Matches Sprite Sheet: Forward, Right, Left
fn coords_for(hp: i32, dir: FaceDir) -> (usize, usize) {
    if hp <= 0 {
        return (1, 10); // Dead (r1_c10)
    }

    let stage = stage_from_hp(hp); // 0..6

    let dir_off = match dir {
        FaceDir::Forward => 0,
        FaceDir::Right => 1,
        FaceDir::Left => 2,
    };

    if stage < 4 {
        // Row 0 has Stages 0..3
        (0, stage * 3 + dir_off)
    } else {
        // Row 1 has Stages 4..6 (0..2 in That Row)
        let s = stage - 4;
        (1, s * 3 + dir_off)
    }
}

#[derive(Clone, Copy)]
struct HudLayout {
    pub hud_scale: f32,

    pub hud_w_px: f32,
    pub status_h_px: f32,

    pub digit_w_px: f32,
    pub digit_h_px: f32,
    pub digit_top_px: f32,

    pub score_x_px: f32,
    pub lives_x_px: f32,
    pub hp_x_px: f32,
    pub ammo_x_px: f32,
    pub floor_x_px: f32,

    pub key_w_px: f32,
    pub key_h_px: f32,
    pub key_x_px: f32,
    pub key_gold_y_px: f32,
    pub key_silver_y_px: f32,

    pub wep_x_px: f32,
    pub wep_top_px: f32,
    pub wep_w_px: f32,
    pub wep_h_px: f32,

    pub face_x_px: f32,
    pub face_top_px: f32,
    pub face_w_px: f32,
    pub face_h_px: f32,

    pub gun_px: f32,
}

#[derive(Clone)]
struct HudSetupAssets {
    pub weapon_idle: Handle<Image>,
    pub status_bar: Handle<Image>,
    pub ui_font: Handle<Font>,

    pub hud_digits: HudDigitSprites,
    pub hud_icons: HudIconSprites,
    pub hud_faces: HudFaceSprites,

    pub bj_pistol_0: Handle<Image>,
}

fn load_hud_setup_assets(
    commands: &mut Commands,
    asset_server: &AssetServer,
    hud: &HudState,
) -> HudSetupAssets {
    // Viewmodel Sprites
    let sprites = ViewModelSprites {
        knife: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/knife_{i}.png"))),
        pistol: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/pistol_{i}.png"))),
        machinegun: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/machinegun_{i}.png"))),
        chaingun: std::array::from_fn(|i| asset_server.load(format!("textures/weapons/chaingun_{i}.png"))),
    };
    commands.insert_resource(sprites.clone());

    // Starting Viewmodel Based on Selected Weapon
    let weapon_idle: Handle<Image> = sprites.idle(hud.selected);

    // HUD Digit Sprites
    let hud_digits = HudDigitSprites {
        digits: std::array::from_fn(|i| asset_server.load(format!("textures/hud/digits/digit_{i}.png"))),
        blank: asset_server.load("textures/hud/digits/digit_blank.png"),
    };
    commands.insert_resource(hud_digits.clone());

    let hud_icons = HudIconSprites {
        weapon_knife: asset_server.load("textures/hud/icons/weapon_knife.png"),
        weapon_pistol: asset_server.load("textures/hud/icons/weapon_pistol.png"),
        weapon_machinegun: asset_server.load("textures/hud/icons/weapon_machinegun.png"),
        weapon_chaingun: asset_server.load("textures/hud/icons/weapon_chaingun.png"),
        key_gold: asset_server.load("textures/hud/icons/key_gold.png"),
        key_silver: asset_server.load("textures/hud/icons/key_silver.png"),
    };
    commands.insert_resource(hud_icons.clone());

    // HUD Face Sprites
    let f = |r: u8, c: u8| asset_server.load(format!("textures/hud/faces/face_r{r}_c{c}.png"));

    let hud_faces = HudFaceSprites {
        bands: [
            [f(0, 0), f(0, 1), f(0, 2)],
            [f(0, 3), f(0, 4), f(0, 5)],
            [f(0, 6), f(0, 7), f(0, 8)],
            [f(0, 9), f(0, 10), f(0, 11)],
            [f(1, 0), f(1, 1), f(1, 2)],
            [f(1, 3), f(1, 4), f(1, 5)],
            [f(1, 6), f(1, 7), f(1, 8)],
        ],
        grin: f(1, 9),    // r1_c9  = Grin
        dead: f(1, 10),   // r1_c10 = Dead
        stone: f(1, 11),  // r1_c11 = God Mode
    };

    commands.insert_resource(hud_faces.clone());
    commands.insert_resource(HudFaceOverride::default());

    // Boxed HUD Strip Background (320x44)
    let status_bar: Handle<Image> = asset_server.load("textures/hud/status_bar.png");

    // Simple UI Text Font (Used for Game Over Overlay)
    let ui_font: Handle<Font> = asset_server.load("fonts/font.ttf");

    // End Level Font Sheet (Bitmap)
    let end_font_sheet: Handle<Image> = asset_server.load("textures/ui/level_end/end_level_font.png");
    commands.insert_resource(crate::ui::level_end_font::LevelEndFont { sheet: end_font_sheet });

    // BJ Card Sprites (Level End Screen)
    let bj_pistol_0: Handle<Image> = asset_server.load("textures/ui/level_end/bj_pistol_clean_0.png");
    let bj_pistol_1: Handle<Image> = asset_server.load("textures/ui/level_end/bj_pistol_clean_1.png");
    let bj_pistol_2: Handle<Image> = asset_server.load("textures/ui/level_end/bj_pistol_clean_2.png");

    commands.insert_resource(MissionBjCardSprites {
        cards: [bj_pistol_0.clone(), bj_pistol_1, bj_pistol_2],
    });

    HudSetupAssets {
        weapon_idle,
        status_bar,
        ui_font,
        hud_digits,
        hud_icons,
        hud_faces,
        bj_pistol_0,
    }
}

fn compute_hud_layout(q_windows: &Query<&Window, With<PrimaryWindow>>) -> HudLayout {
    // --- Native Wolf HUD Sizing (Current Strip-Only HUD) ---
    const HUD_W: f32 = 320.0;
    const STATUS_H: f32 = 44.0;

    // Digit Cell Size (Native)
    const DIGIT_W: f32 = 8.0;
    const DIGIT_H: f32 = 16.0;
    const DIGIT_TOP: f32 = 18.0;

    // Placement Tweaks (Native Coords)
    const SCORE_X: f32 = 48.0;
    const LIVES_X: f32 = 108.0;
    const HP_X: f32 = 168.0;
    const AMMO_X: f32 = 208.0;

    // Icon Sizes
    const KEY_W: f32 = 7.0;
    const KEY_H: f32 = 17.0;

    // Keys: Stacked
    const KEY_X: f32 = 242.0;
    const KEY_GOLD_Y: f32 = 5.2;
    const KEY_SILVER_Y: f32 = 23.0;

    // Weapon icon (to the right of the keys)
    const WEP_X: f32 = 262.0;
    const WEP_TOP: f32 = 9.0;
    const WEP_W: f32 = 48.0;
    const WEP_H: f32 = 24.0;

    // Face Placement (Native Coords, 24x32 Inside Face Window)
    const FACE_X: f32 = 138.0;
    const FACE_TOP: f32 = 7.0;
    const FACE_W: f32 = 24.0;
    const FACE_H: f32 = 32.0;

    // Current Player Level
    const FLOOR_X: f32 = 14.0;

    // Pixel-Perfect Integer Scale From Window Width
    let win = q_windows.iter().next().expect("PrimaryWindow");
    let win_w = win.resolution.width();
    let hud_scale_i = (win_w / HUD_W).floor().max(1.0) as i32;
    let hud_scale = hud_scale_i as f32;

    // Scaled Sizes
    let hud_w_px = HUD_W * hud_scale;
    let status_h_px = STATUS_H * hud_scale;

    let digit_w_px = DIGIT_W * hud_scale;
    let digit_h_px = DIGIT_H * hud_scale;
    let digit_top_px = DIGIT_TOP * hud_scale;

    let score_x_px = SCORE_X * hud_scale;
    let lives_x_px = LIVES_X * hud_scale;
    let hp_x_px = HP_X * hud_scale;
    let ammo_x_px = AMMO_X * hud_scale;
    let floor_x_px = FLOOR_X * hud_scale;

    // Scaled Key Placement
    let key_w_px = KEY_W * hud_scale;
    let key_h_px = KEY_H * hud_scale;
    let key_x_px = KEY_X * hud_scale;
    let key_gold_y_px = KEY_GOLD_Y * hud_scale;
    let key_silver_y_px = KEY_SILVER_Y * hud_scale;

    // Scaled Weapon Placement
    let wep_x_px = WEP_X * hud_scale;
    let wep_top_px = WEP_TOP * hud_scale;
    let wep_w_px = WEP_W * hud_scale;
    let wep_h_px = WEP_H * hud_scale;

    // Scaled Face Placement
    let face_x_px = FACE_X * hud_scale;
    let face_top_px = FACE_TOP * hud_scale;
    let face_w_px = FACE_W * hud_scale;
    let face_h_px = FACE_H * hud_scale;

    const GUN_SCALE: f32 = 6.5;
    const GUN_SRC_PX: f32 = 64.0;
    let gun_px = GUN_SRC_PX * GUN_SCALE;

    HudLayout {
        hud_scale,
        hud_w_px,
        status_h_px,
        digit_w_px,
        digit_h_px,
        digit_top_px,
        score_x_px,
        lives_x_px,
        hp_x_px,
        ammo_x_px,
        floor_x_px,
        key_w_px,
        key_h_px,
        key_x_px,
        key_gold_y_px,
        key_silver_y_px,
        wep_x_px,
        wep_top_px,
        wep_w_px,
        wep_h_px,
        face_x_px,
        face_top_px,
        face_w_px,
        face_h_px,
        gun_px,
    }
}

fn spawn_view_area(commands: &mut Commands, parent: Entity, weapon_idle: Handle<Image>, gun_px: f32) {
    commands.entity(parent).with_children(|ui| {
        // View Area Canvas (Scaled)
        ui.spawn(Node {
            width: Val::Px(gun_px),
            height: Val::Px(gun_px),
            flex_grow: 1.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|view| {
            // Absolute Overlay Layer (Does NOT Affect Layout)
            view.spawn(Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            });

            // ViewModel Root
            view.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            })
            .with_children(|vm| {
                vm.spawn((
                    ImageNode::new(weapon_idle),
                    Node {
                        width: Val::Px(gun_px),
                        height: Val::Px(gun_px),
                        ..default()
                    },
                ));
            });
        });
    });
}

fn spawn_status_bar(
    commands: &mut Commands,
    parent: Entity,
    layout: &HudLayout,
    status_bar: Handle<Image>,
    hud_digits: &HudDigitSprites,
    hud_icons: &HudIconSprites,
    hud_faces: &HudFaceSprites,
    hud: &HudState,
    current_level: &CurrentLevel,
) {
    // Wolf HUD Blue (0, 0, 164)
    let background_color: Srgba = Srgba::new(0.0, 0.0, 164.0 / 255.0, 1.0);

    commands.entity(parent).with_children(|ui| {
        ui.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(layout.status_h_px),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(background_color.into()),
        ))
        .with_children(|bar| {
            bar.spawn(Node {
                width: Val::Px(layout.hud_w_px),
                height: Val::Px(layout.status_h_px),
                position_type: PositionType::Relative,
                ..default()
            })
            .with_children(|inner| {
                inner.spawn((
                    ImageNode::new(status_bar.clone()),
                    ZIndex(0),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(layout.hud_w_px),
                        height: Val::Px(layout.status_h_px),
                        ..default()
                    },
                ));

                inner.spawn((
                    HudFaceImage,
                    ImageNode::new(hud_faces.bands[0][0].clone()),
                    ZIndex(1),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(layout.face_x_px),
                        top: Val::Px(layout.face_top_px),
                        width: Val::Px(layout.face_w_px),
                        height: Val::Px(layout.face_h_px),
                        ..default()
                    },
                ));

                inner.spawn((
                    HudWeaponIcon,
                    ImageNode::new(hud_icons.weapon(hud.selected)),
                    ZIndex(1),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(layout.wep_x_px),
                        top: Val::Px(layout.wep_top_px),
                        width: Val::Px(layout.wep_w_px),
                        height: Val::Px(layout.wep_h_px),
                        ..default()
                    },
                ));

                inner.spawn((
                    HudGoldKeyIcon,
                    ImageNode::new(hud_icons.key_gold.clone()),
                    ZIndex(1),
                    if hud.key_gold { Visibility::Visible } else { Visibility::Hidden },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(layout.key_x_px),
                        top: Val::Px(layout.key_gold_y_px),
                        width: Val::Px(layout.key_w_px),
                        height: Val::Px(layout.key_h_px),
                        ..default()
                    },
                ));

                inner.spawn((
                    HudSilverKeyIcon,
                    ImageNode::new(hud_icons.key_silver.clone()),
                    ZIndex(1),
                    if hud.key_silver { Visibility::Visible } else { Visibility::Hidden },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(layout.key_x_px),
                        top: Val::Px(layout.key_silver_y_px),
                        width: Val::Px(layout.key_w_px),
                        height: Val::Px(layout.key_h_px),
                        ..default()
                    },
                ));

                let floor_num: i32 = current_level.0.floor_number();
                let floor_digits = split_right_aligned_blanks(floor_num, 2);

                inner.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(layout.floor_x_px),
                    top: Val::Px(layout.digit_top_px),
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|floor| {
                    for (slot, dopt) in floor_digits.iter().enumerate() {
                        let handle = match dopt {
                            Some(d) => hud_digits.digits[*d].clone(),
                            None => hud_digits.blank.clone(),
                        };
                        floor.spawn((
                            HudFloorDigit(slot),
                            ImageNode::new(handle),
                            Node {
                                width: Val::Px(layout.digit_w_px),
                                height: Val::Px(layout.digit_h_px),
                                ..default()
                            },
                        ));
                    }
                });

                let score_digits = split_score_6_blanks(hud.score);
                inner.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(layout.score_x_px),
                    top: Val::Px(layout.digit_top_px),
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
                                width: Val::Px(layout.digit_w_px),
                                height: Val::Px(layout.digit_h_px),
                                ..default()
                            },
                        ));
                    }
                });

                let lives_digits = split_right_aligned_blanks(hud.lives, 2);
                inner.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(layout.lives_x_px),
                    top: Val::Px(layout.digit_top_px),
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
                                width: Val::Px(layout.digit_w_px),
                                height: Val::Px(layout.digit_h_px),
                                ..default()
                            },
                        ));
                    }
                });

                let hp_digits = split_3_right_aligned(hud.hp);
                inner.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(layout.hp_x_px),
                    top: Val::Px(layout.digit_top_px),
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
                                width: Val::Px(layout.digit_w_px),
                                height: Val::Px(layout.digit_h_px),
                                ..default()
                            },
                        ));
                    }
                });

                let ammo_digits = split_3_right_aligned(hud.ammo);
                inner.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(layout.ammo_x_px),
                    top: Val::Px(layout.digit_top_px),
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
                                width: Val::Px(layout.digit_w_px),
                                height: Val::Px(layout.digit_h_px),
                                ..default()
                            },
                        ));
                    }
                });
            });
        });
    });
}

fn spawn_status_bar_container(
    commands: &mut Commands,
    parent: Entity,
    // sizes
    hud_w_px: f32,
    status_h_px: f32,
    // background
    background_color: Srgba,
    // background strip image
    status_bar: Handle<Image>,
    // HUD resources / values (pass by value so closures are happy)
    hud: HudState,
    current_level: CurrentLevel,
    hud_digits: HudDigitSprites,
    hud_icons: HudIconSprites,
    hud_faces: HudFaceSprites,
    // layout numbers (already computed px)
    digit_w_px: f32,
    digit_h_px: f32,
    digit_top_px: f32,
    score_x_px: f32,
    lives_x_px: f32,
    hp_x_px: f32,
    ammo_x_px: f32,
    floor_x_px: f32,
    key_w_px: f32,
    key_h_px: f32,
    key_x_px: f32,
    key_gold_y_px: f32,
    key_silver_y_px: f32,
    wep_x_px: f32,
    wep_top_px: f32,
    wep_w_px: f32,
    wep_h_px: f32,
    face_x_px: f32,
    face_top_px: f32,
    face_w_px: f32,
    face_h_px: f32,
) {
    commands.entity(parent).with_children(|ui| {
        ui.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(status_h_px),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(background_color.into()),
        ))
        .with_children(|bar| {
            bar.spawn(Node {
                width: Val::Px(hud_w_px),
                height: Val::Px(status_h_px),
                position_type: PositionType::Relative,
                ..default()
            })
            .with_children(|inner| {
                // Background strip
                inner.spawn((
                    ImageNode::new(status_bar),
                    ZIndex(0),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(hud_w_px),
                        height: Val::Px(status_h_px),
                        ..default()
                    },
                ));

                // Face
                inner.spawn((
                    HudFaceImage,
                    ImageNode::new(hud_faces.bands[0][0].clone()),
                    ZIndex(1),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(face_x_px),
                        top: Val::Px(face_top_px),
                        width: Val::Px(face_w_px),
                        height: Val::Px(face_h_px),
                        ..default()
                    },
                ));

                // Weapon icon
                inner.spawn((
                    HudWeaponIcon,
                    ImageNode::new(hud_icons.weapon(hud.selected)),
                    ZIndex(1),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(wep_x_px),
                        top: Val::Px(wep_top_px),
                        width: Val::Px(wep_w_px),
                        height: Val::Px(wep_h_px),
                        ..default()
                    },
                ));

                // Gold key
                inner.spawn((
                    HudGoldKeyIcon,
                    ImageNode::new(hud_icons.key_gold.clone()),
                    ZIndex(1),
                    if hud.key_gold {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(key_x_px),
                        top: Val::Px(key_gold_y_px),
                        width: Val::Px(key_w_px),
                        height: Val::Px(key_h_px),
                        ..default()
                    },
                ));

                // Silver key
                inner.spawn((
                    HudSilverKeyIcon,
                    ImageNode::new(hud_icons.key_silver.clone()),
                    ZIndex(1),
                    if hud.key_silver {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(key_x_px),
                        top: Val::Px(key_silver_y_px),
                        width: Val::Px(key_w_px),
                        height: Val::Px(key_h_px),
                        ..default()
                    },
                ));

                // FLOOR
                let floor_num: i32 = current_level.0.floor_number();
                let floor_digits = split_right_aligned_blanks(floor_num, 2);

                inner.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(floor_x_px),
                    top: Val::Px(digit_top_px),
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|floor| {
                    for (slot, dopt) in floor_digits.iter().enumerate() {
                        let handle = match dopt {
                            Some(d) => hud_digits.digits[*d].clone(),
                            None => hud_digits.blank.clone(),
                        };
                        floor.spawn((
                            HudFloorDigit(slot),
                            ImageNode::new(handle),
                            Node {
                                width: Val::Px(digit_w_px),
                                height: Val::Px(digit_h_px),
                                ..default()
                            },
                        ));
                    }
                });

                // SCORE
                let score_digits = split_score_6_blanks(hud.score);
                inner.spawn(Node {
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
                inner.spawn(Node {
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
                inner.spawn(Node {
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
                inner.spawn(Node {
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

fn spawn_mission_success_overlay(
    commands: &mut Commands,
    parent: Entity,
    ui_font: Handle<Font>,
    hud_scale: f32,
    start_floor_num: i32,
    bj_pistol_0: Handle<Image>,
) {
    commands.entity(parent).with_children(|ui| {
        // Your existing body goes here unchanged, just using `ui.spawn(...)`
        // Keep your existing sizing math based on `hud_scale`

        ui.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Visibility::Hidden,
            MissionSuccessOverlay,
        ))
        .with_children(|ms| {
            // BJ Card (Animated Later)
            ms.spawn((
                ImageNode::new(bj_pistol_0),
                Node {
                    width: Val::Px(64.0 * hud_scale),
                    height: Val::Px(80.0 * hud_scale),
                    ..default()
                },
                MissionBjCardImage,
            ));

            // Title text etc (keep your existing text tree)
            ms.spawn((
                Text::new(format!("FLOOR {} COMPLETED", start_floor_num)),
                TextFont {
                    font: ui_font,
                    font_size: 32.0 * hud_scale,
                    ..default()
                },
            ));
        });
    });
}

fn spawn_game_over_overlay(commands: &mut Commands, parent: Entity, ui_font: Handle<Font>) {
    commands.entity(parent).with_children(|ui| {
        ui.spawn((
            GameOverOverlay,
            ZIndex(100),
            Visibility::Hidden,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Srgba::new(0.0, 0.0, 0.0, 0.80).into()),
        ))
        .with_children(|go| {
            go.spawn((
                Text::new("GAME OVER"),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 64.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(Justify::Center),
            ));

            go.spawn((
                Text::new("Press ENTER to Continue ..."),
                TextFont {
                    font: ui_font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
    });
}

pub(crate) fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    hud: Res<HudState>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    current_level: Res<CurrentLevel>,
) {
    let assets = load_hud_setup_assets(&asset_server);
    let layout = compute_hud_layout(&q_windows);

    // Root HUD Node (Full Screen)
    let root = commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
        ))
        .id();

    // View Area (276px Tall)
    let weapon_idle = assets.weapon_idle.clone();
    let gun_px = layout.gun_px;
    spawn_view_area(&mut commands, root, weapon_idle, gun_px);

    // Status Bar (44px Tall)
    spawn_status_bar(
        &mut commands,
        root,
        &layout,
        assets.status_bar.clone(),
        &assets.hud_digits,
        &assets.hud_icons,
        &assets.hud_faces,
        &hud,
        &current_level,
    );

    spawn_game_over_overlay(&mut commands, root, assets.ui_font.clone());

    // Mission Success Overlay
    let start_floor_num: i32 = current_level.0.floor_number();
    spawn_mission_success_overlay(
        &mut commands,
        root,
        assets.ui_font.clone(),
        layout.hud_scale,
        start_floor_num,
        assets.bj_pistol_0.clone(),
    );
}
