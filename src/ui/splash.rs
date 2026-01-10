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
pub const GET_PSYCHED_PATH: &str = "textures/ui/get_psyched.png";
pub const MENU_BANNER_PATH: &str = "textures/ui/menu_banner.png";
pub const MENU_HINT_PATH: &str = "textures/ui/menu_hint_move_select_back.png";
pub const MENU_CURSOR_LIGHT_PATH: &str = "textures/ui/menu_cursor_light.png";
pub const MENU_CURSOR_DARK_PATH: &str = "textures/ui/menu_cursor_dark.png";

// Loading Screen
const BASE_HUD_H: f32 = 44.0;   // Status Bar Area
const PSYCHED_DURATION_SECS: f32 = 1.2;
const PSYCHED_SPR_W: f32 = 220.0;
const PSYCHED_SPR_H: f32 = 40.0;

// Wolfenstein 3D-Like Teal + Red Bar
const PSYCHED_TEAL: Color = Color::srgb(0.00, 0.55, 0.55);
const PSYCHED_RED: Color = Color::srgb(0.80, 0.00, 0.00);

// Used to Compute Clean Integer UI Scale
const BASE_W: f32 = 320.0;
const BASE_H: f32 = 200.0;

#[derive(Component)]
pub struct SplashUi;

#[derive(Component)]
struct SplashImage;

#[derive(Component)]
struct MenuHint;

#[derive(Component)]
struct LoadingUi;

#[derive(Component)]
struct MenuCursor;

#[derive(Component)]
struct MenuCursorLight;

#[derive(Component)]
struct MenuCursorDark;

#[derive(Component)]
struct PsychedBar {
    target_w: f32,
}

#[derive(Resource)]
struct PsychedLoad {
    timer: Timer,
    active: bool,
}

impl Default for PsychedLoad {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(PSYCHED_DURATION_SECS, TimerMode::Once),
            active: false,
        }
    }
}

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

#[derive(Default)]
struct MenuLocalState {
    selection: usize, // 0 = New Game, 1 = Quit
    blink: Timer,
    blink_light: bool,
}

impl MenuLocalState {
    fn reset(&mut self) {
        self.selection = 0;
        self.blink = Timer::from_seconds(0.12, TimerMode::Repeating);
        self.blink_light = true;
    }
}

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashStep>();
        app.init_resource::<PsychedLoad>();
        app.add_systems(
            Update,
            (
                splash_advance_on_any_input,
                auto_get_psyched_on_level_start,
                tick_get_psyched_loading,
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
    asset_server: Res<AssetServer>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut step: ResMut<SplashStep>,
    imgs: Option<Res<SplashImages>>,
    q_win: Single<&Window, With<PrimaryWindow>>,
    q_splash_roots: Query<Entity, (With<SplashUi>, Without<bevy::prelude::ChildOf>)>,
    mut lock: ResMut<PlayerControlLock>,
    mut music_mode: ResMut<MusicMode>,
    mut psyched: ResMut<PsychedLoad>,

    // menu-local state you added
    mut menu: Local<MenuLocalState>,

    // quit request (canonical Bevy way)
    mut app_exit: MessageWriter<bevy::app::AppExit>,

    // move both cursor nodes (light+dark) together
    mut q_cursor_nodes: Query<&mut Node, With<MenuCursor>>,

    // IMPORTANT: resolve &mut Visibility borrow conflict
    mut vis_set: bevy::ecs::system::ParamSet<(
        Query<&mut Visibility, With<MenuCursorLight>>,
        Query<&mut Visibility, With<MenuCursorDark>>,
    )>,
) {
    if *step == SplashStep::Done {
        return;
    }
    let Some(imgs) = imgs else { return; };

    match *step {
        SplashStep::Splash0 => {
            let any_key = keys.get_just_pressed().next().is_some();
            let left_click = mouse.just_pressed(MouseButton::Left);
            if !any_key && !left_click { return; }

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
            if !any_key && !left_click { return; }

            *step = SplashStep::Menu;
            menu.reset();

            for e in q_splash_roots.iter() {
                commands.entity(e).despawn();
            }

            // Hard Cut to Menu Music
            music_mode.0 = MusicModeKind::Menu;

            let (w, h) = compute_scaled_size(q_win.width(), q_win.height());
            spawn_splash_ui(&mut commands, imgs.menu.clone(), w, h);

            // If you updated this signature to accept w/h, keep it consistent here.
            spawn_menu_hint(&mut commands, &asset_server, w, h);
        }

        SplashStep::Menu => {
            // Blink cursor
            menu.blink.tick(time.delta());
            if menu.blink.just_finished() {
                menu.blink_light = !menu.blink_light;

                // LIGHT
                for mut v in vis_set.p0().iter_mut() {
                    *v = if menu.blink_light { Visibility::Visible } else { Visibility::Hidden };
                }
                // DARK
                for mut v in vis_set.p1().iter_mut() {
                    *v = if menu.blink_light { Visibility::Hidden } else { Visibility::Visible };
                }
            }

            // ESC exits immediately (per your requirement)
            if keys.just_pressed(KeyCode::Escape) {
                app_exit.write(bevy::app::AppExit::Success);
                return;
            }

            const ITEM_COUNT: usize = 2;

            let moved_up = keys.just_pressed(KeyCode::ArrowUp);
            let moved_down = keys.just_pressed(KeyCode::ArrowDown);

            if moved_up {
                menu.selection = (menu.selection + ITEM_COUNT - 1) % ITEM_COUNT;
            } else if moved_down {
                menu.selection = (menu.selection + 1) % ITEM_COUNT;
            }

            if moved_up || moved_down {
                let (w, _h) = compute_scaled_size(q_win.width(), q_win.height());
                let scale = (w / BASE_W).round().max(1.0);

                let y = match menu.selection {
                    0 => (94.0 * scale).round(),   // NEW GAME
                    1 => (114.0 * scale).round(),  // QUIT
                    _ => (94.0 * scale).round(),
                };

                for mut n in q_cursor_nodes.iter_mut() {
                    n.top = Val::Px(y);
                }
            }

            if keys.just_pressed(KeyCode::Enter) {
                match menu.selection {
                    0 => {
                        // NEW GAME (unchanged behavior)
                        for e in q_splash_roots.iter() {
                            commands.entity(e).despawn();
                        }

                        *step = SplashStep::Done;

                        begin_get_psyched_loading(
                            &mut commands,
                            &asset_server,
                            &q_win,
                            &mut psyched,
                            &mut lock,
                            &mut music_mode,
                        );
                    }
                    1 => {
                        // QUIT
                        app_exit.write(bevy::app::AppExit::Success);
                    }
                    _ => {}
                }
            }
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

fn spawn_menu_hint(commands: &mut Commands, asset_server: &AssetServer, w: f32, h: f32) {
    let ui_font: Handle<Font> = asset_server.load("fonts/honda_font.ttf");

    let banner = asset_server.load(MENU_BANNER_PATH);
    let hint = asset_server.load(MENU_HINT_PATH);
    let cursor_light = asset_server.load(MENU_CURSOR_LIGHT_PATH);
    let cursor_dark = asset_server.load(MENU_CURSOR_DARK_PATH);

    // w/h are already integer-scaled 320x200. Recover the integer UI scale.
    let scale = (w / BASE_W).round().max(1.0);

    // Native (320x200) layout coords, scaled up by `scale`.
    // Banner is 156x52 in the extracted sheet.
    let banner_w = 156.0 * scale;
    let banner_h = 52.0 * scale;
    let banner_x = ((BASE_W - 156.0) * 0.5 * scale).round();
    let banner_y = (6.0 * scale).round();

    // Hint strip is 120x23.
    let hint_w = 120.0 * scale;
    let hint_h = 23.0 * scale;
    let hint_x = ((BASE_W - 120.0) * 0.5 * scale).round();
    let hint_y = ((BASE_H - 23.0 - 6.0) * scale).round();

    // Menu items (native positions)
    let x_text = (150.0 * scale).round();
    let y_new_game = (92.0 * scale).round();
    let y_quit = (112.0 * scale).round();

    // Cursor is 19x10.
    let cursor_w = 19.0 * scale;
    let cursor_h = 10.0 * scale;
    let cursor_x = (128.0 * scale).round();
    let cursor_y = (94.0 * scale).round(); // aligned to NEW GAME line

    let font_size = (18.0 * scale).round();

    commands
        .spawn((
            SplashUi,
            MenuHint,      // keep your existing marker so this stays "menu UI"
            ZIndex(1001),  // above the splash/menu background
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
            BackgroundColor(Color::NONE),
        ))
        .with_children(|root| {
            // Inner canvas sized to the same scaled 320x200 as the background image.
            root.spawn((
                Node {
                    width: Val::Px(w),
                    height: Val::Px(h),
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|c| {
                // Banner
                c.spawn((
                    ImageNode::new(banner),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(banner_x),
                        top: Val::Px(banner_y),
                        width: Val::Px(banner_w),
                        height: Val::Px(banner_h),
                        ..default()
                    },
                ));

                // Hint strip (red MOVE/SELECT/ESC BACK box)
                c.spawn((
                    ImageNode::new(hint),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(hint_x),
                        top: Val::Px(hint_y),
                        width: Val::Px(hint_w),
                        height: Val::Px(hint_h),
                        ..default()
                    },
                ));

                // Menu text items
                c.spawn((
                    Text::new("NEW GAME"),
                    TextFont {
                        font: ui_font.clone(),
                        font_size,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(x_text),
                        top: Val::Px(y_new_game),
                        ..default()
                    },
                ));

                c.spawn((
                    Text::new("QUIT"),
                    TextFont {
                        font: ui_font.clone(),
                        font_size,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(x_text),
                        top: Val::Px(y_quit),
                        ..default()
                    },
                ));

                // Cursor (light + dark entities stacked; we toggle Visibility to "blink")
                c.spawn((
                    MenuCursor,
                    MenuCursorLight,
                    Visibility::Visible,
                    ImageNode::new(cursor_light),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(cursor_x),
                        top: Val::Px(cursor_y),
                        width: Val::Px(cursor_w),
                        height: Val::Px(cursor_h),
                        ..default()
                    },
                ));
                c.spawn((
                    MenuCursor,
                    MenuCursorDark,
                    Visibility::Hidden,
                    ImageNode::new(cursor_dark),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(cursor_x),
                        top: Val::Px(cursor_y),
                        width: Val::Px(cursor_w),
                        height: Val::Px(cursor_h),
                        ..default()
                    },
                ));
            });
        });
}

fn spawn_get_psyched_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    win_w: f32,
    win_h: f32,
) {
    // IMPORTANT: match the HUD sizing rules exactly (see ui/hud.rs).
    // HUD scale is an integer derived from WINDOW WIDTH, and the status bar is 44px * scale.
    const HUD_W: f32 = 320.0;

    let hud_scale = (win_w / HUD_W).floor().max(1.0);
    let hud_h = (BASE_HUD_H * hud_scale).round();
    let view_h = (win_h - hud_h).max(0.0);

    // Banner uses the same integer HUD scale by default, but clamps so it never exceeds the window.
    let mut scale = hud_scale.max(1.0);
    let mut spr_w = (PSYCHED_SPR_W * scale).round();
    let mut spr_h = (PSYCHED_SPR_H * scale).round();
    if spr_w > win_w {
        scale = (win_w / PSYCHED_SPR_W).max(1.0);
        spr_w = (PSYCHED_SPR_W * scale).round();
        spr_h = (PSYCHED_SPR_H * scale).round();
    }

    let banner = asset_server.load(GET_PSYCHED_PATH);

    // Center banner in the view region (which ends above the HUD strip).
    let left = ((win_w - spr_w) * 0.5).round().max(0.0);
    let top = ((view_h - spr_h) * 0.5).round().max(0.0);

    // Bar goes ACROSS THE BOTTOM OF THE BANNER
    let bar_h = (1.0 * scale).max(1.0).round();
    let bar_top = (top + spr_h - bar_h).max(0.0);

    // Root ONLY covers the view region, so it can never overlap the HUD.
    commands
        .spawn((
            LoadingUi,
            ZIndex(950),
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(view_h),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(PSYCHED_TEAL),
        ))
        .with_children(|root| {
            // Banner
            root.spawn((
                ImageNode::new(banner),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left),
                    top: Val::Px(top),
                    width: Val::Px(spr_w),
                    height: Val::Px(spr_h),
                    ..default()
                },
            ));

            // Progress bar (grows to the banner width)
            root.spawn((
                PsychedBar { target_w: spr_w },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left),
                    top: Val::Px(bar_top),
                    width: Val::Px(0.0),
                    height: Val::Px(bar_h),
                    ..default()
                },
                BackgroundColor(PSYCHED_RED),
            ));
        });
}

fn begin_get_psyched_loading(
    commands: &mut Commands,
    asset_server: &AssetServer,
    win: &Window,
    psyched: &mut PsychedLoad,
    lock: &mut PlayerControlLock,
    music_mode: &mut MusicMode,
) {
    // Lock gameplay controls during the fake load
    lock.0 = true;

    // IMPORTANT: start gameplay music DURING the loading overlay
    music_mode.0 = MusicModeKind::Gameplay;

    // Start timer + spawn overlay
    psyched.active = true;
    psyched.timer.reset();
    spawn_get_psyched_ui(commands, asset_server, win.width(), win.height());
}

fn tick_get_psyched_loading(
    mut commands: Commands,
    time: Res<Time>,
    mut lock: ResMut<PlayerControlLock>,
    mut psyched: ResMut<PsychedLoad>,
    q_loading_roots: Query<Entity, (With<LoadingUi>, Without<bevy::prelude::ChildOf>)>,
    mut q_bar: Query<(&mut Node, &PsychedBar)>,
) {
    if !psyched.active {
        return;
    }

    psyched.timer.tick(time.delta());

    let t = (psyched.timer.elapsed_secs() / psyched.timer.duration().as_secs_f32()).clamp(0.0, 1.0);

    if let Some((mut node, bar)) = q_bar.iter_mut().next() {
        node.width = Val::Px((bar.target_w * t).floor());
    }

    if psyched.timer.is_finished() && psyched.timer.just_finished() {
        for e in q_loading_roots.iter() {
            commands.entity(e).despawn();
        }

        psyched.active = false;
        lock.0 = false;
    }
}

fn auto_get_psyched_on_level_start(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_win: Single<&Window, With<PrimaryWindow>>,
    step: Res<SplashStep>,
    level: Res<davelib::level::CurrentLevel>,
    grid: Option<Res<davelib::map::MapGrid>>,
    solid: Option<Res<davelib::decorations::SolidStatics>>,
    markers: Option<Res<davelib::pushwalls::PushwallMarkers>>,
    mut last_ready: Local<bool>,
    mut psyched: ResMut<PsychedLoad>,
    mut lock: ResMut<PlayerControlLock>,
    mut music_mode: ResMut<MusicMode>,
) {
    // Only do this during gameplay (not while still in splash/menu)
    if *step != SplashStep::Done {
        let ready = grid.is_some() && solid.is_some() && markers.is_some();
        *last_ready = ready;
        return;
    }

    let ready = grid.is_some() && solid.is_some() && markers.is_some();
    let ready_rise = ready && !*last_ready;
    *last_ready = ready;

    // Any "new level just became active" signal
    let level_changed = level.is_changed();

    if psyched.active {
        return;
    }

    if level_changed || ready_rise {
        begin_get_psyched_loading(
            &mut commands,
            &asset_server,
            &q_win,
            &mut psyched,
            &mut lock,
            &mut music_mode,
        );
    }
}
