use super::HudState;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use davelib::audio::{PlaySfx, SfxKind};
use davelib::player::Player;

#[derive(Component)]
pub(super) struct HudHpText;

#[derive(Component)]
pub(super) struct HudAmmoText;

#[derive(Component)]
pub(super) struct ViewModelImage;

#[derive(Resource, Clone)]
pub(crate) struct ViewModelSprites {
    pub knife_idle: Handle<Image>,
    pub knife_fire: Handle<Image>,

    pub pistol: [Handle<Image>; 5],
    pub machinegun: [Handle<Image>; 5],
    pub chaingun: [Handle<Image>; 5],
}

impl ViewModelSprites {
    pub fn idle(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            Knife => self.knife_idle.clone(),
            Pistol => self.pistol[0].clone(),
            MachineGun => self.machinegun[0].clone(),
            Chaingun => self.chaingun[0].clone(),
        }
    }

    pub fn fire_simple(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            Pistol => self.pistol[2].clone(),
            Knife => self.knife_fire.clone(),
            _ => self.idle(w),
        }
    }

    pub fn pistol_frame(&self, idx: usize) -> Handle<Image> {
        self.pistol[idx.min(4)].clone()
    }

    // Direct indexing (keep this for any code that truly wants idx)
    #[allow(dead_code)]
    pub fn fire_frame(&self, w: crate::combat::WeaponSlot, idx: usize) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            MachineGun => self.machinegun[idx].clone(),
            Chaingun => self.chaingun[idx].clone(),
            _ => self.fire_simple(w),
        }
    }

    // NEW: Wolf-faithful full-auto animation frame selection.
    // `cycle` is a counter (0,1,2,3,...) NOT a direct sprite index.
    pub fn auto_fire(&self, w: crate::combat::WeaponSlot, cycle: usize) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;

        match w {
            // Machinegun: "bring up/forward" (1) <-> flash (2)
            MachineGun => {
                // Wolf-like MG cycle: bring up/forward -> flash -> recover/back
                // Choose the "back" frame as 3 OR 4 depending on which looks like recoil recovery.
                const SEQ: [usize; 3] = [1, 2, 3];
                self.machinegun[SEQ[cycle % SEQ.len()]].clone()
            }

            // Chaingun: forward (1), flash A (2), forward (1), flash B (3)
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

    // Only allow weapon selection/firing while mouse is locked (Wolf-ish)
    let locked = cursor.grab_mode == CursorGrabMode::Locked;
    if !locked {
        *armed = false;
        *fire_anim_accum = 0.0;
        *last_weapon = Some(hud.selected);

        // Hard snap viewmodel to idle if unlocked
        weapon.fire_cycle = 0;
        weapon.showing_fire = false;
        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.idle(hud.selected);
        }
        return;
    }

    // Prevent the very first click (used to grab the cursor) from also firing
    if !*armed {
        *armed = true;
        *fire_anim_accum = 0.0;
        *last_weapon = Some(hud.selected);
        return;
    }

    // Weapon selection (1–4)
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

    // If weapon changed externally somehow, reset anim accumulator
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

    // Per-weapon params (Wolf-ish tics)
    const TIC: f32 = 1.0 / 70.0;
    let (cooldown_secs, flash_secs, ammo_cost, max_dist) = match hud.selected {
        WeaponSlot::Knife => (10.0 * TIC, 8.0 * TIC, 0, 1.5),
        WeaponSlot::Pistol => (25.0 * TIC, 16.0 * TIC, 1, 64.0),
        WeaponSlot::MachineGun => (12.0 * TIC, 8.0 * TIC, 1, 64.0),
        WeaponSlot::Chaingun => (6.0 * TIC, 8.0 * TIC, 1, 64.0),
    };

    // Ensure timers match current weapon
    if (weapon.cooldown.duration().as_secs_f32() - cooldown_secs).abs() > f32::EPSILON {
        weapon.cooldown = Timer::from_seconds(cooldown_secs, TimerMode::Once);
        weapon.cooldown.set_elapsed(std::time::Duration::from_secs_f32(cooldown_secs));
    }
    if (weapon.flash.duration().as_secs_f32() - flash_secs).abs() > f32::EPSILON {
        weapon.flash = Timer::from_seconds(flash_secs, TimerMode::Once);
    }

    // --- Weapon kind flags (MG handled differently; CG unchanged) ---
    let is_machinegun = hud.selected == WeaponSlot::MachineGun;
    let is_chaingun = hud.selected == WeaponSlot::Chaingun;
    let is_full_auto = is_machinegun || is_chaingun;

    let trigger_down = mouse.pressed(MouseButton::Left);
    let trigger_pressed = mouse.just_pressed(MouseButton::Left);

    // Tick cooldown always
    weapon.cooldown.tick(dt);

    // Ammo check
    let mut has_ammo = ammo_cost == 0 || hud.ammo >= ammo_cost;

    // --- Flash timer handling ---
    // Knife + Pistol keep existing flash behavior.
    // MachineGun ALSO uses flash timer, but Chaingun does NOT (it has its own cycling).
    if weapon.showing_fire && (!is_chaingun) {
        weapon.flash.tick(dt);

        // PISTOL: advance through 4-frame sequence across the flash timer
        if hud.selected == WeaponSlot::Pistol {
            let dur = weapon.flash.duration().as_secs_f32().max(0.0001);
            let t = (weapon.flash.elapsed_secs() / dur).clamp(0.0, 1.0);

            let frame = if t < 0.25 {
                1 // raise
            } else if t < 0.50 {
                2 // muzzle flash
            } else if t < 0.75 {
                3 // recover
            } else {
                4 // settle
            };

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.pistol_frame(frame);
            }
        }

        if weapon.flash.is_finished() {
            weapon.showing_fire = false;

            if let Ok(mut img) = vm_q.single_mut() {
                if hud.selected == WeaponSlot::Pistol {
                    img.image = sprites.pistol_frame(0); // idle
                } else if is_machinegun && trigger_down && has_ammo {
                    img.image = sprites.fire_frame(WeaponSlot::MachineGun, 1); // forward
                } else {
                    img.image = sprites.idle(hud.selected);
                }
            }
        }
    }

    // --- CHAINGUN ONLY: keep your current perfect cycling behavior ---
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

    // --- MACHINEGUN: while holding (and not flashing), keep forward pose ---
    // (This is what makes it look like Wolf between flashes.)
    if is_machinegun && trigger_down && has_ammo && !weapon.showing_fire {
        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.fire_frame(WeaponSlot::MachineGun, 1); // forward
        }
    }

    // MACHINEGUN: ALWAYS snap back to idle when trigger is not held.
    // (Prevents rare "stuck forward" posture after releasing the mouse.)
    if is_machinegun && !trigger_down {
        weapon.showing_fire = false;
        weapon.fire_cycle = 0;
        *auto_linger = 0.0;
        // optional: also reset flash so we don't keep counting down invisibly
        weapon.flash.reset();

        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.idle(WeaponSlot::MachineGun);
        }
    }

    // Fire intent
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
        if !is_full_auto {
            weapon.showing_fire = true;
            weapon.flash.reset();

            if hud.selected == WeaponSlot::Pistol {
                // Start pistol anim on "raise" frame, not flash
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.pistol_frame(1);
                }
            } else {
                // Knife stays 2-frame behavior
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

pub(crate) fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    hud: Res<HudState>,
) {
    let font: Handle<Font> = asset_server.load("fonts/honda_font.ttf");

    // New Wolf sheet naming (assets/weapons/*.png)
    let sprites = ViewModelSprites {
        knife_idle: asset_server.load("weapons/knife_0.png"),
        knife_fire: asset_server.load("weapons/knife_2.png"),

        pistol: std::array::from_fn(|i| {
            asset_server.load(format!("weapons/pistol_{i}.png"))
        }),
        machinegun: std::array::from_fn(|i| {
            asset_server.load(format!("weapons/machinegun_{i}.png"))
        }),
        chaingun: std::array::from_fn(|i| {
            asset_server.load(format!("weapons/chaingun_{i}.png"))
        }),
    };

    // Make sprites available to the firing/viewmodel system
    commands.insert_resource(sprites.clone());

    // Pick the correct starting viewmodel based on currently selected weapon
    let weapon_idle: Handle<Image> = sprites.idle(hud.selected);

    const STATUS_BAR_H: f32 = 64.0;
    const UI_PAD: f32 = 8.0;

    const GUN_SCALE: f32 = 6.5;
    const GUN_SRC_PX: f32 = 64.0;
    const GUN_PX: f32 = GUN_SRC_PX * GUN_SCALE;

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
            });

            // Status bar (still simple for now)
            ui.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(STATUS_BAR_H),
                    padding: UiRect::all(Val::Px(UI_PAD)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::BLACK.into()),
            ))
            .with_children(|bar| {
                bar.spawn((
                    HudHpText,
                    Text::new("HP 100"),
                    TextFont {
                        font: font.clone(),
                        font_size: 36.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                bar.spawn((
                    HudAmmoText,
                    Text::new("AMMO 100"),
                    TextFont {
                        font,
                        font_size: 36.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

pub(crate) fn sync_hud_text(
    hud: Res<HudState>,
    mut q: Query<(&mut Text, Option<&HudHpText>, Option<&HudAmmoText>)>,
) {
    if !hud.is_changed() {
        return;
    }

    for (mut text, hp_tag, ammo_tag) in &mut q {
        if hp_tag.is_some() {
            *text = Text::new(format!("HP {}", hud.hp));
        } else if ammo_tag.is_some() {
            *text = Text::new(format!("AMMO {}", hud.ammo));
        }
    }
}
