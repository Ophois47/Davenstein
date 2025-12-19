use bevy::prelude::*;
use super::HudState;

#[derive(Component)]
pub(super) struct HudHpText;

#[derive(Component)]
pub(super) struct HudAmmoText;

#[derive(Component)]
pub(super) struct ViewModelImage;

pub(crate) fn setup_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/honda_font.ttf");
    let weapon_idle: Handle<Image> = asset_server.load("ui/weapons/pistol_idle.png");

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
