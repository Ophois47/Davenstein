/*
Davenstein - by David Petnick
*/
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};

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
    q_episode_items: Query<
        'w,
        's,
        (&'static EpisodeItem, &'static EpisodeTextVariant, &'static mut Visibility),
        (Without<MenuCursorLight>, Without<MenuCursorDark>),
    >,
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
pub const MENU_HINT_PATH: &str = "textures/ui/menu_hint.png";
pub const MENU_CURSOR_LIGHT_PATH: &str = "textures/ui/menu_cursor_light.png";
pub const MENU_CURSOR_DARK_PATH: &str = "textures/ui/menu_cursor_dark.png";

pub const MENU_FONT_WHITE_PATH: &str = "textures/ui/menu_font_white.png";
pub const MENU_FONT_GRAY_PATH: &str = "textures/ui/menu_font_gray.png";
pub const MENU_FONT_YELLOW_PATH: &str = "textures/ui/menu_font_yellow.png";

const EPISODE_THUMBS_ATLAS_PATH: &str = "textures/ui/episode_thumbs_atlas.png";

const EP_THUMB_W: f32 = 48.0;
const EP_THUMB_H: f32 = 24.0;

const EP_TITLE_TOP: f32 = 10.0;
const EP_LIST_TOP: f32 = 32.0;
const EP_ROW_H: f32 = 26.0;

const EP_THUMB_X: f32 = 24.0;
const EP_TEXT_X: f32 = 88.0;

const EP_HILITE_X: f32 = 76.0;
const EP_HILITE_W: f32 = 220.0;
const EP_HILITE_H: f32 = 20.0;

const BASE_HUD_H: f32 = 44.0;
const PSYCHED_DURATION_SECS: f32 = 1.2;
const PSYCHED_SPR_W: f32 = 220.0;
const PSYCHED_SPR_H: f32 = 40.0;

const PSYCHED_TEAL: Color = Color::srgb(0.00, 0.55, 0.55);
const PSYCHED_RED: Color = Color::srgb(0.80, 0.00, 0.00);

const BASE_W: f32 = 320.0;
const BASE_H: f32 = 200.0;

const MENU_CURSOR_TOP: f32 = 94.0;
const MENU_ITEM_H: f32 = 20.0;
const MENU_ACTIONS: [MenuAction; 2] = [MenuAction::NewGame, MenuAction::Quit];

const MENU_FONT_CELL_W: f32 = 18.0;
const MENU_FONT_CELL_H: f32 = 19.0;
const MENU_FONT_DRAW_SCALE: f32 = 0.5;

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

    menu_font_white: Handle<Image>,
    menu_font_gray: Handle<Image>,
    menu_font_yellow: Handle<Image>,
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
struct EpisodeTextVariant {
    selected: bool,
}

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
    selection: usize,
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

fn menu_font_cell_rect(row: u32, col: u32) -> Rect {
    let x0 = col as f32 * MENU_FONT_CELL_W;
    let y0 = row as f32 * MENU_FONT_CELL_H;
    Rect::from_corners(
        Vec2::new(x0, y0),
        Vec2::new(x0 + MENU_FONT_CELL_W, y0 + MENU_FONT_CELL_H),
    )
}

fn menu_font_glyph_rc(ch: char) -> Option<(u32, u32)> {
    match ch {
        ' ' => None,

        // --- Row 0: small font symbols/digits/uppercase (NON-ASCII layout) ---

        // !"#$%&'()*+/   (NO comma, dash, dot)
        '!' => Some((0, 0)),
        '"' => Some((0, 1)),
        '#' => Some((0, 2)),
        '$' => Some((0, 3)),
        '%' => Some((0, 4)),
        '&' => Some((0, 5)),
        '\'' => Some((0, 6)),
        '(' => Some((0, 7)),
        ')' => Some((0, 8)),
        '*' => Some((0, 9)),
        '+' => Some((0, 10)),
        '/' => Some((0, 11)),

        // 0..9 at columns 12..21
        '0'..='9' => Some((0, 12 + (ch as u32 - '0' as u32))),

        // < = > @ at columns 22..25
        '<' => Some((0, 22)),
        '=' => Some((0, 23)),
        '>' => Some((0, 24)),
        '@' => Some((0, 25)),

        // Uppercase letters in this row START at col 26,
        // but the font sheet is missing N and Q in the small-font row.
        'A'..='M' => Some((0, 26 + (ch as u32 - 'A' as u32))),
        'O'..='P' => Some((0, 39 + (ch as u32 - 'O' as u32))),
        'R'..='Z' => Some((0, 41 + (ch as u32 - 'R' as u32))),

        // If you want *something* for N/Q in small font:
        // use lowercase glyphs as fallback (looks better than garbage).
        'N' => Some((1, 5 + ('n' as u32 - 'a' as u32))),
        'Q' => Some((1, 5 + ('q' as u32 - 'a' as u32))),

        // --- Row 1: [ \ ] ^ _ a..z { | } ~ (NO backtick) ---

        '[' => Some((1, 0)),
        '\\' => Some((1, 1)),
        ']' => Some((1, 2)),
        '^' => Some((1, 3)),
        '_' => Some((1, 4)),

        // lowercase starts at col 5 (NOT 6)
        'a'..='z' => Some((1, 5 + (ch as u32 - 'a' as u32))),

        '{' => Some((1, 31)),
        '|' => Some((1, 32)),
        '}' => Some((1, 33)),
        '~' => Some((1, 34)),

        _ => None,
    }
}

fn spawn_menu_bitmap_text(
    commands: &mut Commands,
    parent: Entity,
    font_img: Handle<Image>,
    left: f32,
    top: f32,
    ui_scale: f32,
    text: &str,
    visibility: Visibility,
) -> Entity {
    let draw_w = (MENU_FONT_CELL_W * MENU_FONT_DRAW_SCALE * ui_scale).round().max(1.0);
    let draw_h = (MENU_FONT_CELL_H * MENU_FONT_DRAW_SCALE * ui_scale).round().max(1.0);
    let line_h = (draw_h + (1.0 * ui_scale).round()).max(1.0);

    let mut max_cols: i32 = 0;
    let mut cur_cols: i32 = 0;
    let mut lines: i32 = 1;

    for ch in text.chars() {
        if ch == '\n' {
            max_cols = max_cols.max(cur_cols);
            cur_cols = 0;
            lines += 1;
            continue;
        }
        cur_cols += 1;
    }
    max_cols = max_cols.max(cur_cols);

    let total_w = (max_cols as f32 * draw_w).max(1.0);
    let total_h = (lines as f32 * line_h).max(1.0);

    let run = commands
        .spawn((
            visibility,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left.round()),
                top: Val::Px(top.round()),
                width: Val::Px(total_w.round()),
                height: Val::Px(total_h.round()),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ChildOf(parent),
        ))
        .id();

    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;

    for ch in text.chars() {
        if ch == '\n' {
            x = 0.0;
            y += line_h;
            continue;
        }

        if let Some((r, c)) = menu_font_glyph_rc(ch) {
            let rect = menu_font_cell_rect(r, c);

            let mut img = ImageNode::new(font_img.clone());
            img.rect = Some(rect);

            commands.spawn((
                img,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x.round()),
                    top: Val::Px(y.round()),
                    width: Val::Px(draw_w),
                    height: Val::Px(draw_h),
                    ..default()
                },
                ChildOf(run),
            ));
        }

        x += draw_w;
    }

    run
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
    let root = commands
        .spawn((
            SplashUi,
            ZIndex(1000),
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

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_yellow.clone(),
        (44.0 * scale).round(),
        (EP_TITLE_TOP * scale).round(),
        scale,
        "Which episode to play?",
        Visibility::Visible,
    );

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

        let text_left = (EP_TEXT_X * scale).round();
        let text_top = (row_top - (2.0 * scale)).round();
        let is_selected = idx == selection;

        let gray_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_gray.clone(),
            text_left,
            text_top,
            scale,
            EP_TEXT[idx],
            if is_selected { Visibility::Hidden } else { Visibility::Visible },
        );
        commands
            .entity(gray_run)
            .insert((EpisodeItem { idx }, EpisodeTextVariant { selected: false }));

        let white_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_white.clone(),
            text_left,
            text_top,
            scale,
            EP_TEXT[idx],
            if is_selected { Visibility::Visible } else { Visibility::Hidden },
        );
        commands
            .entity(white_run)
            .insert((EpisodeItem { idx }, EpisodeTextVariant { selected: true }));
    }

    let hint = asset_server.load(MENU_HINT_PATH);
    let ui_scale = (w / BASE_W).round().max(1.0);

    let hint_native_w = 103.0;
    let hint_native_h = 12.0;
    let hint_bottom_pad = 6.0;

    let hint_w = (hint_native_w * ui_scale).round();
    let hint_h = (hint_native_h * ui_scale).round();
    let hint_x = ((BASE_W - hint_native_w) * 0.5 * ui_scale).round();
    let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();

    commands.spawn((
        ImageNode::new(hint),
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

fn spawn_menu_hint(
    commands: &mut Commands,
    asset_server: &AssetServer,
    w: f32,
    h: f32,
    imgs: &SplashImages,
) {
    let banner = asset_server.load(MENU_BANNER_PATH);
    let hint = asset_server.load(MENU_HINT_PATH);
    let cursor_light = asset_server.load(MENU_CURSOR_LIGHT_PATH);
    let cursor_dark = asset_server.load(MENU_CURSOR_DARK_PATH);

    let scale = (w / BASE_W).round().max(1.0);

    let banner_w = 156.0 * scale;
    let banner_h = 52.0 * scale;
    let banner_x = ((BASE_W - 156.0) * 0.5 * scale).round();
    let banner_y = (6.0 * scale).round();

    let hint_native_w = 103.0;
    let hint_native_h = 12.0;
    let hint_bottom_pad = 6.0;

    let hint_w = (hint_native_w * scale).round();
    let hint_h = (hint_native_h * scale).round();
    let hint_x = ((BASE_W - hint_native_w) * 0.5 * scale).round();
    let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * scale).round();

    let x_text = (150.0 * scale).round();
    let y_new_game = (92.0 * scale).round();
    let y_quit = (112.0 * scale).round();

    let cursor_w = 19.0 * scale;
    let cursor_h = 10.0 * scale;
    let cursor_x = (128.0 * scale).round();
    let cursor_y = (94.0 * scale).round();

    let root = commands
        .spawn((
            SplashUi,
            MenuHint,
            ZIndex(1001),
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
            BackgroundColor(Color::NONE),
            ChildOf(root),
        ))
        .id();

    commands.spawn((
        ImageNode::new(banner),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(banner_x),
            top: Val::Px(banner_y),
            width: Val::Px(banner_w),
            height: Val::Px(banner_h),
            ..default()
        },
        ChildOf(canvas),
    ));

    commands.spawn((
        ImageNode::new(hint),
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

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_white.clone(),
        x_text,
        y_new_game,
        scale,
        "NEW GAME",
        Visibility::Visible,
    );

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_white.clone(),
        x_text,
        y_quit,
        scale,
        "QUIT",
        Visibility::Visible,
    );

    commands.spawn((
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
        ChildOf(canvas),
    ));
    commands.spawn((
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
        ChildOf(canvas),
    ));
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

            let Some(imgs) = imgs.as_ref() else {
                return;
            };

            if q.q_splash_roots.iter().next().is_none() {
                spawn_splash_ui(&mut commands, imgs.splash0.clone(), w, h);
            }

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

            let Some(imgs) = imgs.as_ref() else {
                return;
            };

            if q.q_splash_roots.iter().next().is_none() {
                spawn_splash_ui(&mut commands, imgs.splash1.clone(), w, h);
            }

            if any_key {
                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }
                spawn_splash_ui(&mut commands, imgs.menu.clone(), w, h);
                spawn_menu_hint(&mut commands, &asset_server, w, h, imgs);
                menu.reset();
                *step = SplashStep::Menu;
            }
        }

        SplashStep::Menu => {
            lock.0 = true;
            music_mode.0 = MusicModeKind::Menu;

            let blink_on = (time.elapsed_secs() / 0.2).floor() as i32 % 2 == 0;
            let top = ((MENU_CURSOR_TOP + menu.selection as f32 * MENU_ITEM_H) * scale).round();

            for mut node in q.q_node.iter_mut() {
                node.top = Val::Px(top);
            }

            for mut v in q.q_cursor_light.iter_mut() {
                *v = if blink_on {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
            for mut v in q.q_cursor_dark.iter_mut() {
                *v = if blink_on {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }

            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                if menu.selection > 0 {
                    menu.selection -= 1;
                } else {
                    menu.selection = MENU_ACTIONS.len() - 1;
                }
                sfx.write(PlaySfx {
                    kind: SfxKind::MenuMove,
                    pos: Vec3::ZERO,
                });
            }

            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                menu.selection = (menu.selection + 1) % MENU_ACTIONS.len();
                sfx.write(PlaySfx {
                    kind: SfxKind::MenuMove,
                    pos: Vec3::ZERO,
                });
            }

            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
                sfx.write(PlaySfx {
                    kind: SfxKind::MenuSelect,
                    pos: Vec3::ZERO,
                });

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
                sfx.write(PlaySfx {
                    kind: SfxKind::MenuBack,
                    pos: Vec3::ZERO,
                });

                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }

                if let Some(imgs) = imgs.as_ref() {
                    spawn_splash_ui(&mut commands, imgs.menu.clone(), w, h);
                    spawn_menu_hint(&mut commands, &asset_server, w, h, imgs);
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
                sfx.write(PlaySfx {
                    kind: SfxKind::MenuMove,
                    pos: Vec3::ZERO,
                });
            }

            // NOTE: this is q_episode_items (not q_episode_text_runs).
            for (item, variant, mut vis) in q.q_episode_items.iter_mut() {
                let want_selected = item.idx == episode.selection;
                *vis = if variant.selected == want_selected {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }

            if let Some(mut node) = q.q_episode_hilite.iter_mut().next() {
                let hilite_top = (EP_LIST_TOP + episode.selection as f32 * EP_ROW_H + 2.0) * scale;
                node.top = Val::Px(hilite_top.round());
            }

            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
                if episode.selection == 0 {
                    sfx.write(PlaySfx {
                        kind: SfxKind::MenuSelect,
                        pos: Vec3::ZERO,
                    });

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

                    lock.0 = false;
                    music_mode.0 = MusicModeKind::Gameplay;

                    *step = SplashStep::Done;
                } else {
                    sfx.write(PlaySfx {
                        kind: SfxKind::NoWay,
                        pos: Vec3::ZERO,
                    });
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

    let menu_font_white = asset_server.load(MENU_FONT_WHITE_PATH);
    let menu_font_gray = asset_server.load(MENU_FONT_GRAY_PATH);
    let menu_font_yellow = asset_server.load(MENU_FONT_YELLOW_PATH);

    commands.insert_resource(SplashImages {
        splash0,
        splash1,
        menu,
        episode_thumbs_atlas,
        menu_font_white,
        menu_font_gray,
        menu_font_yellow,
    });
}

fn spawn_get_psyched_ui(commands: &mut Commands, asset_server: &AssetServer, win_w: f32, win_h: f32) {
    const HUD_W: f32 = 320.0;

    let hud_scale = (win_w / HUD_W).floor().max(1.0);
    let hud_h = (BASE_HUD_H * hud_scale).round();
    let view_h = (win_h - hud_h).max(0.0);

    let mut scale = hud_scale.max(1.0);
    let mut spr_w = (PSYCHED_SPR_W * scale).round();
    let mut spr_h = (PSYCHED_SPR_H * scale).round();
    if spr_w > win_w {
        scale = (win_w / PSYCHED_SPR_W).max(1.0);
        spr_w = (PSYCHED_SPR_W * scale).round();
        spr_h = (PSYCHED_SPR_H * scale).round();
    }

    let banner = asset_server.load(GET_PSYCHED_PATH);

    let left = ((win_w - spr_w) * 0.5).round().max(0.0);
    let top = ((view_h - spr_h) * 0.5).round().max(0.0);

    let bar_h = (1.0 * scale).max(1.0).round();
    let bar_top = (top + spr_h - bar_h).max(0.0);

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
    lock.0 = true;
    music_mode.0 = MusicModeKind::Gameplay;

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
