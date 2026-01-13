/*
Davenstein - by David Petnick
*/
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use davelib::audio::{MusicMode, MusicModeKind, PlaySfx, SfxKind};
use davelib::player::PlayerControlLock;

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
const MENU_FONT_HEIGHT: f32 = 20.0;
const MENU_FONT_MAP_PATH: &str = "textures/ui/menu_font_packed_map.json";
const MENU_FONT_SPACE_W: f32 = 8.0;

// Adjust these if you want tighter/looser spacing
const MENU_FONT_TRACKING_PX: f32 = 1.0;
const MENU_FONT_SPACE_ADV_PX: f32 = 8.0;

// Optional knob if you want the font smaller without touching UI scaling
const MENU_FONT_DRAW_SCALE: f32 = 0.5;

#[derive(Deserialize)]
struct PackedFontMap {
    chars: HashMap<String, PackedGlyph>,
    // (other fields exist in the file, but we only need these)
}

#[derive(Deserialize)]
struct PackedGlyph {
    #[allow(dead_code)]
    rect: [u32; 4],
    glyph_bbox_in_atlas: [u32; 4],
    baseline_pos_in_row: u32,
    baseline_in_glyph: u32,
}

static MENU_FONT_MAP: OnceLock<PackedFontMap> = OnceLock::new();

fn menu_font_map() -> &'static PackedFontMap {
    MENU_FONT_MAP.get_or_init(|| {
        let fs_path = std::path::Path::new("assets").join(MENU_FONT_MAP_PATH);
        let txt = std::fs::read_to_string(&fs_path).unwrap_or_else(|e| {
            eprintln!("[menu_font] failed to read {}: {}", fs_path.display(), e);
            String::from(r#"{"chars":{}}"#)
        });

        serde_json::from_str::<PackedFontMap>(&txt).unwrap_or_else(|e| {
            eprintln!("[menu_font] failed to parse {}: {}", fs_path.display(), e);
            PackedFontMap { chars: HashMap::new() }
        })
    })
}

struct MenuGlyph {
    rect: Rect,      // pixel rect in atlas (we use bbox)
    w: f32,
    h: f32,
    advance: f32,
    top_from_line_top: f32, // baseline alignment
}

fn menu_glyph(ch: char) -> Option<MenuGlyph> {
    // Space: advance only
    if ch == ' ' {
        return Some(MenuGlyph {
            rect: Rect::from_corners(Vec2::ZERO, Vec2::ZERO),
            w: 0.0,
            h: 0.0,
            advance: MENU_FONT_SPACE_ADV_PX,
            top_from_line_top: 0.0,
        });
    }

    let map = menu_font_map();
    let key = ch.to_string();

    // Fallback to '?' if unknown
    let g = map
        .chars
        .get(&key)
        .or_else(|| if ch != '?' { map.chars.get("?") } else { None })?;

    let [bx, by, bw, bh] = g.glyph_bbox_in_atlas;
    let bwf = bw as f32;
    let bhf = bh as f32;

    // Half-texel inset to avoid sampling borders.
    let x0 = bx as f32 + 0.5;
    let y0 = by as f32 + 0.5;
    let x1 = (bx as f32 + bwf - 0.5).max(x0 + 0.01);
    let y1 = (by as f32 + bhf - 0.5).max(y0 + 0.01);

    let top_from_line_top = (g.baseline_pos_in_row as f32) - (g.baseline_in_glyph as f32);

    Some(MenuGlyph {
        rect: Rect::from_corners(Vec2::new(x0, y0), Vec2::new(x1, y1)),
        w: bwf,
        h: bhf,
        advance: bwf + MENU_FONT_TRACKING_PX,
        top_from_line_top,
    })
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
    let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);

    // Keep line step based on the row height (not bbox), so multi-line stays stable.
    let line_h = ((MENU_FONT_HEIGHT * s) + s).round().max(1.0);

    // Measure: compute total width/height using glyph advances.
    let mut max_line_w = 0.0f32;
    let mut cur_line_w = 0.0f32;
    let mut line_count = 1;

    for ch in text.chars() {
        if ch == '\n' {
            max_line_w = max_line_w.max(cur_line_w);
            cur_line_w = 0.0;
            line_count += 1;
            continue;
        }

        if ch == ' ' {
            cur_line_w += (MENU_FONT_SPACE_W * s).round();
            continue;
        }

        if let Some(g) = menu_glyph(ch) {
            cur_line_w += (g.advance * s).round();
        }
    }

    max_line_w = max_line_w.max(cur_line_w);

    let total_w = max_line_w.max(1.0);
    let total_h = ((line_count as f32) * line_h).max(1.0);

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

    // Draw pass.
    let mut pen_x: f32 = 0.0;
    let mut pen_y: f32 = 0.0;

    for ch in text.chars() {
        if ch == '\n' {
            pen_x = 0.0;
            pen_y += line_h;
            continue;
        }

        if ch == ' ' {
            pen_x += (MENU_FONT_SPACE_W * s).round();
            continue;
        }

        let Some(g) = menu_glyph(ch) else {
            continue;
        };

        let draw_w = (g.w * s).round().max(1.0);
        let draw_h = (g.h * s).round().max(1.0);

        let mut img = ImageNode::new(font_img.clone());
        img.rect = Some(g.rect);

        commands.spawn((
            img,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(pen_x.round()),
                top: Val::Px((pen_y + g.top_from_line_top * s).round()),
                width: Val::Px(draw_w),
                height: Val::Px(draw_h),
                ..default()
            },
            ChildOf(run),
        ));

        pen_x += (g.advance * s).round();
    }

    run
}

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
