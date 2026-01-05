/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::window::{
    CursorGrabMode,
    CursorOptions,
    PrimaryWindow,
    WindowResized,
};

use davelib::audio::{MusicMode, MusicModeKind};
use davelib::player::PlayerControlLock;

pub const SPLASH_0_PATH: &str = "textures/ui/splash0.png";
pub const SPLASH_1_PATH: &str = "textures/ui/splash1.png";
pub const MAIN_MENU_PATH: &str = "textures/ui/main_menu.png";

// Used to Compute Clean Integer UI Scale
const BASE_W: f32 = 320.0;
const BASE_H: f32 = 200.0;

#[derive(Component)]
pub struct SplashUi;

#[derive(Component)]
struct SplashImage;

#[derive(Component)]
struct MenuHint;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
enum SplashStep {
    Splash0,
    Splash1,
    Menu,
    Done,
}

impl Default for SplashStep {
    fn default() -> Self {
        SplashStep::Splash0
    }
}

#[derive(Resource)]
struct SplashImages {
    splash1: Handle<Image>,
    menu: Handle<Image>,
}

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashStep>();
        app.add_systems(
            Update,
            (
                splash_advance_on_any_input,
                splash_resize_on_window_change,
            )
                .chain(),
        );
    }
}

fn compute_scaled_size(win_w: f32, win_h: f32) -> (f32, f32) {
    let scale = (win_w / BASE_W).min(win_h / BASE_H).floor().max(1.0);
    (BASE_W * scale, BASE_H * scale)
}

pub fn setup_splash(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut lock: ResMut<PlayerControlLock>,
    mut music_mode: ResMut<MusicMode>,
    mut cursor: Single<&mut CursorOptions>,
    q_win: Single<&Window, With<PrimaryWindow>>,
) {
    // Freeze Gameplay Input While Splash / Menu Up
    lock.0 = true;

    // Splash Music Should be Active During Splash Flow
    music_mode.0 = MusicModeKind::Splash;

    // Ensure Mouse is Released and Visible
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;

    // Load Images
    let first = asset_server.load(SPLASH_0_PATH);
    let second = asset_server.load(SPLASH_1_PATH);
    let menu = asset_server.load(MAIN_MENU_PATH);

    // Keep Handles for Swaps
    commands.insert_resource(SplashImages {
        splash1: second.clone(),
        menu: menu.clone(),
    });

    // Spawn First Splash Immediately
    let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
    spawn_splash_ui(&mut commands, first, w, h);
}

fn spawn_splash_ui(commands: &mut Commands, image: Handle<Image>, w: f32, h: f32) {
    commands
        .spawn((
            SplashUi, // ONLY on root
            ZIndex(1000),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            root.spawn((
                SplashImage, // child marker is fine
                ImageNode::new(image),
                Node {
                    width: Val::Px(w),
                    height: Val::Px(h),
                    ..default()
                },
            ));
        });
}

fn splash_advance_on_any_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut step: ResMut<SplashStep>,
    imgs: Option<Res<SplashImages>>,
    q_win: Single<&Window, With<PrimaryWindow>>,
    q_splash_roots: Query<Entity, (With<SplashUi>, Without<bevy::prelude::ChildOf>)>,
    mut lock: ResMut<PlayerControlLock>,
    mut music_mode: ResMut<MusicMode>,
) {
    if *step == SplashStep::Done {
        return;
    }

    let Some(imgs) = imgs else { return; };

    match *step {
        SplashStep::Splash0 => {
            let any_key = keys.get_just_pressed().next().is_some();
            let left_click = mouse.just_pressed(MouseButton::Left);
            if !any_key && !left_click {
                return;
            }

            *step = SplashStep::Splash1;

            for e in q_splash_roots.iter() {
                commands.entity(e).despawn();
            }

            let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
            spawn_splash_ui(&mut commands, imgs.splash1.clone(), w, h);
        }

        SplashStep::Splash1 => {
            let any_key = keys.get_just_pressed().next().is_some();
            let left_click = mouse.just_pressed(MouseButton::Left);
            if !any_key && !left_click {
                return;
            }

            *step = SplashStep::Menu;

            for e in q_splash_roots.iter() {
                commands.entity(e).despawn();
            }

            // Hard cut to menu music
            music_mode.0 = MusicModeKind::Menu;

            let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
            spawn_splash_ui(&mut commands, imgs.menu.clone(), w, h);
            spawn_menu_hint(&mut commands, &asset_server);
        }

        SplashStep::Menu => {
            if !keys.just_pressed(KeyCode::Enter) {
                return;
            }

            *step = SplashStep::Done;

            for e in q_splash_roots.iter() {
                commands.entity(e).despawn();
            }

            // Enter gameplay
            lock.0 = false;
            music_mode.0 = MusicModeKind::Gameplay;
        }

        SplashStep::Done => {}
    }
}

fn splash_resize_on_window_change(
    mut ev: MessageReader<WindowResized>,
    step: Res<SplashStep>,
    mut q_node: Query<&mut Node, With<SplashImage>>,
) {
    if *step == SplashStep::Done {
        return;
    }

    let Some(last) = ev.read().last() else {
        return;
    };

    let (w, h) = compute_scaled_size(last.width, last.height);
    for mut n in q_node.iter_mut() {
        n.width = Val::Px(w);
        n.height = Val::Px(h);
    }
}

fn spawn_menu_hint(commands: &mut Commands, asset_server: &AssetServer) {
    let ui_font: Handle<Font> = asset_server.load("fonts/honda_font.ttf");

    commands
        .spawn((
            SplashUi,
            MenuHint,
            ZIndex(1001),
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(10.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("PRESS ENTER TO START"),
                TextFont {
                    font: ui_font,
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
}
