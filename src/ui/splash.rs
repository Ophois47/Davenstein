/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowResized};

use davelib::player::PlayerControlLock;

// Both should be authored for a 320x200 base resolution
pub const SPLASH_0_PATH: &str = "textures/ui/splash0.png";
pub const SPLASH_1_PATH: &str = "textures/ui/title_screen.png";

// Used to compute a clean integer UI scale
const BASE_W: f32 = 320.0;
const BASE_H: f32 = 200.0;

#[derive(Component)]
pub struct SplashUi;

#[derive(Component)]
struct SplashImage;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
enum SplashStep {
    First,
    Second,
    Done,
}

#[derive(Resource)]
struct SplashImages {
    second: Handle<Image>,
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

impl Default for SplashStep {
    fn default() -> Self {
        SplashStep::First
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
    // Freeze gameplay input while splash is up
    lock.0 = true;

    // Ensure mouse is released and visible
    cursor.visible = true;
    cursor.grab_mode = CursorGrabMode::None;

    // Load images
    let first = asset_server.load(SPLASH_0_PATH);
    let second = asset_server.load(SPLASH_1_PATH);

    // We only need to keep the second handle for the step-2 swap
    commands.insert_resource(SplashImages {
        second: second.clone(),
    });

    // Spawn first splash immediately
    let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
    spawn_splash_ui(&mut commands, first, w, h);
}

fn spawn_splash_ui(commands: &mut Commands, image: Handle<Image>, w: f32, h: f32) {
    // Fullscreen black backdrop + centered art
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
) {
    if *step == SplashStep::Done {
        return;
    }

    let any_key = keys.get_just_pressed().next().is_some();
    let any_mouse = mouse.get_just_pressed().next().is_some();
    if !any_key && !any_mouse {
        return;
    }

    let Some(imgs) = imgs else {
        return;
    };

    match *step {
        SplashStep::First => {
            *step = SplashStep::Second;

            for e in q_splash.iter() {
                commands.entity(e).despawn();
            }

            let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
            spawn_splash_ui(&mut commands, imgs.second.clone(), w, h);
        }
        SplashStep::Second => {
            *step = SplashStep::Done;

            for e in q_splash.iter() {
                commands.entity(e).despawn();
            }

            lock.0 = false;
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
