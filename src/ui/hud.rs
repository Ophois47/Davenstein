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
    pub idle: Handle<Image>,
    pub fire: Handle<Image>,
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
        const PISTOL_COOLDOWN_TICS: f32 = 32.0;
        const PISTOL_FLASH_TICS: f32 = 14.0;

        let cooldown_secs = PISTOL_COOLDOWN_TICS * TIC;
        let flash_secs = PISTOL_FLASH_TICS * TIC;

        let mut cooldown = Timer::from_seconds(cooldown_secs, TimerMode::Once);
        cooldown.set_elapsed(std::time::Duration::from_secs_f32(cooldown_secs)); // start ready

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
    cursor: Single<&CursorOptions>,
    sprites: Option<Res<ViewModelSprites>>,
    mut weapon: ResMut<WeaponState>,
    mut hud: ResMut<HudState>,
    mut vm_q: Query<&mut ImageNode, With<ViewModelImage>>,
    q_player: Query<&Transform, With<Player>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut armed: Local<bool>,
) {
    // Don’t do anything until setup_hud has inserted the sprite handles
    let Some(sprites) = sprites else { return; };

    // Tick timers
    let dt = time.delta();
    weapon.cooldown.tick(dt);

    // Handle the "flash" (fire frame -> back to idle)
    if weapon.showing_fire {
        weapon.flash.tick(dt);
        if weapon.flash.is_finished() {
            weapon.showing_fire = false;
            if let Ok(mut img) = vm_q.single_mut() {
                img.image = sprites.idle.clone();
            }
        }
    }

    // Only allow firing while mouse is locked
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

    // Fire
    if mouse.just_pressed(MouseButton::Left)
        && weapon.cooldown.is_finished()
        && hud.ammo > 0
    {
        hud.ammo -= 1;

        weapon.cooldown.reset();
        weapon.flash.reset();
        weapon.showing_fire = true;

        if let Ok(mut img) = vm_q.single_mut() {
            img.image = sprites.fire.clone();
        }

        if let Ok(tf) = q_player.single() {
            // keep y consistent with your other SFX
            let pos = Vec3::new(tf.translation.x, 0.6, tf.translation.z);
            sfx.write(PlaySfx { kind: SfxKind::PistolFire, pos });
        }
    }
}

pub(crate) fn setup_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/honda_font.ttf");
    let weapon_idle: Handle<Image> = asset_server.load("ui/weapons/pistol_idle.png");
    let weapon_fire: Handle<Image> = asset_server.load("ui/weapons/pistol_fire.png");

    commands.insert_resource(ViewModelSprites {
        idle: weapon_idle.clone(),
        fire: weapon_fire.clone(),
    });

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

            // Status bar
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
                    TextFont { font: font.clone(), font_size: 24.0, ..default() },
                    TextColor(Color::WHITE),
                ));

                bar.spawn((
                    HudAmmoText,
                    Text::new("AMMO 8"),
                    TextFont { font, font_size: 24.0, ..default() },
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
