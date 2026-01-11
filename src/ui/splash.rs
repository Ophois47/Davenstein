/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use bevy::ecs::system::SystemParam;

use davelib::audio::{MusicMode, MusicModeKind, PlaySfx, SfxKind};
use davelib::player::PlayerControlLock;

#[derive(SystemParam)]
struct SplashAdvanceQueries<'w, 's> {
    q_win: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    q_splash_roots: Query<'w, 's, Entity, With<SplashUi>>,

    q_node: Query<'w, 's, &'static mut Node, (With<MenuCursor>, Without<EpisodeHighlight>)>,

    q_cursor_light: Query<
        'w,
        's,
        &'static mut Visibility,
        (With<MenuCursorLight>, Without<MenuCursorDark>),
    >,
    q_cursor_dark: Query<
        'w,
        's,
        &'static mut Visibility,
        (With<MenuCursorDark>, Without<MenuCursorLight>),
    >,

    q_episode_items: Query<'w, 's, (&'static EpisodeItem, &'static mut TextColor)>,

    q_episode_hilite: Query<
        'w,
        's,
        &'static mut Node,
        (With<EpisodeHighlight>, Without<MenuCursor>),
    >,
}

pub const SPLASH_0_PATH: &str = "textures/ui/splash0.png";
pub const SPLASH_1_PATH: &str = "textures/ui/splash1.png";
pub const MAIN_MENU_PATH: &str = "textures/ui/main_menu.png";
pub const GET_PSYCHED_PATH: &str = "textures/ui/get_psyched.png";
pub const MENU_BANNER_PATH: &str = "textures/ui/menu_banner.png";
pub const MENU_HINT_PATH: &str = "textures/ui/menu_hint_move_select_back.png";
pub const MENU_CURSOR_LIGHT_PATH: &str = "textures/ui/menu_cursor_light.png";
pub const MENU_CURSOR_DARK_PATH: &str = "textures/ui/menu_cursor_dark.png";

// Episode Selection Screen
const EPISODE_THUMBS_ATLAS_PATH: &str = "textures/ui/episode_thumbs_atlas.png";

// 48x24 Cells, Arranged 3x2 in Atlas Image
const EP_THUMB_W: f32 = 48.0;
const EP_THUMB_H: f32 = 24.0;

// Episode Layout in "Native" 320x200 Coords
const EP_TITLE_TOP: f32 = 10.0;
const EP_LIST_TOP: f32 = 32.0;
const EP_ROW_H: f32 = 26.0;

const EP_THUMB_X: f32 = 24.0;
const EP_TEXT_X: f32 = 88.0;

// Highlight Behind Selected Row
const EP_HILITE_X: f32 = 76.0;
const EP_HILITE_W: f32 = 220.0;
const EP_HILITE_H: f32 = 20.0;

// Loading Screen
const BASE_HUD_H: f32 = 44.0; // Status Bar Area
const PSYCHED_DURATION_SECS: f32 = 1.2;
const PSYCHED_SPR_W: f32 = 220.0;
const PSYCHED_SPR_H: f32 = 40.0;

// Wolfenstein 3D-Like Teal + Red Bar
const PSYCHED_TEAL: Color = Color::srgb(0.00, 0.55, 0.55);
const PSYCHED_RED: Color = Color::srgb(0.80, 0.00, 0.00);

// Used to Compute Clean Integer UI Scale
const BASE_W: f32 = 320.0;
const BASE_H: f32 = 200.0;

// Cursor positioning in the native (unscaled) menu art.
// These match the Y values you hard-coded in spawn_menu_hint().
const MENU_CURSOR_TOP: f32 = 94.0;
const MENU_ITEM_H: f32 = 20.0;
const MENU_ACTIONS: [MenuAction; 2] = [MenuAction::NewGame, MenuAction::Quit];

#[derive(Component)]
pub struct SplashUi;

#[derive(Component)]
struct SplashImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Resource)]
pub enum SplashStep {
    Splash0,
    Splash1,
    Menu,
    EpisodeSelect,
    Done,
}

#[derive(Resource)]
struct SplashImages {
    splash0: Handle<Image>,
    splash1: Handle<Image>,
    menu: Handle<Image>,
    episode_thumbs_atlas: Handle<Image>,
}

#[derive(Default)]
struct EpisodeLocalState {
    selection: usize,
}

#[derive(Component)]
struct EpisodeItem {
    idx: usize,
}

#[derive(Component)]
struct EpisodeHighlight;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    NewGame,
    Quit,
}

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

impl Default for SplashStep {
    fn default() -> Self {
        SplashStep::Splash0
    }
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
        app.add_systems(Update, splash_advance_on_any_input);
        app.add_systems(Update, auto_get_psyched_on_level_start);
        app.add_systems(Update, tick_get_psyched_loading);
        app.add_systems(Update, splash_resize_on_window_change);
    }
}

fn compute_scaled_size(win_w: f32, win_h: f32) -> (f32, f32) {
    let scale = (win_w / BASE_W).min(win_h / BASE_H).floor().max(1.0);
    (BASE_W * scale, BASE_H * scale)
}

fn spawn_episode_select_ui(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    w: f32,
    h: f32,
    scale: f32,
    imgs: &SplashImages,
    selection: usize,
) {
    let ui_font: Handle<Font> = asset_server.load("fonts/honda_font.ttf");
    let hint_strip: Handle<Image> = asset_server.load(MENU_HINT_PATH);

    // Full-screen root (opaque) so the world doesn't show around the 320x200 canvas.
    let root = commands
        .spawn((
            SplashUi,
            ZIndex(1001),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .id();

    let canvas = commands
        .spawn((
            SplashUi,
            Node {
                width: Val::Px(w),
                height: Val::Px(h),
                position_type: PositionType::Relative,
                ..default()
            },
            BackgroundColor(Color::srgb(0.55, 0.0, 0.0)),
            ChildOf(root),
        ))
        .id();

    // Title
    commands.spawn((
        Text::new("Which episode to play?"),
        TextFont {
            font: ui_font.clone(),
            font_size: (24.0 * scale).round(),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px((EP_TITLE_TOP * scale).round()),
            left: Val::Px((44.0 * scale).round()),
            ..default()
        },
        ChildOf(canvas),
    ));

    // Highlight bar
    let hilite_top = (EP_LIST_TOP + selection as f32 * EP_ROW_H + 2.0) * scale;
    commands.spawn((
        EpisodeHighlight,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px((EP_HILITE_X * scale).round()),
            top: Val::Px(hilite_top.round()),
            width: Val::Px((EP_HILITE_W * scale).round()),
            height: Val::Px((EP_HILITE_H * scale).round()),
            ..default()
        },
        BackgroundColor(Color::srgb(0.65, 0.65, 0.65)),
        ChildOf(canvas),
    ));

    // Episodes (DOS ordering)
    const EP_TEXT: [&str; 6] = [
        "Episode 1\nEscape from Wolfenstein",
        "Episode 2\nOperation: Eisenfaust",
        "Episode 3\nDie, Fuhrer, Die!",
        "Episode 4\nA Dark Secret",
        "Episode 5\nTrail of the Madman",
        "Episode 6\nConfrontation",
    ];

    for idx in 0..6 {
        let row_top = (EP_LIST_TOP + idx as f32 * EP_ROW_H) * scale;

        // Thumb (cropped from atlas)
        let col = (idx % 3) as f32;
        let row = (idx / 3) as f32;

        let rect = Rect::from_corners(
            Vec2::new(col * EP_THUMB_W, row * EP_THUMB_H),
            Vec2::new((col + 1.0) * EP_THUMB_W, (row + 1.0) * EP_THUMB_H),
        );

        let mut img = ImageNode::new(imgs.episode_thumbs_atlas.clone());
        img.rect = Some(rect);

        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((EP_THUMB_X * scale).round()),
                top: Val::Px(row_top.round()),
                width: Val::Px((EP_THUMB_W * scale).round()),
                height: Val::Px((EP_THUMB_H * scale).round()),
                ..default()
            },
            img,
            ChildOf(canvas),
        ));

        // Text
        let is_selected = idx == selection;
        commands.spawn((
            EpisodeItem { idx },
            Text::new(EP_TEXT[idx]),
            TextFont {
                font: ui_font.clone(),
                font_size: (18.0 * scale).round(),
                ..default()
            },
            TextColor(if is_selected {
                Color::srgb(1.0, 1.0, 1.0)
            } else {
                Color::srgb(0.75, 0.75, 0.75)
            }),
            TextLayout::new_with_justify(Justify::Left),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((EP_TEXT_X * scale).round()),
                top: Val::Px((row_top - (2.0 * scale)).round()),
                ..default()
            },
            ChildOf(canvas),
        ));
    }

    // Bottom hint strip: reuse the same art as the main menu.
    // Hint strip is 120x23, positioned same as spawn_menu_hint().
    let hint_w = (120.0 * scale).round();
    let hint_h = (23.0 * scale).round();
    let hint_x = ((BASE_W - 120.0) * 0.5 * scale).round();
    let hint_y = ((BASE_H - 23.0 - 6.0) * scale).round();

    commands.spawn((
        ImageNode::new(hint_strip),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(hint_x),
            top: Val::Px(hint_y),
            width: Val::Px(hint_w),
            height: Val::Px(hint_h),
            ..default()
        },
        ChildOf(canvas),
    ));
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
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut step: ResMut<SplashStep>,
    imgs: Option<Res<SplashImages>>,
    mut lock: ResMut<PlayerControlLock>,
    mut music_mode: ResMut<MusicMode>,
    mut psyched: ResMut<PsychedLoad>,
    mut menu: Local<MenuLocalState>,
    mut episode: Local<EpisodeLocalState>,
    mut sfx: MessageWriter<PlaySfx>,
    mut app_exit: MessageWriter<bevy::app::AppExit>,
    mut q: SplashAdvanceQueries,
) {
    let Some(win) = q.q_win.iter().next() else {
        return;
    };

    let (w, h) = compute_scaled_size(win.width(), win.height());
    let scale = w / BASE_W;

    let any_key = keyboard.get_just_pressed().len() > 0 || mouse.get_just_pressed().len() > 0;

    match *step {
        SplashStep::Splash0 => {
            lock.0 = true;
            music_mode.0 = MusicModeKind::Splash;

            let Some(imgs) = imgs.as_ref() else { return };

            // Ensure splash0 is shown (no input needed to display it).
            if q.q_splash_roots.iter().next().is_none() {
                spawn_splash_ui(&mut commands, imgs.splash0.clone(), w, h);
            }

            // Input advances to splash1.
            if any_key {
                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }
                spawn_splash_ui(&mut commands, imgs.splash1.clone(), w, h);
                *step = SplashStep::Splash1;
            }
        }

        SplashStep::Splash1 => {
            lock.0 = true;
            music_mode.0 = MusicModeKind::Splash;

            let Some(imgs) = imgs.as_ref() else { return };

            // Safety: if we somehow enter Splash1 with no UI, show it.
            if q.q_splash_roots.iter().next().is_none() {
                spawn_splash_ui(&mut commands, imgs.splash1.clone(), w, h);
            }

            // Input advances to main menu.
            if any_key {
                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }
                spawn_splash_ui(&mut commands, imgs.menu.clone(), w, h);
                spawn_menu_hint(&mut commands, &asset_server, w, h);
                menu.reset();
                *step = SplashStep::Menu;
            }
        }

        SplashStep::Menu => {
            lock.0 = true;
            music_mode.0 = MusicModeKind::Menu;

            let blink_on = (time.elapsed_secs() / 0.2).floor() as i32 % 2 == 0;
            let top = ((MENU_CURSOR_TOP + menu.selection as f32 * MENU_ITEM_H) * scale).round();

            // Move BOTH cursor entities so blink doesn't "jump".
            for mut node in q.q_node.iter_mut() {
                node.top = Val::Px(top);
            }

            for mut v in q.q_cursor_light.iter_mut() {
                *v = if blink_on { Visibility::Visible } else { Visibility::Hidden };
            }
            for mut v in q.q_cursor_dark.iter_mut() {
                *v = if blink_on { Visibility::Hidden } else { Visibility::Visible };
            }

            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                if menu.selection > 0 {
                    menu.selection -= 1;
                } else {
                    menu.selection = MENU_ACTIONS.len() - 1;
                }
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
            }

            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                menu.selection = (menu.selection + 1) % MENU_ACTIONS.len();
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
            }

            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
                sfx.write(PlaySfx { kind: SfxKind::MenuSelect, pos: Vec3::ZERO });

                match MENU_ACTIONS[menu.selection] {
                    MenuAction::NewGame => {
                        for e in q.q_splash_roots.iter() {
                            commands.entity(e).despawn();
                        }

                        episode.selection = 0;
                        if let Some(imgs) = imgs.as_ref() {
                            spawn_episode_select_ui(
                                &mut commands,
                                &asset_server,
                                w,
                                h,
                                scale,
                                imgs,
                                episode.selection,
                            );
                            *step = SplashStep::EpisodeSelect;
                        }
                    }
                    MenuAction::Quit => {
                        app_exit.write(bevy::app::AppExit::Success);
                    }
                }
            }
        }

        SplashStep::EpisodeSelect => {
            lock.0 = true;
            music_mode.0 = MusicModeKind::Menu;

            if keyboard.just_pressed(KeyCode::Escape) {
                sfx.write(PlaySfx { kind: SfxKind::MenuBack, pos: Vec3::ZERO });

                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }

                if let Some(imgs) = imgs.as_ref() {
                    spawn_splash_ui(&mut commands, imgs.menu.clone(), w, h);
                    spawn_menu_hint(&mut commands, &asset_server, w, h);
                    menu.reset();
                    *step = SplashStep::Menu;
                }
                return;
            }

            let mut moved = false;

            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                if episode.selection > 0 {
                    episode.selection -= 1;
                } else {
                    episode.selection = 5;
                }
                moved = true;
            }

            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                episode.selection = (episode.selection + 1) % 6;
                moved = true;
            }

            if moved {
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
            }

            for (item, mut color) in q.q_episode_items.iter_mut() {
                *color = TextColor(if item.idx == episode.selection {
                    Color::srgb(1.0, 1.0, 1.0)
                } else {
                    Color::srgb(0.75, 0.75, 0.75)
                });
            }

            if let Some(mut node) = q.q_episode_hilite.iter_mut().next() {
                let hilite_top = (EP_LIST_TOP + episode.selection as f32 * EP_ROW_H + 2.0) * scale;
                node.top = Val::Px(hilite_top.round());
            }

            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
                if episode.selection == 0 {
                    sfx.write(PlaySfx { kind: SfxKind::MenuSelect, pos: Vec3::ZERO });

                    for e in q.q_splash_roots.iter() {
                        commands.entity(e).despawn();
                    }

                    begin_get_psyched_loading(
                        &mut commands,
                        &asset_server,
                        win,
                        &mut *psyched,
                        &mut *lock,
                        &mut *music_mode,
                    );

                    // keep your existing behavior here:
                    lock.0 = false;
                    music_mode.0 = MusicModeKind::Gameplay;

                    *step = SplashStep::Done;
                } else {
                    sfx.write(PlaySfx { kind: SfxKind::NoWay, pos: Vec3::ZERO });
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

pub(crate) fn setup_splash(mut commands: Commands, asset_server: Res<AssetServer>) {
    let splash0 = asset_server.load(SPLASH_0_PATH);
    let splash1 = asset_server.load(SPLASH_1_PATH);
    let menu = asset_server.load(MAIN_MENU_PATH);
    let episode_thumbs_atlas = asset_server.load(EPISODE_THUMBS_ATLAS_PATH);

    commands.insert_resource(SplashImages {
        splash0,
        splash1,
        menu,
        episode_thumbs_atlas,
    });
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
            MenuHint,     // keep your existing marker so this stays "menu UI"
            ZIndex(1001), // above the splash/menu background
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

fn spawn_get_psyched_ui(commands: &mut Commands, asset_server: &AssetServer, win_w: f32, win_h: f32) {
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
    if *step != SplashStep::Done {
        let ready = grid.is_some() && solid.is_some() && markers.is_some();
        *last_ready = ready;
        return;
    }

    let ready = grid.is_some() && solid.is_some() && markers.is_some();
    let ready_rise = ready && !*last_ready;
    *last_ready = ready;

    let level_changed = level.is_changed();

    if psyched.active {
        return;
    }

    if level_changed || ready_rise {
        let win: &Window = q_win.into_inner();
        begin_get_psyched_loading(
            &mut commands,
            &asset_server,
            win,
            &mut *psyched,
            &mut *lock,
            &mut *music_mode,
        );
    }
}
