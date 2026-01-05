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
    mut cursor: Single<&mut CursorOptions>,
    q_win: Single<&Window, With<PrimaryWindow>>,
) {
    // Freeze Gameplay Input While Splash / Menu Up
    lock.0 = true;

    // Ensure Mouse Released + Visible
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;

    // Load Images
    let splash0 = asset_server.load(SPLASH_0_PATH);
    let splash1 = asset_server.load(SPLASH_1_PATH);
    let menu = asset_server.load(MAIN_MENU_PATH);

    commands.insert_resource(SplashImages {
        splash1: splash1.clone(),
        menu: menu.clone(),
    });

    let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
    spawn_splash_ui(&mut commands, splash0, w, h);
}

fn spawn_splash_ui(commands: &mut Commands, image: Handle<Image>, w: f32, h: f32) {
    // Fullscreen Black Backdrop + Centered Art
    commands
        .spawn((
            SplashUi,
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
                SplashUi,
                SplashImage,
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
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut step: ResMut<SplashStep>,
    imgs: Option<Res<SplashImages>>,
    q_win: Single<&Window, With<PrimaryWindow>>,
    q_splash: Query<Entity, With<SplashUi>>,
    mut lock: ResMut<PlayerControlLock>,
    mut new_game: ResMut<super::sync::NewGameRequested>,
    mut music_mode: ResMut<davelib::audio::MusicMode>,
) {
    if *step == SplashStep::Done {
        return;
    }

    let any_key = keys.get_just_pressed().next().is_some();
    let left_click = mouse.just_pressed(MouseButton::Left);

    let Some(imgs) = imgs else { return; };

    match *step {
        SplashStep::Splash0 => {
            if !any_key && !left_click { return; }
            *step = SplashStep::Splash1;

            for e in q_splash.iter() {
                commands.entity(e).despawn();
            }

            let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
            spawn_splash_ui(&mut commands, imgs.splash1.clone(), w, h);
        }
        SplashStep::Splash1 => {
            if !any_key && !left_click { return; }
            *step = SplashStep::Menu;

            for e in q_splash.iter() {
                commands.entity(e).despawn();
            }

            // Switch to Menu Music on Entry
            music_mode.0 = davelib::audio::MusicModeKind::Menu;

            let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
            spawn_splash_ui(&mut commands, imgs.menu.clone(), w, h);
        }
        SplashStep::Menu => {
            // Enter Starts Game
            if !keys.just_pressed(KeyCode::Enter) { return; }

            *step = SplashStep::Done;

            for e in q_splash.iter() {
                commands.entity(e).despawn();
            }

            // Start Fresh Run + Allow Gameplay Input
            new_game.0 = true;
            lock.0 = false;

            // Hand Off to Gameplay Music (Level Music System)
            music_mode.0 = davelib::audio::MusicModeKind::Gameplay;
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
