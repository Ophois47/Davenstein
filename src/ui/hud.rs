use super::HudState;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use davelib::audio::{PlaySfx, SfxKind};
use davelib::player::Player;
use crate::combat::WeaponSlot::*;

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

    pub pistol_idle: Handle<Image>,
    pub pistol_fire: Handle<Image>,

    pub machinegun: [Handle<Image>; 5],
    pub chaingun: [Handle<Image>; 5],
}

impl ViewModelSprites {
    pub fn idle(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            Knife => self.knife_idle.clone(),
            Pistol => self.pistol_idle.clone(),
            MachineGun => self.machinegun[0].clone(),
            Chaingun => self.chaingun[0].clone(),
        }
    }

    pub fn fire_simple(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        // For Knife/Pistol only (2-frame behavior stays)
        use crate::combat::WeaponSlot::*;
        match w {
            Knife => self.knife_fire.clone(),
            Pistol => self.pistol_fire.clone(),
            _ => self.idle(w),
        }
    }

    // Direct indexing (keep this for any code that truly wants idx)
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
                const SEQ: [usize; 2] = [1, 2];
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
        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.idle(hud.selected);
        }
    }

    // Per-weapon params (Wolf-ish tics)
    const TIC: f32 = 1.0 / 70.0;
    let (cooldown_secs, flash_secs, ammo_cost, max_dist) = match hud.selected {
        WeaponSlot::Knife => (10.0 * TIC, 8.0 * TIC, 0, 1.5),
        WeaponSlot::Pistol => (25.0 * TIC, 12.0 * TIC, 1, 64.0),
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

    let is_full_auto = matches!(hud.selected, WeaponSlot::MachineGun | WeaponSlot::Chaingun);
    let trigger_down = mouse.pressed(MouseButton::Left);
    let trigger_pressed = mouse.just_pressed(MouseButton::Left);

    // Tick cooldown always
    weapon.cooldown.tick(dt);

    // Revert fire->idle when flash finishes (NOT for full-auto)
    if weapon.showing_fire && !is_full_auto {
        weapon.flash.tick(dt);
        if weapon.flash.is_finished() {
            weapon.showing_fire = false;
            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.idle(hud.selected);
            }
        }
    }

    // Ammo check
    let mut has_ammo = ammo_cost == 0 || hud.ammo >= ammo_cost;

    // FULL-AUTO ANIMATION (Wolf-like):
    // While held + has ammo, loop firing frames (2 <-> 4) every 6 tics.
    // Only return to idle when trigger released or ammo is gone.
    let firing_anim_tic_secs = 12.0 * TIC;

    if is_full_auto && trigger_down && has_ammo {
        *auto_linger = 0.0;

        // If we just entered firing, force an immediate firing frame (tap/hold responsiveness)
        if weapon.fire_cycle == 0 {
            weapon.showing_fire = true;

            // Start at cycle 0 and immediately display it
            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
            }
        }

        *fire_anim_accum += dt_secs;

        // Advance at most one anim step per rendered frame (prevents flicker)
        if *fire_anim_accum >= firing_anim_tic_secs {
            *fire_anim_accum -= firing_anim_tic_secs;

            // Cycle counter advances (do NOT use 2/4 here anymore)
            weapon.fire_cycle = weapon.fire_cycle.wrapping_add(1);
            weapon.showing_fire = true;

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
            }
        }
    } else {
        *fire_anim_accum = 0.0;

        if is_full_auto && (!trigger_down || !has_ammo) {
            // Count down linger time after the last shot.
            if *auto_linger > 0.0 {
                *auto_linger = (*auto_linger - dt_secs).max(0.0);

                // Keep showing the last firing frame during the linger.
                // (Any non-zero fire_cycle means we have a last firing pose to show.)
                if weapon.fire_cycle != 0 {
                    weapon.showing_fire = true;
                    if let Ok(mut img) = vm_q.single_mut() {
                        img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
                    }
                }
            } else {
                // Linger expired: now snap to idle.
                weapon.fire_cycle = 0;
                weapon.showing_fire = false;
                if let Ok(mut img) = vm_q.single_mut() {
                    img.image = sprites.idle(hud.selected);
                }
            }
        } else {
            // If we're not in a "need to idle" condition, make sure linger isn't accumulating weirdly.
            *auto_linger = 0.0;
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
        crate::combat::WeaponSlot::Chaingun => 1,
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
        
        if is_full_auto {
            *auto_linger = 0.10; // seconds; tweak 0.08..0.14
        }

        // Full-auto: tapping should still show a firing frame on the shot
        if is_full_auto {
            weapon.showing_fire = true;

            // Start on a firing frame immediately
            weapon.fire_cycle = match hud.selected {
                WeaponSlot::Chaingun => 2,     // firing frame
                WeaponSlot::MachineGun => 2,   // firing frame
                _ => weapon.fire_cycle,
            };

            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.auto_fire(hud.selected, weapon.fire_cycle);
            }
        }

        // For non-full-auto, show the simple fire sprite immediately
        if !is_full_auto {
            weapon.showing_fire = true;
            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.fire_simple(hud.selected);
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

        pistol_idle: asset_server.load("weapons/pistol_0.png"),
        pistol_fire: asset_server.load("weapons/pistol_2.png"),

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
