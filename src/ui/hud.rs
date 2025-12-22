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

    pub pistol_idle: Handle<Image>,
    pub pistol_fire: Handle<Image>,

    pub mg_idle: Handle<Image>,
    pub mg_fire: Handle<Image>,

    pub chaingun_idle: Handle<Image>,
    pub chaingun_fire: Handle<Image>,
}

impl ViewModelSprites {
    pub fn idle(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            Knife => self.knife_idle.clone(),
            Pistol => self.pistol_idle.clone(),
            MachineGun => self.mg_idle.clone(),
            Chaingun => self.chaingun_idle.clone(),
        }
    }

    pub fn fire(&self, w: crate::combat::WeaponSlot) -> Handle<Image> {
        use crate::combat::WeaponSlot::*;
        match w {
            Knife => self.knife_fire.clone(),
            Pistol => self.pistol_fire.clone(),
            MachineGun => self.mg_fire.clone(),
            Chaingun => self.chaingun_fire.clone(),
        }
    }
}

#[derive(Resource)]
pub(crate) struct WeaponState {
    pub cooldown: Timer,
    pub flash: Timer,
    pub showing_fire: bool,
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
) {
    let Some(sprites) = sprites else { return; };

    // Tick timers
    let dt = time.delta();
    weapon.cooldown.tick(dt);

    // Revert fire->idle when flash finishes
    if weapon.showing_fire {
        weapon.flash.tick(dt);
        if weapon.flash.is_finished() {
            weapon.showing_fire = false;
            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.idle(hud.selected);
            }
        }
    }

    // Only allow weapon selection/firing while mouse is locked (Wolf-ish)
    let locked = cursor.grab_mode == CursorGrabMode::Locked;
    if !locked {
        *armed = false;
        return;
    }

    // Prevent the very first click (used to grab the cursor) from also firing
    if !*armed {
        *armed = true;
        return;
    }

    // Weapon selection (1–4)
    for code in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4] {
        if keys.just_pressed(code) {
            if let Some(slot) = crate::combat::WeaponSlot::from_digit_key(code) {
                if hud.owns(slot) {
                    hud.selected = slot;
                    weapon.showing_fire = false;
                    weapon.flash.reset(); // harmless
                    if let Ok(mut img) = vm_q.single_mut() {
                        img.image = sprites.idle(hud.selected);
                    }
                }
            }
        }
    }

    // Per-weapon params (Wolf-ish tics)
    const TIC: f32 = 1.0 / 70.0;
    let (cooldown_secs, flash_secs, ammo_cost, max_dist) = match hud.selected {
        crate::combat::WeaponSlot::Knife => (10.0 * TIC, 8.0 * TIC, 0, 1.5),
        crate::combat::WeaponSlot::Pistol => (20.0 * TIC, 12.0 * TIC, 1, 64.0),
        crate::combat::WeaponSlot::MachineGun => (8.0 * TIC, 8.0 * TIC, 1, 64.0),
        crate::combat::WeaponSlot::Chaingun => (6.0 * TIC, 8.0 * TIC, 1, 64.0),
    };

    // Ensure timers match current weapon (simple + safe)
    if weapon.cooldown.duration().as_secs_f32() != cooldown_secs {
        weapon.cooldown = Timer::from_seconds(cooldown_secs, TimerMode::Once);
        weapon.cooldown.set_elapsed(std::time::Duration::from_secs_f32(cooldown_secs));
    }
    if weapon.flash.duration().as_secs_f32() != flash_secs {
        weapon.flash = Timer::from_seconds(flash_secs, TimerMode::Once);
    }

    // Fire
    let has_ammo = ammo_cost == 0 || hud.ammo >= ammo_cost;
    if mouse.just_pressed(MouseButton::Left) && weapon.cooldown.is_finished() && has_ammo {
        // Spend ammo (knife is 0 cost)
        if ammo_cost > 0 {
            hud.ammo -= ammo_cost;
        }

        // Start cooldown + flash
        weapon.cooldown.reset();
        weapon.flash.reset();
        weapon.showing_fire = true;

        // Swap viewmodel to "fire" frame
        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.fire(hud.selected);
        }

        // Emit SFX + FireShot
        if let Ok(tf) = q_player.single() {
            let origin = tf.translation;
            let dir = (tf.rotation * Vec3::NEG_Z).normalize();
            let sfx_pos = Vec3::new(origin.x, 0.6, origin.z);

            // Weapon fire SFX (knife swing vs pistol shot)
            match hud.selected {
                crate::combat::WeaponSlot::Knife => {
                    sfx.write(PlaySfx { kind: SfxKind::KnifeSwing, pos: sfx_pos });
                }
                crate::combat::WeaponSlot::Pistol => {
                    sfx.write(PlaySfx { kind: SfxKind::PistolFire, pos: sfx_pos });
                }
                crate::combat::WeaponSlot::MachineGun => {
                    sfx.write(PlaySfx { kind: SfxKind::MachineGunFire, pos: sfx_pos });
                }
                crate::combat::WeaponSlot::Chaingun => {
                    sfx.write(PlaySfx { kind: SfxKind::ChaingunFire, pos: sfx_pos });
                }
            }

            // FireShot (combat decides what gets hit)
            fire_ev.write(crate::combat::FireShot {
                weapon: hud.selected,
                origin,
                dir,
                max_dist,
            });
        }
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

        mg_idle: asset_server.load("weapons/machinegun_0.png"),
        mg_fire: asset_server.load("weapons/machinegun_2.png"),

        chaingun_idle: asset_server.load("weapons/chaingun_0.png"),
        chaingun_fire: asset_server.load("weapons/chaingun_2.png"),
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
                        font_size: 32.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                bar.spawn((
                    HudAmmoText,
                    Text::new("AMMO 25"),
                    TextFont {
                        font,
                        font_size: 32.0,
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
