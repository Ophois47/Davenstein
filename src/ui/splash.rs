/*
Davenstein - by David Petnick
*/
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::{
    PrimaryWindow,
    WindowResized,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::ui::{
    DeathOverlay,
    GameOver,
    level_end_font::LevelEndBitmapText,
};
use davelib::audio::{
    MusicMode,
    MusicModeKind,
    PlaySfx,
    SfxKind,
};
use davelib::player::PlayerControlLock;
use crate::options::{DisplayMode, ResolutionList, VideoSettings};

pub const SPLASH_0_PATH: &str = "textures/ui/splash0.png";
pub const SPLASH_1_PATH: &str = "textures/ui/splash1.png";
pub const GET_PSYCHED_PATH: &str = "textures/ui/get_psyched.png";
pub const MENU_BANNER_PATH: &str = "textures/ui/menu_banner.png";
pub const SCORE_BANNER_PATH: &str = "textures/ui/score_banner.png";
pub const MENU_HINT_PATH: &str = "textures/ui/menu_hint.png";
pub const MENU_CURSOR_LIGHT_PATH: &str = "textures/ui/menu_cursor_light.png";
pub const MENU_CURSOR_DARK_PATH: &str = "textures/ui/menu_cursor_dark.png";
pub const SKILL_FACE_0_PATH: &str = "textures/ui/skill_faces/skill_face_0.png";
pub const SKILL_FACE_1_PATH: &str = "textures/ui/skill_faces/skill_face_1.png";
pub const SKILL_FACE_2_PATH: &str = "textures/ui/skill_faces/skill_face_2.png";
pub const SKILL_FACE_3_PATH: &str = "textures/ui/skill_faces/skill_face_3.png";
pub const MENU_FONT_WHITE_PATH: &str = "textures/ui/menu_font_white.png";
pub const MENU_FONT_GRAY_PATH: &str = "textures/ui/menu_font_gray.png";
pub const MENU_FONT_YELLOW_PATH: &str = "textures/ui/menu_font_yellow.png";
const MENU_FONT_MAP_PATH: &str = "textures/ui/menu_font_packed_map.json";
const EPISODE_THUMBS_ATLAS_PATH: &str = "textures/ui/episode_thumbs_atlas.png";
pub const MENU_FONT_BLACK_PATH: &str = "textures/ui/episode_end/menu_font_black.png";

const EP_THUMB_W: f32 = 48.0;
const EP_THUMB_H: f32 = 24.0;

const EP_TITLE_TOP: f32 = 10.0;
const EP_LIST_TOP: f32 = 32.0;
const EP_ROW_H: f32 = 24.0;

const BASE_HUD_H: f32 = 44.0;
const PSYCHED_DURATION_SECS: f32 = 2.5;
const PSYCHED_SPR_W: f32 = 220.0;
const PSYCHED_SPR_H: f32 = 40.0;

const PSYCHED_TEAL: Color = Color::srgb(0.00, 0.55, 0.55);
const PSYCHED_RED: Color = Color::srgb(0.80, 0.00, 0.00);

const BASE_W: f32 = 320.0;
const BASE_H: f32 = 200.0;

const MENU_CURSOR_TOP: f32 = 64.0;
const MENU_ITEM_H: f32 = 13.0;
const MENU_FONT_HEIGHT: f32 = 20.0;
const MENU_FONT_SPACE_W: f32 = 8.0;

// Adjust these if you want tighter/looser spacing
const MENU_FONT_TRACKING_PX: f32 = 1.0;
const MENU_FONT_SPACE_ADV_PX: f32 = 8.0;

// Optional knob if you want the font smaller without touching UI scaling
const MENU_FONT_DRAW_SCALE: f32 = 0.5;

// Episode menu layout
const EP_THUMB_X: f32 = 24.0; // left edge of the thumbnail column (in 320x200 space)
const EP_TEXT_X: f32 = 88.0;  // left edge of the episode text block (in 320x200 space)

#[derive(Resource)]
pub(crate) struct EpisodeEndImages {
    pub bj_victory_walk: [Handle<Image>; 4],
    pub bj_victory_jump: [Handle<Image>; 4],
    pub you_win: Handle<Image>,
    pub chaingun_belt: Handle<Image>,
    pub episode_page1_pic: Handle<Image>,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SplashUpdateSet {
    AdvanceInput,
    PsychedLoading,
}

#[derive(SystemParam)]
struct SplashResources<'w> {
    step: ResMut<'w, SplashStep>,
    imgs: Option<Res<'w, SplashImages>>,
    episode_end: Option<Res<'w, EpisodeEndImages>>,
    episode_stats: Res<'w, davelib::level_score::EpisodeStats>,
    hud: Res<'w, crate::ui::HudState>,
    lock: ResMut<'w, PlayerControlLock>,
    music_mode: ResMut<'w, MusicMode>,
    psyched: ResMut<'w, PsychedLoad>,
    name_entry: ResMut<'w, davelib::high_score::NameEntryState>,
    high_scores: ResMut<'w, davelib::high_score::HighScores>,
    death_overlay: Res<'w, DeathOverlay>,
    game_over: Res<'w, GameOver>,
    video_settings: ResMut<'w, VideoSettings>,
    res_list: Res<'w, ResolutionList>,
}

#[derive(SystemParam)]
pub struct SplashAdvanceInput<'w> {
	pub keyboard: Res<'w, ButtonInput<KeyCode>>,
	pub mouse: Res<'w, ButtonInput<MouseButton>>,
}

#[derive(Deserialize)]
struct PackedFontMap {
    chars: HashMap<String, PackedGlyph>,
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

const EPISODE_INFO_TITLES: [[&str; 2]; 6] = [
	["CONGRATULATIONS!", "MORE WOLFENSTEIN"],
	["CONGRATULATIONS!", "MISSION: TERMINATE HITLER"],
	["CONGRATULATIONS!", "BUT THE ADVENTURE IS JUST BEGINNING!"],
	["CONGRATULATIONS!", "THE NEXT ADVENTURE AWAITS!"],
	["CONGRATULATIONS!", "THE END IS NEAR!"],
	["CONGRATULATIONS!", "YOU DID IT!"],
];

fn episode_info_title(episode: u8, page: usize) -> &'static str {
	let epi = (episode as usize).saturating_sub(1).min(EPISODE_INFO_TITLES.len() - 1);
	let idx = page.min(1);
	EPISODE_INFO_TITLES[epi][idx]
}

const EPISODE_INFO_PAGES: [[&str; 2]; 6] = [
    // Episode 1
	[
		concat!(
			"You run out of the\n",
			"castle and hook up with\n",
			"the Underground. They\n",
			"inform you that the\n",
			"rumors were true:\n",
			"some hideous human\n",
			"experiments were seen\n",
			"around Castle Hollehammer. So Operation\n",
			"Eisenfaust is real!\n",
			"\n",
			"You must journey there and terminate the\n",
			"maniacal Dr. Schabbs before his undead\n",
			"army marches against humanity!"
		),
		concat!(
			"And in episode three, Hitler hides in his\n",
			"titanic bunker as the Third Reich crumbles\n",
			"about him. It is your job to assassinate\n",
			"him, ending his mad reign.\n",
			"\n",
			"And if you like Wolfenstein, you'll love the\n",
			"prequel trilogy of Nocturnal Missions!\n",
			"Thirty more action-packed, super-\n",
			"challenging levels!"
		),
	],
    // Episode 2
	[
		concat!(
			"You stand over Schabbs'\n",
			"fat, evil, swollen putrid\n",
			"body, glad your mission\n",
			"is finally over.  All his\n",
			"journals and equipment\n",
			"will be destroyed.\n",
			"Humanity is safe from\n",
			"his hordes of hideous mutants.\n",
			"\n",
			"Yet the Nazi atrocities continue: thousands\n",
			"march into death camps even as the Nazi\n",
			"war machine falls to its knees.  There is\n",
			"only one way to stop the madness. . . ."
		),
		concat!(
			"In episode three, Hitler hides in his titanic\n",
			"bunker as the Third Reich crumbles about\n",
			"him.  It is your job to assassinate him,\n",
			"ending his mad reign.  You find he has\n",
			"escaped to the Reichstag, and there you\n",
			"must confront him.\n",
			"\n",
			"And if you like Wolfenstein, you'll love the\n",
			"prequel trilogy of \"Nocturnal Missions!\"\n",
			"Thirty more action-packed, super-\n",
			"challenging levels!"
		),
	],
    // Episode 3
	[
        concat!(
            "The absolute incarnation\n",
            "of evil, Adolf Hitler, lies\n",
            "at your feet in a pool\n",
            "of his own blood.  His\n",
            "wrinkled, crimson-\n",
            "splattered visage still\n",
            "strains, a jagged-\n",
            "toothed rictus trying to cry out.  Insane\n",
            "even in death.  Your lips pinched in bitter\n",
            "victory, you kick his head off his remains\n",
            "and spit on his corpse.\n",
            "\n",
            "Sieg heil . . . huh.  Sieg hell."
        ),
        concat!(
            "And if you like Wolfenstein, you'll love the\n",
            "prequel trilogy of \"Nocturnal Missions!\"\n",
            "Thirty more action-packed, super-\n",
            "challenging levels!  B.J. battles the Nazis as\n",
            "they plan large-scale chemical warfare.\n",
            "Fight Otto Giftmacher, Gretel Grosse, and\n",
            "General Fettgesicht!"
        ),
    ],
    // Episode 4
	[
        concat!(
            "MAD OTTO GIFTMACHER IS\n",
            "DEAD!\n",
            "\n",
            "The twisted scientist\n",
            "behind the chemical war\n",
            "lies at your feet, but\n",
            "the fruits of his labor\n",
            "grow elsewhere!  The\n",
            "first wave of chemical\n",
            "war is already underway.  In the heavily\n",
            "guarded fortress of Erlangen are the plans\n",
            "for the upcoming Giftkrieg (or Poison War).\n",
            "Find them and you'll know where to find\n",
            "General Fettgesicht, leader of the deadly\n",
            "assault."
        ),
        concat!(
            "So don't wait . . . start the next\n",
            "adventure and find those plans!"
        ),
    ],
    // Episode 5
    [
        concat!(
            "Gretel Grosse the\n",
            "giantess guard has\n",
            "fallen.  Hope her\n",
            "brother, Hans, doesn't\n",
            "get mad about this....\n",
            "\n",
            "Now rush to the military installation at\n",
            "Offenbach and stop the horrible attack\n",
            "before thousands die under the deadly,\n",
            "burning clouds of chemical war.  Only you\n",
            "can do it, B.J.\n",
        ),
        concat!(
            "Go get General Fettgeischt before he\n",
            "begins the mad plans of pain and\n",
            "destruction!\n",
        )
    ],
    // Episode 6
	[
        concat!(
            "The General gasps his\n",
            "last breath, and the\n",
            "free world is safe\n",
            "from the terrifying\n",
            "Nazi chemical war. You\n",
            "return to Allied\n",
            "Headquarters, a Medal\n",
            "of Honor waiting for you.\n",
            "\n",
            "Allied Command informs you of some\n",
            "nefarious activities around Castle\n",
            "Hollehammer. Something about some\n",
            "grey-skinned berserk soldiers . . . .\n",
        ),
        concat!(
            "You have finished the sixth Wolfenstein\n",
            "episode!\n",
            "\n",
            "You are truly one of the great heroes!\n",
            "The world cheers your name! You get your\n",
            "picture taken with the president! People\n",
            "name their babies after you! You marry a\n",
            "movie star! Yes! You are so cool!\n",
            "\n",
            "However, In the last trilogy, B.J. must\n",
            "stop the nazis trying to fulfill Hitler's\n",
            "legacy. Fight Joseph Schultz, Eugene Grosse,\n",
            "and Heinrich!\n",
        )
    ],
];

fn episode_info_page(episode: u8, page: usize) -> &'static str {
	let epi = (episode as usize).saturating_sub(1).min(EPISODE_INFO_PAGES.len() - 1);
	let idx = page.min(1);
	EPISODE_INFO_PAGES[epi][idx]
}

struct MenuGlyph {
    rect: Rect, // Pixel Rect in Atlas (bbox)
    w: f32,
    h: f32,
    advance: f32,
    top_from_line_top: f32, // Baseline Alignment
}

fn menu_glyph(ch: char) -> Option<MenuGlyph> {
    // Space: Advance Only
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

    // Fallback to '?' if Unknown
    let g = map
        .chars
        .get(&key)
        .or_else(|| if ch != '?' { map.chars.get("?") } else { None })?;

    let [bx, by, bw, bh] = g.glyph_bbox_in_atlas;
    let bwf = bw as f32;
    let bhf = bh as f32;

    // Half Texel Inset to Avoid Sampling Borders
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

    // Keep Line Step Based on Row Height (not bbox), so Multi Line Stays Stable
    let line_h = ((MENU_FONT_HEIGHT * s) + s).round().max(1.0);

    // Measure: Compute Total Width / Height Using Glyph Advances
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

    // Draw Pass
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
    q_win: Query<'w, 's, &'static mut Window, With<PrimaryWindow>>,
    q_splash_roots: Query<'w, 's, Entity, (With<SplashUi>, Without<ChildOf>)>,
    q_node: Query<'w, 's, &'static mut Node, (With<MenuCursor>, Without<EpisodeHighlight>)>,
    q_cursor_light: Query<'w, 's, &'static mut Visibility, (With<MenuCursorLight>, Without<MenuCursorDark>)>,
    q_cursor_dark: Query<'w, 's, &'static mut Visibility, (With<MenuCursorDark>, Without<MenuCursorLight>)>,
    q_episode_items: Query<
        'w,
        's,
        (
            &'static EpisodeItem,
            &'static EpisodeTextVariant,
            &'static mut Visibility
        ),
        (
            Without<MenuCursorLight>,
            Without<MenuCursorDark>,
            Without<SkillItem>
        )
    >,
    q_skill_items: Query<
        'w,
        's,
        (
            &'static SkillItem,
            &'static SkillTextVariant,
            &'static mut Visibility
        ),
        (
            Without<MenuCursorLight>,
            Without<MenuCursorDark>,
            Without<EpisodeItem>
        )
    >,
    q_skill_face: Query<
        'w,
        's,
        &'static mut ImageNode,
        With<SkillFace>
    >,
    q_change_view_items: Query<
        'w,
        's,
        (
            &'static ChangeViewItem,
            &'static ChangeViewTextVariant,
            &'static mut Visibility
        ),
        (
            Without<MenuCursorLight>,
            Without<MenuCursorDark>,
            Without<EpisodeItem>,
            Without<SkillItem>
        ),
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
    PauseMenu,
    EpisodeSelect,
    SkillSelect,
    Scores,
    EpisodeVictory,
    EpisodeEndText0,
    EpisodeEndText1,
    NameEntry,
    ChangeView,
    Done,
}

#[derive(Default)]
struct EpisodeLocalState {
    selection: usize,
    from_pause: bool,
}

#[derive(Component, Clone, Copy)]
enum EpisodeScoreStatKind {
    Kill,
    Secret,
    Treasure,
}

#[derive(Component, Clone, Copy)]
struct EpisodeScoreStatText {
    kind: EpisodeScoreStatKind,
}

#[derive(Default)]
struct ChangeViewLocalState {
    selection: usize,
    /// When True, Resolution Sub List is Open
    res_submenu_open: bool,
    /// Currently Highlighted Index in Resolution Sub List
    res_submenu_idx: usize,
    /// Track Last Window Size to Detect When Display Mode Change
    /// Completes and UI Respawn is Needed
    needs_respawn: bool,
    /// True if Entered Change View From Pause Menu
    from_pause: bool,
    /// Hold Repeat State for Left / Right on Numeric Values (FOV, View Size)
    /// Direction: -1 = Left Held, +1 = Right Held, 0 = Not Held
    hold_dir: i8,
    /// Seconds Accumulated Since Last Repeat Tick
    hold_accum: f32,
    /// Current Repeat Interval (Starts Slow, Speeds Up)
    hold_interval: f32,
    /// How Many Ticks Have Fired in This Hold
    hold_ticks: u32,
}

/// Initial Delay Before Hold Repeat Starts (Seconds)
const HOLD_REPEAT_INITIAL: f32 = 0.35;
/// Fastest Repeat Interval (Seconds)
const HOLD_REPEAT_FAST: f32 = 0.03;
/// Interval Decreases by This Factor Each Tick
const HOLD_REPEAT_ACCEL: f32 = 0.85;

#[derive(Component)]
struct ChangeViewItem {
    idx: usize,
}

#[derive(Component, Clone, Copy)]
struct ChangeViewTextVariant {
    selected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EpisodeVictoryPhase {
    Kill,
    Secret,
    Treasure,
    Done,
}

#[derive(Resource, Debug, Clone)]
struct EpisodeVictoryTally {
    active: bool,
    phase: EpisodeVictoryPhase,

    shown_kill: i32,
    shown_secret: i32,
    shown_treasure: i32,

    target_kill: i32,
    target_secret: i32,
    target_treasure: i32,

    tick: Timer,
}

impl Default for EpisodeVictoryTally {
    fn default() -> Self {
        Self {
            active: false,
            phase: EpisodeVictoryPhase::Done,

            shown_kill: 0,
            shown_secret: 0,
            shown_treasure: 0,

            target_kill: 0,
            target_secret: 0,
            target_treasure: 0,

            tick: Timer::from_seconds(
                1.0 / 120.0,
                TimerMode::Repeating,
            ),
        }
    }
}

impl EpisodeVictoryTally {
    fn begin(&mut self, summary: davelib::level_score::EpisodeSummary) {
        self.active = true;
        self.phase = EpisodeVictoryPhase::Kill;

        self.shown_kill = 0;
        self.shown_secret = 0;
        self.shown_treasure = 0;

        self.target_kill = summary.avg_kill_pct.clamp(0, 100);
        self.target_secret = summary.avg_secret_pct.clamp(0, 100);
        self.target_treasure = summary.avg_treasure_pct.clamp(0, 100);

        self.tick.reset();
    }

    fn force_finish(&mut self) {
        self.shown_kill = self.target_kill;
        self.shown_secret = self.target_secret;
        self.shown_treasure = self.target_treasure;

        self.active = false;
        self.phase = EpisodeVictoryPhase::Done;
    }
}

#[derive(Default)]
struct SkillLocalState {
    selection: usize,
    episode_num: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    BackToGame,
    NewGame,
    Sound,
    Control,
    ChangeView,
    ViewScores,
    Quit,
}

const MENU_ACTIONS_MAIN: [MenuAction; 6] = [
    MenuAction::NewGame,
    MenuAction::Sound,
    MenuAction::Control,
    MenuAction::ChangeView,
    MenuAction::ViewScores,
    MenuAction::Quit,
];

const MENU_ACTIONS_PAUSE: [MenuAction; 7] = [
    MenuAction::NewGame,
    MenuAction::Sound,
    MenuAction::Control,
    MenuAction::ChangeView,
    MenuAction::ViewScores,
    MenuAction::BackToGame,
    MenuAction::Quit,
];

const MENU_LABELS_MAIN: [&str; 6] = [
    "New Game",
    "Sound",
    "Control",
    "Change View",
    "View Scores",
    "Quit",
];

const MENU_LABELS_PAUSE: [&str; 7] = [
    "New Game",
    "Sound",
    "Control",
    "Change View",
    "View Scores",
    "Return to Game",
    "Quit",
];

#[derive(Resource)]
struct SplashImages {
    splash0: Handle<Image>,
    splash1: Handle<Image>,
    episode_thumbs_atlas: Handle<Image>,
    menu_font_white: Handle<Image>,
    menu_font_gray: Handle<Image>,
    menu_font_yellow: Handle<Image>,
    menu_font_black: Handle<Image>,
    skill_faces: [Handle<Image>; 4],
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
struct SkillItem {
    idx: usize,
}

#[derive(Component)]
struct SkillTextVariant {
    selected: bool,
}

#[derive(Component)]
struct SkillFace;

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
            timer: Timer::from_seconds(
                PSYCHED_DURATION_SECS,
                TimerMode::Once,
            ),
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
        self.blink = Timer::from_seconds(
            0.12,
            TimerMode::Repeating,
        );
        self.blink_light = true;
    }
}

fn clear_splash_ui(
    commands: &mut Commands,
    q_splash_roots: &Query<Entity, (With<SplashUi>, Without<ChildOf>)>,
) {
    for e in q_splash_roots.iter() {
        commands.entity(e).despawn();
    }
}

/// Build Dynamic Item List for Change View Based on Current VideoSettings
/// Resolution Row Hidden When Not in Windowed Mode
/// Returns Vec<String> of Labels and Mapping of Visual Row
/// Index to Logical Item Kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeViewKind {
    Vsync,
    DisplayMode,
    Resolution,
    Fov,
    ViewSize,
    Back,
}

fn build_change_view_items(
    video: &VideoSettings,
    res_list: &ResolutionList,
) -> Vec<(ChangeViewKind, String)> {
    let mut items = Vec::new();

    // VSync
    let vsync_label = if video.vsync { "VSync: ON" } else { "VSync: OFF" };
    items.push((ChangeViewKind::Vsync, vsync_label.to_string()));

    // Display Mode
    items.push((
        ChangeViewKind::DisplayMode,
        format!("Display: {}", video.display_mode.label()),
    ));

    // Resolution (Only Shown in Windowed Mode)
    if video.display_mode == DisplayMode::Windowed {
        let res_idx = res_list.index_of(video.resolution);
        items.push((
            ChangeViewKind::Resolution,
            format!("Resolution: {}", res_list.label_at(res_idx)),
        ));
    }

    // FOV
    items.push((
        ChangeViewKind::Fov,
        format!("FOV: {}", video.fov_label()),
    ));

    // View Size
    items.push((
        ChangeViewKind::ViewSize,
        format!("View Size: {}", video.view_size_label()),
    ));

    // Back
    items.push((ChangeViewKind::Back, "Back".to_string()));

    items
}

fn spawn_change_view_ui(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    w: f32,
    h: f32,
    scale: f32,
    imgs: &SplashImages,
    selection: usize,
    video: &VideoSettings,
    res_list: &ResolutionList,
) {
    let items = build_change_view_items(video, res_list);
    let item_count = items.len();
    let selection = selection.min(item_count.saturating_sub(1));

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

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);

        let mut max_line_w = 0.0f32;
        let mut cur_line_w = 0.0f32;

        for ch in text.chars() {
            if ch == '\n' {
                max_line_w = max_line_w.max(cur_line_w);
                cur_line_w = 0.0;
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
        max_line_w.max(1.0)
    };

    let ui_scale = (w / BASE_W).round().max(1.0);

    // Title
    let title = "Change View";
    let title_w = measure_menu_text_width(scale, title);
    let title_x = ((w - title_w) * 0.5).round().max(0.0);

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_yellow.clone(),
        title_x,
        (EP_TITLE_TOP * scale).round(),
        scale,
        title,
        Visibility::Visible,
    );

    // Bottom Hint Geometry
    let hint_native_w = 103.0;
    let hint_native_h = 12.0;
    let hint_bottom_pad = 6.0;

    let hint_w = (hint_native_w * ui_scale).round();
    let hint_h = (hint_native_h * ui_scale).round();
    let hint_x = ((BASE_W - hint_native_w) * 0.5 * ui_scale).round();
    let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();

    // Panel Geometry Matches Episode Select Style
    let panel_left = (18.0 * ui_scale).round();
    let panel_top = ((EP_LIST_TOP - 4.0) * ui_scale).round();
    let panel_right = ((BASE_W - 18.0) * ui_scale).round();

    let panel_w = (panel_right - panel_left).max(1.0);
    let panel_bottom = (hint_y - (2.0 * ui_scale).round()).max(panel_top + 1.0);
    let panel_h = (panel_bottom - panel_top).max(1.0);

    let border_w = (2.0 * ui_scale).round().max(1.0);

    // Main Panel Background
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.40, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Top Shadow
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Left Shadow
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Bottom Highlight
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top + panel_h - border_w),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Right Highlight
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left + panel_w - border_w),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Option List Text Centered in Panel
    let item_labels: Vec<&str> = items.iter().map(|(_, s)| s.as_str()).collect();

    let cursor_w = (19.0 * ui_scale).round();
    let cursor_h = (10.0 * ui_scale).round();
    let row_h = (16.0 * ui_scale).round().max(1.0);

    let mut max_item_w = 0.0f32;
    for t in &item_labels {
        max_item_w = max_item_w.max(measure_menu_text_width(ui_scale, t));
    }

    let list_h = (item_count as f32 * row_h).round();
    let list_top = (panel_top + ((panel_h - list_h) * 0.5)).round();

    let text_x = (panel_left + ((panel_w - max_item_w) * 0.5)).round().max(0.0);
    let cursor_x = (text_x - cursor_w - (8.0 * ui_scale).round()).round().max(0.0);

    for idx in 0..item_count {
        let y = (list_top + idx as f32 * row_h).round();
        let is_selected = idx == selection;

        let gray_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_gray.clone(),
            text_x,
            y,
            ui_scale,
            item_labels[idx],
            if is_selected { Visibility::Hidden } else { Visibility::Visible },
        );
        commands.entity(gray_run).insert((
            ChangeViewItem { idx },
            ChangeViewTextVariant { selected: false },
        ));

        let white_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_white.clone(),
            text_x,
            y,
            ui_scale,
            item_labels[idx],
            if is_selected { Visibility::Visible } else { Visibility::Hidden },
        );
        commands.entity(white_run).insert((
            ChangeViewItem { idx },
            ChangeViewTextVariant { selected: true },
        ));
    }

    // Gun Cursor
    let cursor_light = asset_server.load(MENU_CURSOR_LIGHT_PATH);
    let cursor_dark = asset_server.load(MENU_CURSOR_DARK_PATH);

    let cursor_y = (list_top + selection as f32 * row_h + ((row_h - cursor_h) * 0.5)).round();

    commands.spawn((
        SplashUi,
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
        SplashUi,
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

    // Bottom Hint
    let hint = asset_server.load(MENU_HINT_PATH);
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

/// Spawn Resolution Sub Menu: List of All Available Resolutions
/// Using Same Panel Style as Change View, With Current Resolution Highlighted
fn spawn_resolution_submenu_ui(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    w: f32,
    h: f32,
    scale: f32,
    imgs: &SplashImages,
    selection: usize,
    res_list: &ResolutionList,
) {
    let item_count = res_list.entries.len();
    let selection = selection.min(item_count.saturating_sub(1));

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

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);
        let mut max_line_w = 0.0f32;
        let mut cur_line_w = 0.0f32;
        for ch in text.chars() {
            if ch == '\n' { 
                max_line_w = max_line_w.max(cur_line_w);
                cur_line_w = 0.0;
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
        max_line_w.max(1.0)
    };

    let ui_scale = (w / BASE_W).round().max(1.0);

    // Title
    let title = "Resolution";
    let title_w = measure_menu_text_width(scale, title);
    let title_x = ((w - title_w) * 0.5).round().max(0.0);

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_yellow.clone(),
        title_x,
        (EP_TITLE_TOP * scale).round(),
        scale,
        title,
        Visibility::Visible,
    );

    // Bottom Hint Geometry
    let hint_native_w = 103.0;
    let hint_native_h = 12.0;
    let hint_bottom_pad = 6.0;
    let hint_w = (hint_native_w * ui_scale).round();
    let hint_h = (hint_native_h * ui_scale).round();
    let hint_x = ((BASE_W - hint_native_w) * 0.5 * ui_scale).round();
    let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();

    // Panel Geometry
    let panel_left = (18.0 * ui_scale).round();
    let panel_top = ((EP_LIST_TOP - 4.0) * ui_scale).round();
    let panel_right = ((BASE_W - 18.0) * ui_scale).round();
    let panel_w = (panel_right - panel_left).max(1.0);
    let panel_bottom = (hint_y - (2.0 * ui_scale).round()).max(panel_top + 1.0);
    let panel_h = (panel_bottom - panel_top).max(1.0);
    let border_w = (2.0 * ui_scale).round().max(1.0);

    // Panel Background + Borders (Same Style as Change View)
    commands.spawn((SplashUi, Node {
        position_type: PositionType::Absolute,
        left: Val::Px(panel_left), top: Val::Px(panel_top),
        width: Val::Px(panel_w), height: Val::Px(panel_h), ..default()
    }, BackgroundColor(Color::srgb(0.40, 0.0, 0.0)), ChildOf(canvas)));

    commands.spawn((SplashUi, Node {
        position_type: PositionType::Absolute,
        left: Val::Px(panel_left), top: Val::Px(panel_top),
        width: Val::Px(panel_w), height: Val::Px(border_w), ..default()
    }, BackgroundColor(Color::srgb(0.20, 0.0, 0.0)), ChildOf(canvas)));

    commands.spawn((SplashUi, Node {
        position_type: PositionType::Absolute,
        left: Val::Px(panel_left), top: Val::Px(panel_top),
        width: Val::Px(border_w), height: Val::Px(panel_h), ..default()
    }, BackgroundColor(Color::srgb(0.20, 0.0, 0.0)), ChildOf(canvas)));

    commands.spawn((SplashUi, Node {
        position_type: PositionType::Absolute,
        left: Val::Px(panel_left), top: Val::Px(panel_top + panel_h - border_w),
        width: Val::Px(panel_w), height: Val::Px(border_w), ..default()
    }, BackgroundColor(Color::srgb(0.70, 0.0, 0.0)), ChildOf(canvas)));

    commands.spawn((SplashUi, Node {
        position_type: PositionType::Absolute,
        left: Val::Px(panel_left + panel_w - border_w), top: Val::Px(panel_top),
        width: Val::Px(border_w), height: Val::Px(panel_h), ..default()
    }, BackgroundColor(Color::srgb(0.70, 0.0, 0.0)), ChildOf(canvas)));

    // Resolution List
    let labels: Vec<String> = (0..item_count).map(|i| res_list.label_at(i)).collect();

    let cursor_w = (19.0 * ui_scale).round();
    let cursor_h = (10.0 * ui_scale).round();
    let row_h = (16.0 * ui_scale).round().max(1.0);

    let mut max_item_w = 0.0f32;
    for t in &labels {
        max_item_w = max_item_w.max(measure_menu_text_width(ui_scale, t));
    }

    let list_h = (item_count as f32 * row_h).round();
    let list_top = (panel_top + ((panel_h - list_h) * 0.5)).round();
    let text_x = (panel_left + ((panel_w - max_item_w) * 0.5)).round().max(0.0);
    let cursor_x = (text_x - cursor_w - (8.0 * ui_scale).round()).round().max(0.0);

    for idx in 0..item_count {
        let y = (list_top + idx as f32 * row_h).round();
        let is_selected = idx == selection;

        let gray_run = spawn_menu_bitmap_text(
            commands, canvas, imgs.menu_font_gray.clone(),
            text_x, y, ui_scale, &labels[idx],
            if is_selected { Visibility::Hidden } else { Visibility::Visible },
        );
        commands.entity(gray_run).insert((
            ChangeViewItem { idx },
            ChangeViewTextVariant { selected: false },
        ));

        let white_run = spawn_menu_bitmap_text(
            commands, canvas, imgs.menu_font_white.clone(),
            text_x, y, ui_scale, &labels[idx],
            if is_selected { Visibility::Visible } else { Visibility::Hidden },
        );
        commands.entity(white_run).insert((
            ChangeViewItem { idx },
            ChangeViewTextVariant { selected: true },
        ));
    }

    // Gun Cursor
    let cursor_light = asset_server.load(MENU_CURSOR_LIGHT_PATH);
    let cursor_dark = asset_server.load(MENU_CURSOR_DARK_PATH);
    let cursor_y = (list_top + selection as f32 * row_h + ((row_h - cursor_h) * 0.5)).round();

    commands.spawn((
        SplashUi, MenuCursor, MenuCursorLight, Visibility::Visible,
        ImageNode::new(cursor_light),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cursor_x), top: Val::Px(cursor_y),
            width: Val::Px(cursor_w), height: Val::Px(cursor_h), ..default()
        },
        ChildOf(canvas),
    ));

    commands.spawn((
        SplashUi, MenuCursor, MenuCursorDark, Visibility::Hidden,
        ImageNode::new(cursor_dark),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cursor_x), top: Val::Px(cursor_y),
            width: Val::Px(cursor_w), height: Val::Px(cursor_h), ..default()
        },
        ChildOf(canvas),
    ));

    // Bottom Hint
    let hint = asset_server.load(MENU_HINT_PATH);
    commands.spawn((
        ImageNode::new(hint),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(hint_x), top: Val::Px(hint_y),
            width: Val::Px(hint_w), height: Val::Px(hint_h), ..default()
        },
        ChildOf(canvas),
    ));
}

fn spawn_episode_score_ui(
    commands: &mut Commands,
    _imgs: &SplashImages,
    episode_end: &EpisodeEndImages,
    episode_stats: &davelib::level_score::EpisodeStats,
    episode_num: u8,
    w: f32,
    h: f32,
    _total_score: i32,
) {
    const BASE_VIEW_H: f32 = BASE_H - BASE_HUD_H;
    const TEXT_SCALE: f32 = 0.80;

    let hud_scale = (w / BASE_W).floor().max(1.0);
    let hud_h_px = (BASE_HUD_H * hud_scale).round();
    let view_h_px = (h - hud_h_px).max(1.0);

    let max_scale_h = (view_h_px / BASE_VIEW_H).floor().max(1.0);
    let ui_scale = hud_scale.min(max_scale_h);

    let canvas_w_px = (BASE_W * ui_scale).round().max(1.0);
    let canvas_h_px = (BASE_VIEW_H * ui_scale).round().max(1.0);

    let canvas_left_px = ((w - canvas_w_px) * 0.5).round();
    let canvas_top_px = ((view_h_px - canvas_h_px) * 0.5).round();

    let teal_bg = Color::srgb(0.0, 64.0 / 255.0, 64.0 / 255.0);

    let root = commands
        .spawn((
            SplashUi,
            ZIndex(-10),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(hud_h_px),
                ..default()
            },
            BackgroundColor(teal_bg),
        ))
        .id();

    let canvas = commands
        .spawn((
            ChildOf(root),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(canvas_left_px),
                top: Val::Px(canvas_top_px),
                width: Val::Px(canvas_w_px),
                height: Val::Px(canvas_h_px),
                ..default()
            },
        ))
        .id();

    let summary = episode_stats.summary_for_episode(episode_num);
    let total_secs = summary.total_time_secs.max(0.0).floor() as u32;
    let total_minutes = total_secs / 60;
    let total_seconds = total_secs % 60;
    let total_time_str = format!("{total_minutes}:{total_seconds:02}");

    let bt_mul = (ui_scale / hud_scale).max(0.01);
    let bt_scale = TEXT_SCALE * bt_mul;

    let spawn_bt_box =
        |commands: &mut Commands, text: &str, x: f32, y: f32, w: f32, justify: JustifyContent| -> Entity {
            commands
                .spawn((
                    ChildOf(canvas),
                    LevelEndBitmapText {
                        text: text.to_string(),
                        scale: bt_scale,
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px((x * ui_scale).round()),
                        top: Val::Px((y * ui_scale).round()),
                        width: Val::Px((w * ui_scale).round().max(1.0)),
                        flex_direction: FlexDirection::Row,
                        justify_content: justify,
                        ..default()
                    },
                ))
                .id()
        };

    let portrait_img = ImageNode::new(episode_end.chaingun_belt.clone());
    commands.spawn((
        ChildOf(canvas),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px((8.0 * ui_scale).round()),
            top: Val::Px((4.0 * ui_scale).round()),
            width: Val::Px((87.0 * ui_scale).round()),
            height: Val::Px((87.0 * ui_scale).round()),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        portrait_img,
    ));

    let _ = spawn_bt_box(commands, "YOU WIN!", 96.0, 16.0, 224.0, JustifyContent::Center);
    let _ = spawn_bt_box(commands, "TOTAL TIME", 96.0, 48.0, 192.0, JustifyContent::Center);
    let _ = spawn_bt_box(commands, &total_time_str, 114.0, 64.0, 120.0, JustifyContent::FlexStart);
    let _ = spawn_bt_box(commands, "AVERAGES", 0.0, 96.0, 320.0, JustifyContent::Center);

    let label_col_w = 173.0;
    let pct_w = 125.0;
    let pct_x = 304.0 - pct_w;

    let _ = spawn_bt_box(commands, "KILL", 0.0, 112.0, label_col_w, JustifyContent::FlexEnd);
    let e = spawn_bt_box(commands, "0%", pct_x, 112.0, pct_w, JustifyContent::FlexEnd);
    commands.entity(e).insert(EpisodeScoreStatText { kind: EpisodeScoreStatKind::Kill });

    let _ = spawn_bt_box(commands, "SECRET", 0.0, 128.0, label_col_w, JustifyContent::FlexEnd);
    let e = spawn_bt_box(commands, "0%", pct_x, 128.0, pct_w, JustifyContent::FlexEnd);
    commands.entity(e).insert(EpisodeScoreStatText { kind: EpisodeScoreStatKind::Secret });

    let _ = spawn_bt_box(commands, "TREASURE", 0.0, 144.0, label_col_w, JustifyContent::FlexEnd);
    let e = spawn_bt_box(commands, "0%", pct_x, 144.0, pct_w, JustifyContent::FlexEnd);
    commands.entity(e).insert(EpisodeScoreStatText { kind: EpisodeScoreStatKind::Treasure });
}

fn tick_episode_victory_tally(
    time: Res<Time>,
    step: Res<SplashStep>,
    mut tally: ResMut<EpisodeVictoryTally>,
    mut sfx: MessageWriter<PlaySfx>,
    mut pause_steps_local: Local<u8>,
    mut pending_stinger_local: Local<Option<SfxKind>>,
    mut pending_stinger_pause_local: Local<u8>,
) {
    if *step != SplashStep::EpisodeVictory {
        tally.active = false;
        tally.phase = EpisodeVictoryPhase::Done;
        *pause_steps_local = 0;
        *pending_stinger_local = None;
        *pending_stinger_pause_local = 0;
        return;
    }

    if !tally.active {
        return;
    }

    if !tally.tick.tick(time.delta()).just_finished() {
        return;
    }

    if *pause_steps_local > 0 {
        *pause_steps_local = pause_steps_local.saturating_sub(1);
        return;
    }

    if let Some(stinger) = pending_stinger_local.take() {
        sfx.write(PlaySfx { kind: stinger, pos: Vec3::ZERO });
        *pause_steps_local = *pending_stinger_pause_local;
        *pending_stinger_pause_local = 0;
        return;
    }

    let mut schedule_end = |ratio: i32, pause_after: u8, next_pause: u8| {
        if ratio >= 100 {
            *pending_stinger_local = Some(SfxKind::IntermissionPercent100);
            *pending_stinger_pause_local = next_pause;
        } else if ratio <= 0 {
            *pending_stinger_local = Some(SfxKind::IntermissionNoBonus);
            *pending_stinger_pause_local = next_pause;
        } else {
            *pending_stinger_local = Some(SfxKind::IntermissionConfirm);
            *pending_stinger_pause_local = next_pause;
        }

        *pause_steps_local = pause_after;
    };

    match tally.phase {
        EpisodeVictoryPhase::Kill => {
            if tally.shown_kill < tally.target_kill {
                tally.shown_kill = (tally.shown_kill + 2).min(tally.target_kill);
                if tally.shown_kill % 10 == 0 {
                    sfx.write(PlaySfx { kind: SfxKind::IntermissionTick, pos: Vec3::ZERO });
                }
            } else {
                schedule_end(tally.target_kill, 10, 30);
                tally.phase = EpisodeVictoryPhase::Secret;
            }
        }

        EpisodeVictoryPhase::Secret => {
            if tally.shown_secret < tally.target_secret {
                tally.shown_secret = (tally.shown_secret + 2).min(tally.target_secret);
                if tally.shown_secret % 10 == 0 {
                    sfx.write(PlaySfx { kind: SfxKind::IntermissionTick, pos: Vec3::ZERO });
                }
            } else {
                schedule_end(tally.target_secret, 10, 30);
                tally.phase = EpisodeVictoryPhase::Treasure;
            }
        }

        EpisodeVictoryPhase::Treasure => {
            if tally.shown_treasure < tally.target_treasure {
                tally.shown_treasure = (tally.shown_treasure + 2).min(tally.target_treasure);
                if tally.shown_treasure % 10 == 0 {
                    sfx.write(PlaySfx { kind: SfxKind::IntermissionTick, pos: Vec3::ZERO });
                }
            } else {
                schedule_end(tally.target_treasure, 10, 30);
                tally.active = false;
                tally.phase = EpisodeVictoryPhase::Done;
            }
        }

        EpisodeVictoryPhase::Done => {
            tally.active = false;
        }
    }
}

fn sync_episode_victory_score_text(
    step: Res<SplashStep>,
    tally: Res<EpisodeVictoryTally>,
    mut q_text: Query<(&EpisodeScoreStatText, &mut LevelEndBitmapText)>,
) {
    if *step != SplashStep::EpisodeVictory {
        return;
    }

    for (tag, mut bt) in q_text.iter_mut() {
        let v = match tag.kind {
            EpisodeScoreStatKind::Kill => tally.shown_kill,
            EpisodeScoreStatKind::Secret => tally.shown_secret,
            EpisodeScoreStatKind::Treasure => tally.shown_treasure,
        };

        bt.text = format!("{v}%");
    }
}

fn spawn_episode_end_text_ui(
    commands: &mut Commands,
    w: f32,
    h: f32,
    imgs: &SplashImages,
    episode_end: &EpisodeEndImages,
    episode_num: u8,
    page_idx: usize,
) -> Entity {
    let ui_scale = (w / BASE_W).round().max(1.0);

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
            BackgroundColor(Color::BLACK),
            ChildOf(root),
        ))
        .id();

    commands.spawn((
        SplashUi,
        ImageNode::new(episode_end.you_win.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        ChildOf(canvas),
    ));

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);

        let mut max_line_w = 0.0f32;
        let mut cur_line_w = 0.0f32;

        for ch in text.chars() {
            if ch == '\n' {
                max_line_w = max_line_w.max(cur_line_w);
                cur_line_w = 0.0;
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
        max_line_w.max(1.0)
    };

    fn tokenize_for_wrap(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let lines: Vec<&str> = text.split('\n').collect();

        for (li, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                out.push("\n".to_string());
            } else {
                for w in trimmed.split_whitespace() {
                    out.push(w.to_string());
                }

                if li + 1 < lines.len() {
                    out.push("\n".to_string());
                }
            }
        }

        out
    }

    fn wrap_tokens<F: Fn(&str) -> f32>(
        tokens: &[String],
        mut i: usize,
        max_w: f32,
        max_lines: Option<usize>,
        measure: &F,
    ) -> (Vec<String>, usize) {
        let mut lines: Vec<String> = Vec::new();
        let mut cur = String::new();

        let push_line = |lines: &mut Vec<String>, cur: &mut String| {
            if !cur.is_empty() {
                lines.push(std::mem::take(cur));
            } else {
                lines.push(String::new());
            }
        };

        while i < tokens.len() {
            if let Some(limit) = max_lines {
                if lines.len() >= limit {
                    break;
                }
            }

            if tokens[i] == "\n" {
                push_line(&mut lines, &mut cur);
                i += 1;
                continue;
            }

            let word = &tokens[i];

            let candidate = if cur.is_empty() {
                word.clone()
            } else {
                let mut s = String::with_capacity(cur.len() + 1 + word.len());
                s.push_str(&cur);
                s.push(' ');
                s.push_str(word);
                s
            };

            if measure(&candidate) <= max_w || cur.is_empty() {
                cur = candidate;
                i += 1;
                continue;
            }

            push_line(&mut lines, &mut cur);
        }

        if let Some(limit) = max_lines {
            if lines.len() < limit && !cur.is_empty() {
                lines.push(cur);
            }
        } else if !cur.is_empty() {
            lines.push(cur);
        }

        (lines, i)
    }

    let panel_left = (8.0 * ui_scale).round();
    let panel_top = (8.0 * ui_scale).round();
    let panel_w = (304.0 * ui_scale).round().max(1.0);
    let panel_h = (168.0 * ui_scale).round().max(1.0);

    let panel = commands
        .spawn((
            SplashUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(panel_left),
                top: Val::Px(panel_top),
                width: Val::Px(panel_w),
                height: Val::Px(panel_h),
                ..default()
            },
            BackgroundColor(Color::WHITE),
            ChildOf(canvas),
        ))
        .id();

    let title = episode_info_title(episode_num, page_idx);

    let pad_x = (10.0 * ui_scale).round();
    let pad_y = (10.0 * ui_scale).round();

    let _title_w = measure_menu_text_width(ui_scale, title);

    let title_x = if page_idx == 0 {
        (pad_x + (96.0 * ui_scale)).round()
    } else {
        pad_x
    };

    let title_tint = Color::srgb(0.00, 0.64, 0.56);

    spawn_menu_bitmap_text_tinted(
        commands,
        panel,
        imgs.menu_font_white.clone(),
        title_x,
        pad_y,
        ui_scale,
        title,
        Visibility::Visible,
        title_tint,
    );

    let body = episode_info_page(episode_num, page_idx);

    let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);
    let body_y = (pad_y
        + ((MENU_FONT_HEIGHT + 1.0) * (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01))
        + (4.0 * ui_scale))
        .round();

    let line_h = ((MENU_FONT_HEIGHT * s) + s).round().max(1.0);

    if page_idx == 0 {
        let pic_x = pad_x;
        let pic_y = pad_y;

        let pic_w = (88.0 * ui_scale).round();
        let pic_h = 64.0 * ui_scale;

        let pic_gap_x = (8.0 * ui_scale).round();

        commands.spawn((
            SplashUi,
            ImageNode::new(episode_end.episode_page1_pic.clone()),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(pic_x),
                top: Val::Px(pic_y),
                width: Val::Px(pic_w),
                height: Val::Px(pic_h),
                ..default()
            },
            ChildOf(panel),
        ));

        let pic_lines = ((pic_h / line_h).ceil() as usize).max(1);

        let narrow_x = (pic_x + pic_w + pic_gap_x).round();
        let narrow_w = (panel_w - narrow_x - pad_x).round().max(1.0);

        let full_x = pad_x;
        let full_w = (panel_w - (2.0 * pad_x)).round().max(1.0);

        let tokens = tokenize_for_wrap(body);
        let measure_line = |t: &str| -> f32 { measure_menu_text_width(ui_scale, t) };

        let (lines_a, next_i) = wrap_tokens(&tokens, 0, narrow_w, Some(pic_lines), &measure_line);
        let (lines_b, _) = wrap_tokens(&tokens, next_i, full_w, None, &measure_line);

        if !lines_a.is_empty() {
            spawn_menu_bitmap_text(
                commands,
                panel,
                imgs.menu_font_black.clone(),
                narrow_x,
                body_y,
                ui_scale,
                &lines_a.join("\n"),
                Visibility::Visible,
            );
        }

        if !lines_b.is_empty() {
            let full_y = (body_y + (pic_lines as f32 * line_h)).round();
            spawn_menu_bitmap_text(
                commands,
                panel,
                imgs.menu_font_black.clone(),
                full_x,
                full_y,
                ui_scale,
                &lines_b.join("\n"),
                Visibility::Visible,
            );
        }
    } else {
        spawn_menu_bitmap_text(
            commands,
            panel,
            imgs.menu_font_black.clone(),
            pad_x,
            body_y,
            ui_scale,
            body,
            Visibility::Visible,
        );
    }

    let page_text = format!("pg {} of 2", page_idx + 1);
    let page_w = measure_menu_text_width(ui_scale, &page_text);
    let page_h = (MENU_FONT_HEIGHT * s).round().max(1.0);

    let btn_left = (200.0 * ui_scale).round();
    let btn_top = (180.0 * ui_scale).round();
    let btn_w = (90.0 * ui_scale).round();
    let btn_h = (16.0 * ui_scale).round();

    let page_x = (btn_left + (btn_w - page_w) * 0.5).round().max(0.0);
    let page_y = (btn_top + (btn_h - page_h) * 0.5).round().max(0.0);

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_black.clone(),
        page_x,
        page_y,
        ui_scale,
        &page_text,
        Visibility::Visible,
    );

    root
}

fn spawn_menu_bitmap_text_tinted(
    commands: &mut Commands,
    parent: Entity,
    font_img: Handle<Image>,
    left: f32,
    top: f32,
    ui_scale: f32,
    text: &str,
    visibility: Visibility,
    tint: Color,
) -> Entity {
    let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);

    let line_h = ((MENU_FONT_HEIGHT * s) + s).round().max(1.0);

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
        img.color = tint;

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

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashStep>();
        app.init_resource::<PsychedLoad>();
        app.init_resource::<EpisodeVictoryTally>();
        app.configure_sets(
            Update,
            (SplashUpdateSet::AdvanceInput, SplashUpdateSet::PsychedLoading).chain_ignore_deferred(),
        );
        app.add_systems(
            Update,
            splash_advance_on_any_input,
        );
        app.add_systems(
            Update,
            tick_episode_victory_tally.after(splash_advance_on_any_input),
        );
        app.add_systems(
            Update,
            sync_episode_victory_score_text.after(tick_episode_victory_tally),
        );
        app.add_systems(
            Update,
            auto_get_psyched_on_level_start.in_set(SplashUpdateSet::PsychedLoading),
        );
        app.add_systems(
            Update,
            tick_get_psyched_loading.in_set(SplashUpdateSet::PsychedLoading),
        );
        app.add_systems(
            Update,
            splash_resize_on_window_change.in_set(SplashUpdateSet::PsychedLoading),
        );
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

    // ---- Title ----
    let title = "Which episode to play?";

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);

        let mut max_line_w = 0.0f32;
        let mut cur_line_w = 0.0f32;

        for ch in text.chars() {
            if ch == '\n' {
                max_line_w = max_line_w.max(cur_line_w);
                cur_line_w = 0.0;
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
        max_line_w.max(1.0)
    };

    let title_w = measure_menu_text_width(scale, title);
    let title_x = ((w - title_w) * 0.5).round().max(0.0);

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_yellow.clone(),
        title_x,
        (EP_TITLE_TOP * scale).round(),
        scale,
        title,
        Visibility::Visible,
    );

    // ---- Hint Placement (so panel doesn't cover it) ----
    let hint_native_w = 103.0;
    let hint_native_h = 12.0;
    let hint_bottom_pad = 6.0;

    let ui_scale = (w / BASE_W).round().max(1.0);
    let hint_w = (hint_native_w * ui_scale).round();
    let hint_h = (hint_native_h * ui_scale).round();
    let hint_x = ((BASE_W - hint_native_w) * 0.5 * ui_scale).round();
    let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();

    // ---- Cursor + Gutter Column (so gun never overlaps thumbs) ----
    let cursor_w = (19.0 * ui_scale).round();
    let cursor_h = (10.0 * ui_scale).round();

    // Space Reserved to Left of Thumbnail Column:
    // Cursor Width + Little Breathing Room
    let gutter_x = cursor_w + (10.0 * ui_scale).round();

    // Thumbnails + Text Start After Gutter
    let thumb_x = (EP_THUMB_X * ui_scale).round() + gutter_x;
    let text_x = (EP_TEXT_X * ui_scale).round() + gutter_x;

    // Cursor Sits Just Left of Thumbnail Column
    let cursor_x = (thumb_x - cursor_w - (8.0 * ui_scale).round()).max(0.0);

    // ---- Sunken Darker-Red Panel Behind Episode Thumbs + Text + Cursor ----
    let panel_left = (cursor_x - (8.0 * ui_scale).round()).max(0.0);
    let panel_top = ((EP_LIST_TOP - 4.0) * ui_scale).round();

    let panel_right = ((BASE_W - 18.0) * ui_scale).round();
    let panel_w = (panel_right - panel_left).max(1.0);

    let panel_bottom = (hint_y - (2.0 * ui_scale).round()).max(panel_top + 1.0);
    let panel_h = (panel_bottom - panel_top).max(1.0);

    let border_w = (2.0 * ui_scale).round().max(1.0);

    // Main panel background
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.40, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Top shadow (darker - makes it look recessed)
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Left shadow (darker)
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Bottom highlight (lighter - the "light source")
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top + panel_h - border_w),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Right highlight (lighter)
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left + panel_w - border_w),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // ---- Episodes ----
    const EP_TEXT: [&str; 6] = [
        "Episode 1\nEscape from Wolfenstein",
        "Episode 2\nOperation: Eisenfaust",
        "Episode 3\nDie, Fuhrer, Die!",
        "Episode 4\nA Dark Secret",
        "Episode 5\nTrail of the Madman",
        "Episode 6\nConfrontation",
    ];

    for idx in 0..6 {
        let row_top = (EP_LIST_TOP + idx as f32 * EP_ROW_H) * ui_scale;

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
                left: Val::Px(thumb_x),
                top: Val::Px(row_top.round()),
                width: Val::Px((EP_THUMB_W * ui_scale).round()),
                height: Val::Px((EP_THUMB_H * ui_scale).round()),
                ..default()
            },
            img,
            ChildOf(canvas),
        ));

        let text_top = (row_top + (1.8 * ui_scale)).round();
        let is_selected = idx == selection;

        let gray_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_gray.clone(),
            text_x,
            text_top,
            ui_scale,
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
            text_x,
            text_top,
            ui_scale,
            EP_TEXT[idx],
            if is_selected { Visibility::Visible } else { Visibility::Hidden },
        );
        commands
            .entity(white_run)
            .insert((EpisodeItem { idx }, EpisodeTextVariant { selected: true }));
    }

    // ---- Gun Cursor ----
    let cursor_light = asset_server.load(MENU_CURSOR_LIGHT_PATH);
    let cursor_dark = asset_server.load(MENU_CURSOR_DARK_PATH);

    let sel_row_top = (EP_LIST_TOP + selection as f32 * EP_ROW_H) * ui_scale;
    let cursor_y = (sel_row_top + ((EP_THUMB_H * ui_scale - cursor_h) * 0.5)).round();

    commands.spawn((
        SplashUi,
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
        SplashUi,
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

    // ---- Bottom Hint ----
    let hint = asset_server.load(MENU_HINT_PATH);
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

fn spawn_skill_select_ui(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    w: f32,
    h: f32,
    scale: f32,
    imgs: &SplashImages,
    selection: usize,
) {
    let selection = selection.min(3);

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

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);

        let mut max_line_w = 0.0f32;
        let mut cur_line_w = 0.0f32;

        for ch in text.chars() {
            if ch == '\n' {
                max_line_w = max_line_w.max(cur_line_w);
                cur_line_w = 0.0;
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
        max_line_w.max(1.0)
    };

    let ui_scale = (w / BASE_W).round().max(1.0);

    // Bottom hint geometry
    let hint_native_w = 103.0;
    let hint_native_h = 12.0;
    let hint_bottom_pad = 6.0;

    let hint_w = (hint_native_w * ui_scale).round();
    let hint_h = (hint_native_h * ui_scale).round();
    let hint_x = ((BASE_W - hint_native_w) * 0.5 * ui_scale).round();
    let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();

    // Title
    let title = "How tough are you?";
    let title_w = measure_menu_text_width(scale, title);
    let title_x = ((w - title_w) * 0.5).round().max(0.0);
    let title_top = (40.0 * ui_scale).round();

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_yellow.clone(),
        title_x,
        title_top,
        ui_scale,
        title,
        Visibility::Visible,
    );

    // Panel layout
    let desired_panel_w = (236.0 * ui_scale).round().max(1.0);
    let panel_left = ((w - desired_panel_w) * 0.5).round().max(0.0);
    let panel_top = (58.0 * ui_scale).round();

    let row_h = (MENU_ITEM_H * ui_scale).round();
    let pad_y = (12.0 * ui_scale).round();
    let desired_panel_h = (pad_y * 2.0 + row_h * 4.0).round();

    let max_panel_h = (hint_y - (2.0 * ui_scale).round() - panel_top).max(1.0);
    let panel_h = desired_panel_h.min(max_panel_h).max(1.0);
    let panel_w = desired_panel_w;

    let border_w = (2.0 * ui_scale).round().max(1.0);

    // Main panel background
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.40, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Top shadow
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Left shadow
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Bottom highlight
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top + panel_h - border_w),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Right highlight
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left + panel_w - border_w),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Cursor + text layout inside panel
    let cursor_w = (19.0 * ui_scale).round();
    let cursor_h = (10.0 * ui_scale).round();

    let cursor_x = (panel_left + (14.0 * ui_scale).round()).round();
    let cursor_y0 = (panel_top + (14.0 * ui_scale).round()).round();

    let text_x = (cursor_x + cursor_w + (6.0 * ui_scale).round()).round();
    let text_y0 = (cursor_y0 - (2.0 * ui_scale).round()).round();

    // Face portrait on the right side of the panel
    let face_w = (24.0 * ui_scale).round().max(1.0);
    let face_h = (32.0 * ui_scale).round().max(1.0);
    let face_x = (panel_left + panel_w - face_w - (12.0 * ui_scale).round()).round();
    let face_y = (panel_top + (12.0 * ui_scale).round()).round();

    commands.spawn((
        SplashUi,
        SkillFace,
        ImageNode::new(imgs.skill_faces[selection].clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(face_x),
            top: Val::Px(face_y),
            width: Val::Px(face_w),
            height: Val::Px(face_h),
            ..default()
        },
        ChildOf(canvas),
    ));

    // Options
    const SKILL_TEXT: [&str; 4] = [
        "Can I play, Daddy?",
        "Don't hurt me.",
        "Bring 'em on!",
        "I am Death incarnate!",
    ];

    for idx in 0..4 {
        let y = (text_y0 + idx as f32 * row_h).round();
        let is_selected = idx == selection;

        let gray_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_gray.clone(),
            text_x,
            y,
            ui_scale,
            SKILL_TEXT[idx],
            if is_selected { Visibility::Hidden } else { Visibility::Visible },
        );
        commands
            .entity(gray_run)
            .insert((SkillItem { idx }, SkillTextVariant { selected: false }));

        let white_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_white.clone(),
            text_x,
            y,
            ui_scale,
            SKILL_TEXT[idx],
            if is_selected { Visibility::Visible } else { Visibility::Hidden },
        );
        commands
            .entity(white_run)
            .insert((SkillItem { idx }, SkillTextVariant { selected: true }));
    }

    // Gun cursor
    let cursor_light = asset_server.load(MENU_CURSOR_LIGHT_PATH);
    let cursor_dark = asset_server.load(MENU_CURSOR_DARK_PATH);

    let cursor_y = (cursor_y0 + selection as f32 * row_h).round();

    commands.spawn((
        SplashUi,
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
        SplashUi,
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

    // Bottom hint
    let hint = asset_server.load(MENU_HINT_PATH);
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

fn spawn_splash_ui(
    commands: &mut Commands,
    image: Handle<Image>,
    w: f32,
    h: f32,
    version_font_img: Option<Handle<Image>>,
) {
    const BUILD_VERSION: &str = concat!("V", env!("CARGO_PKG_VERSION"));
    const VERSION_SCALE: f32 = 0.50;

    let ui_scale = (w / BASE_W).floor().max(1.0);

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);

        let mut max_line_w = 0.0f32;
        let mut cur_line_w = 0.0f32;

        for ch in text.chars() {
            if ch == '\n' {
                max_line_w = max_line_w.max(cur_line_w);
                cur_line_w = 0.0;
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
        max_line_w.max(1.0)
    };

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
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .id();

    let canvas = commands
        .spawn((
            SplashImage,
            Node {
                width: Val::Px(w),
                height: Val::Px(h),
                position_type: PositionType::Relative,
                ..default()
            },
            ChildOf(root),
        ))
        .id();

    commands.spawn((
        ImageNode::new(image),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ChildOf(canvas),
    ));

    let Some(font_img) = version_font_img else { return; };

    let ver_ui_scale = (ui_scale * VERSION_SCALE).max(0.01);

    let ver_w = measure_menu_text_width(ver_ui_scale, BUILD_VERSION);

    let s = (ver_ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);
    let ver_h = ((MENU_FONT_HEIGHT * s) + s).round().max(1.0);

    // Anchor Small Container to Bottom Right of Splash Canvas
    // This Avoids Any Mismatch Between Placement Math and spawn_menu_bitmap_text Scaling
    let margin = (2.0 * ui_scale).round().max(2.0);

    let ver_root = commands
        .spawn((
            Node {
                width: Val::Px(ver_w),
                height: Val::Px(ver_h),
                position_type: PositionType::Absolute,
                right: Val::Px(margin),
                bottom: Val::Px(margin),
                ..default()
            },
            ChildOf(canvas),
        ))
        .id();

    spawn_menu_bitmap_text(
        commands,
        ver_root,
        font_img,
        0.0,
        0.0,
        ver_ui_scale,
        BUILD_VERSION,
        Visibility::Visible,
    );
}

fn high_score_rank_for(high_scores: &davelib::high_score::HighScores, score: i32) -> usize {
    let score = score.max(0);

    for (i, e) in high_scores.entries.iter().enumerate() {
        if e.score < score {
            return i;
        }
    }

    high_scores.entries.len()
}

fn spawn_name_entry_ui(
    commands: &mut Commands,
    w: f32,
    h: f32,
    imgs: &SplashImages,
    rank: usize,
    current_name: &str,
) {
    let ui_scale = (w / BASE_W).round().max(1.0);

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

    // Title based on rank
    let title = match rank {
        0 => "You're the BEST player!",
        1 => "You're the 2nd best player!",
        2 => "You're the 3rd best player!",
        _ => "You got a high score!",
    };

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);
        let mut w = 0.0f32;
        for ch in text.chars() {
            if ch == ' ' {
                w += (MENU_FONT_SPACE_W * s).round();
                continue;
            }
            if let Some(g) = menu_glyph(ch) {
                w += (g.advance * s).round();
            }
        }
        w.max(1.0)
    };

    let title_w = measure_menu_text_width(ui_scale, title);
    let title_x = ((w - title_w) * 0.5).round().max(0.0);
    let title_y = (40.0 * ui_scale).round();

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_yellow.clone(),
        title_x,
        title_y,
        ui_scale,
        title,
        Visibility::Visible,
    );

    // Prompt
    let prompt = "Enter your name:";
    let prompt_w = measure_menu_text_width(ui_scale, prompt);
    let prompt_x = ((w - prompt_w) * 0.5).round().max(0.0);
    let prompt_y = (80.0 * ui_scale).round();

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_white.clone(),
        prompt_x,
        prompt_y,
        ui_scale,
        prompt,
        Visibility::Visible,
    );

    // Name display (3 slots with underscores for empty slots)
    let mut display_name = current_name.to_string();
    while display_name.len() < 3 {
        display_name.push('_');
    }

    let name_y = (110.0 * ui_scale).round();
    let name_w = measure_menu_text_width(ui_scale, &display_name);
    let name_x = ((w - name_w) * 0.5).round().max(0.0);

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_yellow.clone(),
        name_x,
        name_y,
        ui_scale,
        &display_name,
        Visibility::Visible,
    );

    // Hint at bottom
    let hint = "(Press ENTER when done)";
    let hint_w = measure_menu_text_width(ui_scale, hint);
    let hint_x = ((w - hint_w) * 0.5).round().max(0.0);
    let hint_y = (160.0 * ui_scale).round();

    spawn_menu_bitmap_text(
        commands,
        canvas,
        imgs.menu_font_gray.clone(),
        hint_x,
        hint_y,
        ui_scale,
        hint,
        Visibility::Visible,
    );
}

fn spawn_scores_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    w: f32,
    h: f32,
    imgs: &SplashImages,
    high_scores: &davelib::high_score::HighScores,
) {
    let banner = asset_server.load(SCORE_BANNER_PATH);
    let ui_scale = (w / BASE_W).round().max(1.0);

    // Match main menu banner approach EXACTLY
    let banner_native_h = 48.0;
    let top_red = (3.0 * ui_scale).round();

    let banner_x = 0.0;
    let banner_y = top_red;
    let banner_w = w;
    let banner_h = (banner_native_h * ui_scale).round();

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

    // Top red strip (matches menu exactly)
    commands.spawn((
        SplashUi,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(w),
            height: Val::Px(top_red),
            ..default()
        },
        BackgroundColor(Color::srgb(0.60, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Black banner band
    let band = commands
        .spawn((
            SplashUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(banner_x),
                top: Val::Px(banner_y),
                width: Val::Px(banner_w),
                height: Val::Px(banner_h),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            ChildOf(canvas),
        ))
        .id();

    // Centered score banner image inside the black band
    commands.spawn((
        SplashUi,
        ImageNode::new(banner),
        Node {
            width: Val::Px(banner_w),
            height: Val::Px(banner_h),
            ..default()
        },
        ChildOf(band),
    ));

    // Convert high scores to display format
    let mut rows: Vec<(String, String, String)> = Vec::new();
    for (i, entry) in high_scores.entries.iter().enumerate() {
        rows.push((
            format!("{}", i + 1),
            entry.name.clone(),
            format!("{:06}", entry.score),
        ));
    }

    // Pad to 10 rows if needed (original Wolf3D always showed 10 slots)
    while rows.len() < 10 {
        let rank = rows.len() + 1;
        rows.push((
            format!("{}", rank),
            "---".to_string(),
            "------".to_string(),
        ));
    }

    let measure_menu_text_width = |ui_scale: f32, text: &str| -> f32 {
        let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);
        let mut w = 0.0f32;
        for ch in text.chars() {
            if ch == ' ' {
                w += (MENU_FONT_SPACE_W * s).round();
                continue;
            }
            if let Some(g) = menu_glyph(ch) {
                w += (g.advance * s).round();
            }
        }
        w.max(1.0)
    };

    // CALCULATE AVAILABLE SPACE FOR SCORES LIST
    let content_start_y = top_red + banner_h;
    let bottom_pad = (6.0 * ui_scale).round();
    let list_top_pad = (12.0 * ui_scale).round();
    let list_top = content_start_y + list_top_pad;
    
    // Calculate row spacing that fits all 10 entries
    let row_spacing_available = (h - list_top - bottom_pad).max(1.0);
    let row_step = if rows.len() > 1 {
        (row_spacing_available / rows.len() as f32).floor().max(1.0)
    } else {
        (13.0 * ui_scale).round()
    };

    // Column positions (in 320x200 space)
    let rank_right = (72.0 * ui_scale).round();
    let name_left = (88.0 * ui_scale).round();
    let score_right = (272.0 * ui_scale).round();

    for (i, (rank, name, score)) in rows.iter().enumerate() {
        let y = (list_top + (i as f32) * row_step).round();

        let rank_w = measure_menu_text_width(ui_scale, rank);
        let score_w = measure_menu_text_width(ui_scale, score);

        let rank_x = (rank_right - rank_w).round().max(0.0);
        let score_x = (score_right - score_w).round().max(0.0);

        spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_yellow.clone(),
            rank_x,
            y,
            ui_scale,
            rank,
            Visibility::Visible,
        );

        spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_yellow.clone(),
            name_left,
            y,
            ui_scale,
            name,
            Visibility::Visible,
        );

        spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_yellow.clone(),
            score_x,
            y,
            ui_scale,
            score,
            Visibility::Visible,
        );
    }
}

fn spawn_menu_hint(
    commands: &mut Commands,
    asset_server: &AssetServer,
    w: f32,
    h: f32,
    imgs: &SplashImages,
    from_pause: bool,
) {
    let banner = asset_server.load(MENU_BANNER_PATH);
    let hint = asset_server.load(MENU_HINT_PATH);
    let cursor_light = asset_server.load(MENU_CURSOR_LIGHT_PATH);
    let cursor_dark = asset_server.load(MENU_CURSOR_DARK_PATH);

    let ui_scale = (w / BASE_W).round().max(1.0);

    // ---- Banner Geometry ----
    let banner_native_h = 48.0;
    let top_red = (3.0 * ui_scale).round();

    let banner_x = 0.0;
    let banner_y = top_red;
    let banner_w = w;
    let banner_h = (banner_native_h * ui_scale).round();

    // ---- Hint Placement ----
    let hint_native_w = 103.0;
    let hint_native_h = 12.0;
    let hint_bottom_pad = 6.0;

    let hint_w = (hint_native_w * ui_scale).round();
    let hint_h = (hint_native_h * ui_scale).round();
    let hint_x = ((BASE_W - hint_native_w) * 0.5 * ui_scale).round();
    let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();

    // ---- Menu Panel + Items ----
    let labels: &[&str] = if from_pause {
        &MENU_LABELS_PAUSE
    } else {
        &MENU_LABELS_MAIN
    };

    let row_count = labels.len();

    let panel_left = (76.0 * ui_scale).round();
    let panel_top = (55.0 * ui_scale).round();
    let panel_w = (178.0 * ui_scale).round();

    let cursor_w = (19.0 * ui_scale).round();
    let cursor_h = (10.0 * ui_scale).round();

    let cursor_x = (panel_left + (18.0 * ui_scale).round()).round();
    let cursor_y0 = (MENU_CURSOR_TOP * ui_scale).round();

    let text_x = (cursor_x + cursor_w + (6.0 * ui_scale).round()).round();
    let row_h = (MENU_ITEM_H * ui_scale).round();
    let text_y0 = (cursor_y0 - (2.0 * ui_scale).round()).round();

    let pad_y = (8.0 * ui_scale).round();
    let desired_panel_h = (pad_y * 2.0 + row_h * row_count as f32).round();

    // Never Overlap Hint
    let max_panel_h = (hint_y - (2.0 * ui_scale).round() - panel_top).max(1.0);
    let panel_h = desired_panel_h.min(max_panel_h).max(1.0);

    // ---- Root + Canvas ----
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

    // ---- Full-Width Banner ----
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

    // ---- Darker-Red Background Menu Panel with Sunken Border ----
    let border_w = (2.0 * ui_scale).round().max(1.0);

    // Main panel background
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.40, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Top shadow (darker - makes it look recessed)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Left shadow (darker)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.20, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Bottom highlight (lighter - the "light source")
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left),
            top: Val::Px(panel_top + panel_h - border_w),
            width: Val::Px(panel_w),
            height: Val::Px(border_w),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // Right highlight (lighter)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(panel_left + panel_w - border_w),
            top: Val::Px(panel_top),
            width: Val::Px(border_w),
            height: Val::Px(panel_h),
            ..default()
        },
        BackgroundColor(Color::srgb(0.70, 0.0, 0.0)),
        ChildOf(canvas),
    ));

    // ---- Menu Text ----
    for (row_idx, &label) in labels.iter().enumerate() {
        let y = (text_y0 + row_idx as f32 * row_h).round();

        // Pause menu: "Return to Game" Always Yellow
        if from_pause && label == "Return to Game" {
            spawn_menu_bitmap_text(
                commands,
                canvas,
                imgs.menu_font_yellow.clone(),
                text_x,
                y,
                ui_scale,
                label,
                Visibility::Visible,
            );
            continue;
        }

        // Default Cursor Starts at Top
        let is_selected = row_idx == 0;

        let gray_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_gray.clone(),
            text_x,
            y,
            ui_scale,
            label,
            if is_selected { Visibility::Hidden } else { Visibility::Visible },
        );
        commands
            .entity(gray_run)
            .insert((EpisodeItem { idx: row_idx }, EpisodeTextVariant { selected: false }));

        let white_run = spawn_menu_bitmap_text(
            commands,
            canvas,
            imgs.menu_font_white.clone(),
            text_x,
            y,
            ui_scale,
            label,
            if is_selected { Visibility::Visible } else { Visibility::Hidden },
        );
        commands
            .entity(white_run)
            .insert((EpisodeItem { idx: row_idx }, EpisodeTextVariant { selected: true }));
    }

    // ---- Gun Cursor ----
    commands.spawn((
        MenuCursor,
        MenuCursorLight,
        Visibility::Visible,
        ImageNode::new(cursor_light),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(cursor_x),
            top: Val::Px(cursor_y0),
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
            top: Val::Px(cursor_y0),
            width: Val::Px(cursor_w),
            height: Val::Px(cursor_h),
            ..default()
        },
        ChildOf(canvas),
    ));

    // ---- Bottom Hint ----
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

fn splash_advance_on_any_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    input: SplashAdvanceInput,
    time: Res<Time>,
    mut resources: SplashResources,
    mut menu: Local<MenuLocalState>,
    mut new_game: ResMut<crate::ui::sync::NewGameRequested>,
    mut episode_tally: ResMut<EpisodeVictoryTally>,
    mut current_level: ResMut<davelib::level::CurrentLevel>,
    mut episode: Local<EpisodeLocalState>,
    mut skill: Local<SkillLocalState>,
    mut skill_level: ResMut<davelib::skill::SkillLevel>,
    mut sfx: MessageWriter<PlaySfx>,
    mut app_exit: MessageWriter<bevy::app::AppExit>,
    mut q: SplashAdvanceQueries,
    mut change_view: Local<ChangeViewLocalState>,
) {
    let keyboard = &*input.keyboard;
    let mouse = &*input.mouse;
    let Some(win) = q.q_win.iter().next() else { return; };

    let (w, h) = compute_scaled_size(win.width(), win.height());
    let scale = w / BASE_W;

    let any_key = keyboard.get_just_pressed().len() > 0 || mouse.get_just_pressed().len() > 0;

    match *resources.step {
        SplashStep::Splash0 => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Splash;

            let Some(imgs) = resources.imgs.as_ref() else { return; };

            if q.q_splash_roots.iter().next().is_none() {
                spawn_splash_ui(
                    &mut commands,
                    imgs.splash0.clone(),
                    w,
                    h,
                    Some(imgs.menu_font_white.clone()),
                );
            }

            if any_key {
                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                spawn_splash_ui(&mut commands, imgs.splash1.clone(), w, h, None);
                *resources.step = SplashStep::Splash1;
            }
        }

        SplashStep::Splash1 => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Splash;

            let Some(imgs) = resources.imgs.as_ref() else { return; };

            if q.q_splash_roots.iter().next().is_none() {
                spawn_splash_ui(&mut commands, imgs.splash1.clone(), w, h, None);
            }

            if any_key {
                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                spawn_menu_hint(&mut commands, &asset_server, w, h, imgs, false);
                menu.reset();
                *resources.step = SplashStep::Menu;
                resources.music_mode.0 = MusicModeKind::Menu;
            }
        }

        SplashStep::PauseMenu | SplashStep::Menu => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Menu;

            let Some(imgs) = resources.imgs.as_ref() else { return; };

            let is_pause = *resources.step == SplashStep::PauseMenu;

            let item_count = if is_pause {
                MENU_ACTIONS_PAUSE.len()
            } else {
                MENU_ACTIONS_MAIN.len()
            };

            if item_count == 0 {
                return;
            }

            menu.selection = menu.selection.min(item_count - 1);

            // Ensure Menu UI Exists
            if q.q_splash_roots.iter().next().is_none() {
                spawn_menu_hint(&mut commands, &asset_server, w, h, imgs, is_pause);
                menu.reset();
                menu.selection = menu.selection.min(item_count - 1);
            }

            // Navigation
            let mut moved = false;
            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                if menu.selection > 0 {
                    menu.selection -= 1;
                } else {
                    menu.selection = item_count - 1;
                }
                moved = true;
            }
            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                menu.selection = (menu.selection + 1) % item_count;
                moved = true;
            }
            if moved {
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
            }

            // Update Item Visibility
            for (item, variant, mut vis) in q.q_episode_items.iter_mut() {
                let want_selected = item.idx == menu.selection;
                *vis = if variant.selected == want_selected {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }

            // Cursor Blink
            if menu.blink.tick(time.delta()).just_finished() {
                menu.blink_light = !menu.blink_light;
            }

            // Cursor Position Matches spawn_menu_hint
            let ui_scale = (w / BASE_W).round().max(1.0);
            let panel_left = (76.0 * ui_scale).round();
            let cursor_w = (19.0 * ui_scale).round();
            let cursor_x = (panel_left + (18.0 * ui_scale).round()).round();

            let row_h = (MENU_ITEM_H * ui_scale).round();
            let cursor_y0 = (MENU_CURSOR_TOP * ui_scale).round();
            let cursor_y = (cursor_y0 + menu.selection as f32 * row_h).round();

            for mut node in q.q_node.iter_mut() {
                node.left = Val::Px(cursor_x);
                node.top = Val::Px(cursor_y);
                node.width = Val::Px(cursor_w);
            }

            for mut v in q.q_cursor_light.iter_mut() {
                *v = if menu.blink_light { Visibility::Visible } else { Visibility::Hidden };
            }
            for mut v in q.q_cursor_dark.iter_mut() {
                *v = if menu.blink_light { Visibility::Hidden } else { Visibility::Visible };
            }

            // Activate Selection
            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::NumpadEnter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                sfx.write(PlaySfx { kind: SfxKind::MenuSelect, pos: Vec3::ZERO });

                let action = if is_pause {
                    MENU_ACTIONS_PAUSE[menu.selection]
                } else {
                    MENU_ACTIONS_MAIN[menu.selection]
                };

                match action {
                    MenuAction::BackToGame => {
                        for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                        *resources.step = SplashStep::Done;
                        resources.lock.0 = false;
                        resources.music_mode.0 = MusicModeKind::Gameplay;
                    }

                    MenuAction::NewGame => {
                        for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                        episode.selection = 0;
                        episode.from_pause = is_pause;

                        if let Some(imgs) = resources.imgs.as_ref() {
                            spawn_episode_select_ui(
                                &mut commands,
                                &asset_server,
                                w, h, scale,
                                imgs,
                                episode.selection,
                            );
                            *resources.step = SplashStep::EpisodeSelect;
                        }
                    }

                    MenuAction::Sound => {}
                    
                    MenuAction::Control => {}

                    MenuAction::ChangeView => {
                        for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                        change_view.selection = 0;
                        change_view.res_submenu_open = false;
                        change_view.needs_respawn = false;
                        change_view.from_pause = is_pause;

                        if let Some(imgs) = resources.imgs.as_ref() {
                            spawn_change_view_ui(
                                &mut commands,
                                &asset_server,
                                w, h, scale,
                                imgs,
                                change_view.selection,
                                &resources.video_settings,
                                &resources.res_list,
                            );

                            *resources.step = SplashStep::ChangeView;
                            resources.music_mode.0 = MusicModeKind::Menu;
                        }
                    }

                    MenuAction::ViewScores => {
                        let Some(imgs) = resources.imgs.as_ref() else { return; };

                        episode.from_pause = is_pause;
                        for e in q.q_splash_roots.iter() {
                            commands.entity(e).despawn();
                        }

                        let high_scores = &*resources.high_scores;
                        spawn_scores_ui(&mut commands, asset_server.as_ref(), w, h, imgs, high_scores);

                        menu.reset();
                        *resources.step = SplashStep::Scores;
                        resources.music_mode.0 = MusicModeKind::Scores;
                    }

                    MenuAction::Quit => {
                        app_exit.write(bevy::app::AppExit::Success);
                    }
                }
            }
        }

        SplashStep::EpisodeSelect => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Menu;

            if keyboard.just_pressed(KeyCode::Escape) {
                sfx.write(PlaySfx { kind: SfxKind::MenuBack, pos: Vec3::ZERO });

                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                if let Some(imgs) = resources.imgs.as_ref() {
                    let back_to_pause = episode.from_pause;
                    episode.from_pause = false;

                    spawn_menu_hint(&mut commands, &asset_server, w, h, imgs, back_to_pause);
                    menu.reset();
                    *resources.step = if back_to_pause { SplashStep::PauseMenu } else { SplashStep::Menu };
                }
                return;
            }

            let mut moved = false;

            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                if episode.selection > 0 { episode.selection -= 1; } else { episode.selection = 5; }
                moved = true;
            }
            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                episode.selection = (episode.selection + 1) % 6;
                moved = true;
            }
            if moved {
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
            }

            for (item, variant, mut vis) in q.q_episode_items.iter_mut() {
                let want_selected = item.idx == episode.selection;
                *vis = if variant.selected == want_selected { Visibility::Visible } else { Visibility::Hidden };
            }

            let blink_on = (time.elapsed_secs() / 0.2).floor() as i32 % 2 == 0;

            let ui_scale = (w / BASE_W).round().max(1.0);
            let panel_left = (18.0 * ui_scale).round();
            let cursor_x = (panel_left + (6.0 * ui_scale).round()).round();

            let cursor_h = (10.0 * ui_scale).round();
            let sel_row_top = (EP_LIST_TOP + episode.selection as f32 * EP_ROW_H) * ui_scale;
            let cursor_y = (sel_row_top + ((EP_THUMB_H * ui_scale - cursor_h) * 0.5)).round();

            for mut node in q.q_node.iter_mut() {
                node.left = Val::Px(cursor_x);
                node.top = Val::Px(cursor_y);
            }
            for mut v in q.q_cursor_light.iter_mut() {
                *v = if blink_on { Visibility::Visible } else { Visibility::Hidden };
            }
            for mut v in q.q_cursor_dark.iter_mut() {
                *v = if blink_on { Visibility::Hidden } else { Visibility::Visible };
            }

            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::NumpadEnter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                let episode_num = (episode.selection + 1) as u8;

                sfx.write(PlaySfx { kind: SfxKind::MenuSelect, pos: Vec3::ZERO });

                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                skill.selection = 2;
                skill.episode_num = episode_num;

                if let Some(imgs) = resources.imgs.as_ref() {
                    spawn_skill_select_ui(
                        &mut commands,
                        &asset_server,
                        w, h, scale,
                        imgs,
                        skill.selection,
                    );
                    *resources.step = SplashStep::SkillSelect;
                }
            }
        }

        SplashStep::SkillSelect => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Menu;

            let Some(imgs) = resources.imgs.as_ref() else { return; };

            if keyboard.just_pressed(KeyCode::Escape) {
                sfx.write(PlaySfx { kind: SfxKind::MenuBack, pos: Vec3::ZERO });

                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                spawn_episode_select_ui(
                    &mut commands,
                    &asset_server,
                    w, h, scale,
                    imgs,
                    episode.selection,
                );
                *resources.step = SplashStep::EpisodeSelect;
                return;
            }

            let mut moved = false;

            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                if skill.selection > 0 { skill.selection -= 1; } else { skill.selection = 3; }
                moved = true;
            }

            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                skill.selection = (skill.selection + 1) % 4;
                moved = true;
            }

            if moved {
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
            }

            for (item, variant, mut vis) in q.q_skill_items.iter_mut() {
                let want_selected = item.idx == skill.selection;
                *vis = if variant.selected == want_selected { Visibility::Visible } else { Visibility::Hidden };
            }

            if moved {
                for mut img in q.q_skill_face.iter_mut() {
                    *img = ImageNode::new(imgs.skill_faces[skill.selection].clone());
                }
            }

            let blink_on = (time.elapsed_secs() / 0.2).floor() as i32 % 2 == 0;

            let ui_scale = (w / BASE_W).round().max(1.0);

            let desired_panel_w = (236.0 * ui_scale).round().max(1.0);
            let panel_left = ((w - desired_panel_w) * 0.5).round().max(0.0);
            let panel_top = (58.0 * ui_scale).round();

            let cursor_w = (19.0 * ui_scale).round();
            let cursor_x = (panel_left + (14.0 * ui_scale).round()).round();

            let row_h = (MENU_ITEM_H * ui_scale).round();
            let cursor_y0 = (panel_top + (14.0 * ui_scale).round()).round();
            let cursor_y = (cursor_y0 + skill.selection as f32 * row_h).round();

            for mut node in q.q_node.iter_mut() {
                node.left = Val::Px(cursor_x);
                node.top = Val::Px(cursor_y);
                node.width = Val::Px(cursor_w);
            }

            for mut v in q.q_cursor_light.iter_mut() {
                *v = if blink_on { Visibility::Visible } else { Visibility::Hidden };
            }
            for mut v in q.q_cursor_dark.iter_mut() {
                *v = if blink_on { Visibility::Hidden } else { Visibility::Visible };
            }

            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::NumpadEnter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                let episode_num = skill.episode_num.max(1).min(6);

                sfx.write(PlaySfx { kind: SfxKind::MenuSelect, pos: Vec3::ZERO });

                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                *skill_level = davelib::skill::SkillLevel::from_selection(skill.selection);
                new_game.0 = true;
                current_level.0 = davelib::level::LevelId::first_level_of_episode(episode_num);

                info!(
                    "Menu: selected difficulty {} (idx={}) episode={}",
                    skill_level.name(),
                    skill.selection,
                    episode_num
                );

                begin_get_psyched_loading(
                    &mut commands,
                    &asset_server,
                    win,
                    &mut *resources.psyched,
                    &mut *resources.lock,
                    &mut *resources.music_mode,
                );

                resources.lock.0 = false;
                resources.music_mode.0 = MusicModeKind::Gameplay;

                episode.from_pause = false;
                *resources.step = SplashStep::Done;
            }
        }

        SplashStep::ChangeView => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Menu;

            let Some(imgs) = resources.imgs.as_ref() else { return; };

            // Deferred respawn: after a display mode change, the window size
            // updates on the next frame. Respawn the menu UI with correct dims.
            if change_view.needs_respawn {
                change_view.needs_respawn = false;

                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                spawn_change_view_ui(
                    &mut commands, &asset_server,
                    w, h, scale, imgs,
                    change_view.selection,
                    &resources.video_settings, &resources.res_list,
                );
                return;
            }

            // Build the dynamic item list to know what kind each row is
            let items = build_change_view_items(&resources.video_settings, &resources.res_list);
            let item_count = items.len();

            // Clamp selection in case item count changed (e.g. Resolution row appeared/disappeared)
            if change_view.selection >= item_count {
                change_view.selection = item_count.saturating_sub(1);
            }

            let current_kind = items.get(change_view.selection).map(|(k, _)| *k);

            // --- Resolution Sub-Menu Mode ---
            if change_view.res_submenu_open {
                if keyboard.just_pressed(KeyCode::Escape) {
                    sfx.write(PlaySfx { kind: SfxKind::MenuBack, pos: Vec3::ZERO });
                    change_view.res_submenu_open = false;

                    for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                    spawn_change_view_ui(
                        &mut commands, &asset_server,
                        w, h, scale, imgs,
                        change_view.selection,
                        &resources.video_settings, &resources.res_list,
                    );
                    return;
                }

                let res_count = resources.res_list.entries.len();

                if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                    if change_view.res_submenu_idx > 0 {
                        change_view.res_submenu_idx -= 1;
                    } else {
                        change_view.res_submenu_idx = res_count.saturating_sub(1);
                    }
                    sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
                }

                if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                    change_view.res_submenu_idx = (change_view.res_submenu_idx + 1) % res_count;
                    sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
                }

                if keyboard.just_pressed(KeyCode::Enter)
                    || keyboard.just_pressed(KeyCode::NumpadEnter)
                    || keyboard.just_pressed(KeyCode::Space)
                {
                    sfx.write(PlaySfx { kind: SfxKind::MenuSelect, pos: Vec3::ZERO });

                    if let Some(&(rw, rh)) = resources.res_list.entries.get(change_view.res_submenu_idx) {
                        resources.video_settings.resolution = (rw, rh);
                    }

                    change_view.res_submenu_open = false;

                    for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                    spawn_change_view_ui(
                        &mut commands, &asset_server,
                        w, h, scale, imgs,
                        change_view.selection,
                        &resources.video_settings, &resources.res_list,
                    );
                    return;
                }

                // Update highlight/cursor for sub-menu
                // (Resolution sub-menu reuses the same ChangeViewItem query
                //  since we respawn UI when entering/leaving sub-menu)
                for (item, variant, mut vis) in q.q_change_view_items.iter_mut() {
                    let want_selected = item.idx == change_view.res_submenu_idx;
                    *vis = if variant.selected == want_selected { Visibility::Visible } else { Visibility::Hidden };
                }

                let blink_on = (time.elapsed_secs() / 0.2).floor() as i32 % 2 == 0;
                for mut v in q.q_cursor_light.iter_mut() {
                    *v = if blink_on { Visibility::Visible } else { Visibility::Hidden };
                }
                for mut v in q.q_cursor_dark.iter_mut() {
                    *v = if blink_on { Visibility::Hidden } else { Visibility::Visible };
                }

                // Cursor positioning for sub-menu
                let ui_scale = (w / BASE_W).round().max(1.0);
                let hint_native_h = 12.0;
                let hint_bottom_pad = 6.0;
                let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();
                let panel_left = (18.0 * ui_scale).round();
                let panel_top = ((EP_LIST_TOP - 4.0) * ui_scale).round();
                let panel_right = ((BASE_W - 18.0) * ui_scale).round();
                let panel_w = (panel_right - panel_left).max(1.0);
                let panel_bottom = (hint_y - (2.0 * ui_scale).round()).max(panel_top + 1.0);
                let panel_h = (panel_bottom - panel_top).max(1.0);
                let cursor_w = (19.0 * ui_scale).round();
                let cursor_h = (10.0 * ui_scale).round();
                let row_h = (16.0 * ui_scale).round().max(1.0);
                let sub_count = resources.res_list.entries.len();
                let list_h = (sub_count as f32 * row_h).round();
                let list_top = (panel_top + ((panel_h - list_h) * 0.5)).round();

                // Measure max width of sub-menu items
                let mut max_item_w = 0.0f32;
                for idx in 0..sub_count {
                    let label = resources.res_list.label_at(idx);
                    let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);
                    let mut lw = 0.0f32;
                    for ch in label.chars() {
                        if ch == ' ' { lw += (MENU_FONT_SPACE_W * s).round(); continue; }
                        if let Some(g) = menu_glyph(ch) { lw += (g.advance * s).round(); }
                    }
                    max_item_w = max_item_w.max(lw.max(1.0));
                }

                let text_x = (panel_left + ((panel_w - max_item_w) * 0.5)).round().max(0.0);
                let cursor_x = (text_x - cursor_w - (8.0 * ui_scale).round()).round().max(0.0);
                let cursor_y = (list_top + change_view.res_submenu_idx as f32 * row_h + ((row_h - cursor_h) * 0.5)).round();

                for mut node in q.q_node.iter_mut() {
                    node.left = Val::Px(cursor_x);
                    node.top = Val::Px(cursor_y);
                }

                return;
            }

            // --- Normal Change View Mode ---
            if keyboard.just_pressed(KeyCode::Escape) {
                sfx.write(PlaySfx { kind: SfxKind::MenuBack, pos: Vec3::ZERO });

                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                let back_to_pause = change_view.from_pause;
                change_view.from_pause = false;
                spawn_menu_hint(&mut commands, &asset_server, w, h, imgs, back_to_pause);
                menu.reset();
                *resources.step = if back_to_pause { SplashStep::PauseMenu } else { SplashStep::Menu };
                return;
            }

            let mut moved = false;

            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                if change_view.selection > 0 { change_view.selection -= 1; } else { change_view.selection = item_count - 1; }
                moved = true;
            }

            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                change_view.selection = (change_view.selection + 1) % item_count;
                moved = true;
            }

            // Left/Right for inline-adjustable items (with hold-to-accelerate)
            // A/D trigger a single nudge on press; arrow keys support hold-repeat
            let left_just = keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA);
            let right_just = keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);
            let left_held = keyboard.pressed(KeyCode::ArrowLeft);
            let right_held = keyboard.pressed(KeyCode::ArrowRight);

            let is_nudgeable = matches!(
                current_kind,
                Some(ChangeViewKind::Fov) | Some(ChangeViewKind::ViewSize)
            );

            // Track hold state for nudgeable items
            if is_nudgeable && (left_held || right_held) {
                let dir: i8 = if right_held { 1 } else { -1 };

                if left_just || right_just {
                    // Fresh press: immediate first tick, reset hold state
                    change_view.hold_dir = dir;
                    change_view.hold_accum = 0.0;
                    change_view.hold_interval = HOLD_REPEAT_INITIAL;
                    change_view.hold_ticks = 0;
                } else if dir == change_view.hold_dir {
                    // Continuing hold: accumulate time
                    change_view.hold_accum += time.delta_secs();
                } else {
                    // Direction changed mid-hold
                    change_view.hold_dir = dir;
                    change_view.hold_accum = 0.0;
                    change_view.hold_interval = HOLD_REPEAT_INITIAL;
                    change_view.hold_ticks = 0;
                }
            } else {
                // Released
                change_view.hold_dir = 0;
                change_view.hold_accum = 0.0;
                change_view.hold_interval = HOLD_REPEAT_INITIAL;
                change_view.hold_ticks = 0;
            }

            // Determine how many nudge ticks to fire this frame
            let mut nudge_ticks: u32 = 0;

            if is_nudgeable && change_view.hold_dir != 0 {
                if left_just || right_just {
                    // First press always fires one tick immediately
                    nudge_ticks = 1;
                } else {
                    // Hold repeat: fire ticks based on accumulated time
                    while change_view.hold_accum >= change_view.hold_interval {
                        change_view.hold_accum -= change_view.hold_interval;
                        nudge_ticks += 1;
                        change_view.hold_ticks += 1;
                        // Accelerate: shrink interval, floor at FAST
                        change_view.hold_interval = (change_view.hold_interval * HOLD_REPEAT_ACCEL)
                            .max(HOLD_REPEAT_FAST);
                    }
                }
            }

            // Handle non-nudgeable left/right (DisplayMode cycles on just_pressed only)
            let left_pressed = left_just;
            let right_pressed = right_just;

            let mut value_changed = false;

            if left_pressed || right_pressed {
                match current_kind {
                    Some(ChangeViewKind::DisplayMode) => {
                        resources.video_settings.display_mode = if right_pressed {
                            resources.video_settings.display_mode.next()
                        } else {
                            resources.video_settings.display_mode.prev()
                        };
                        // Don't respawn now — window dimensions will change next
                        // frame after apply system runs. Set flag for deferred respawn.
                        change_view.needs_respawn = true;
                        sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
                        return;
                    }
                    _ => {}
                }
            }

            // Apply nudge ticks for FOV / View Size
            if nudge_ticks > 0 {
                let dir = change_view.hold_dir;
                for _ in 0..nudge_ticks {
                    match current_kind {
                        Some(ChangeViewKind::Fov) => {
                            resources.video_settings.nudge_fov(dir as f32);
                        }
                        Some(ChangeViewKind::ViewSize) => {
                            resources.video_settings.nudge_view_size(dir);
                        }
                        _ => {}
                    }
                }
                value_changed = true;
            }

            if value_changed {
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });

                // Respawn UI to reflect new values and possibly changed item count
                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                // Rebuild items to get new count (Resolution row may appear/disappear)
                let new_items = build_change_view_items(&resources.video_settings, &resources.res_list);
                if change_view.selection >= new_items.len() {
                    change_view.selection = new_items.len().saturating_sub(1);
                }

                spawn_change_view_ui(
                    &mut commands, &asset_server,
                    w, h, scale, imgs,
                    change_view.selection,
                    &resources.video_settings, &resources.res_list,
                );
                return;
            }

            if moved {
                sfx.write(PlaySfx { kind: SfxKind::MenuMove, pos: Vec3::ZERO });
            }

            for (item, variant, mut vis) in q.q_change_view_items.iter_mut() {
                let want_selected = item.idx == change_view.selection;
                *vis = if variant.selected == want_selected { Visibility::Visible } else { Visibility::Hidden };
            }

            let blink_on = (time.elapsed_secs() / 0.2).floor() as i32 % 2 == 0;

            for mut v in q.q_cursor_light.iter_mut() {
                *v = if blink_on { Visibility::Visible } else { Visibility::Hidden };
            }
            for mut v in q.q_cursor_dark.iter_mut() {
                *v = if blink_on { Visibility::Hidden } else { Visibility::Visible };
            }

            // Only reposition cursor when user navigated (up/down).
            // spawn_change_view_ui already places it correctly at spawn time,
            // so redundant repositioning every frame causes drift during
            // display mode transitions when w/h are shifting.
            if moved {
                // Cursor Positioning Uses Same Math as spawn_change_view_ui
                let ui_scale = (w / BASE_W).round().max(1.0);

                let hint_native_h = 12.0;
                let hint_bottom_pad = 6.0;
                let hint_y = ((BASE_H - hint_native_h - hint_bottom_pad) * ui_scale).round();

                let panel_left = (18.0 * ui_scale).round();
                let panel_top = ((EP_LIST_TOP - 4.0) * ui_scale).round();
                let panel_right = ((BASE_W - 18.0) * ui_scale).round();
                let panel_w = (panel_right - panel_left).max(1.0);

                let panel_bottom = (hint_y - (2.0 * ui_scale).round()).max(panel_top + 1.0);
                let panel_h = (panel_bottom - panel_top).max(1.0);

                let cursor_w = (19.0 * ui_scale).round();
                let cursor_h = (10.0 * ui_scale).round();
                let row_h = (16.0 * ui_scale).round().max(1.0);

                let list_h = (item_count as f32 * row_h).round();
                let list_top = (panel_top + ((panel_h - list_h) * 0.5)).round();

                // Measure max width for cursor X positioning
                let item_labels: Vec<String> = items.iter().map(|(_, s)| s.clone()).collect();
                let mut max_item_w = 0.0f32;
                for t in &item_labels {
                    let s = (ui_scale * MENU_FONT_DRAW_SCALE).max(0.01);
                    let mut lw = 0.0f32;
                    for ch in t.chars() {
                        if ch == ' ' { lw += (MENU_FONT_SPACE_W * s).round(); continue; }
                        if let Some(g) = menu_glyph(ch) { lw += (g.advance * s).round(); }
                    }
                    max_item_w = max_item_w.max(lw.max(1.0));
                }

                let text_x = (panel_left + ((panel_w - max_item_w) * 0.5)).round().max(0.0);
                let cursor_x = (text_x - cursor_w - (8.0 * ui_scale).round()).round().max(0.0);

                let cursor_y = (list_top + change_view.selection as f32 * row_h + ((row_h - cursor_h) * 0.5)).round();

                for mut node in q.q_node.iter_mut() {
                    node.left = Val::Px(cursor_x);
                    node.top = Val::Px(cursor_y);
                }
            }

            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::NumpadEnter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                sfx.write(PlaySfx { kind: SfxKind::MenuSelect, pos: Vec3::ZERO });

                match current_kind {
                    Some(ChangeViewKind::Vsync) => {
                        resources.video_settings.vsync = !resources.video_settings.vsync;

                        for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                        spawn_change_view_ui(
                            &mut commands, &asset_server,
                            w, h, scale, imgs,
                            change_view.selection,
                            &resources.video_settings, &resources.res_list,
                        );
                    }

                    Some(ChangeViewKind::Resolution) => {
                        // Open the resolution sub-menu
                        change_view.res_submenu_open = true;
                        change_view.res_submenu_idx = resources.res_list.index_of(resources.video_settings.resolution);

                        for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }
                        spawn_resolution_submenu_ui(
                            &mut commands, &asset_server,
                            w, h, scale, imgs,
                            change_view.res_submenu_idx,
                            &resources.res_list,
                        );
                    }

                    Some(ChangeViewKind::Back) => {
                        for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                        let back_to_pause = change_view.from_pause;
                        change_view.from_pause = false;
                        spawn_menu_hint(&mut commands, &asset_server, w, h, imgs, back_to_pause);
                        menu.reset();
                        *resources.step = if back_to_pause { SplashStep::PauseMenu } else { SplashStep::Menu };
                    }

                    // DisplayMode, FOV, ViewSize are adjusted by Left/Right, Enter does nothing extra
                    _ => {}
                }
            }
        }

        SplashStep::NameEntry => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Scores;

            let Some(imgs) = resources.imgs.as_ref() else { return; };

            if !resources.name_entry.active {
                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }

                let high_scores = &*resources.high_scores;
                spawn_scores_ui(&mut commands, asset_server.as_ref(), w, h, imgs, high_scores);

                *resources.step = SplashStep::Scores;
                return;
            }

            if q.q_splash_roots.iter().next().is_none() {
                spawn_name_entry_ui(
                    &mut commands,
                    w,
                    h,
                    imgs,
                    resources.name_entry.rank,
                    &resources.name_entry.name,
                );
            }

            let keycode_to_letter = |kc: KeyCode| -> Option<char> {
                Some(match kc {
                    KeyCode::KeyA => 'A',
                    KeyCode::KeyB => 'B',
                    KeyCode::KeyC => 'C',
                    KeyCode::KeyD => 'D',
                    KeyCode::KeyE => 'E',
                    KeyCode::KeyF => 'F',
                    KeyCode::KeyG => 'G',
                    KeyCode::KeyH => 'H',
                    KeyCode::KeyI => 'I',
                    KeyCode::KeyJ => 'J',
                    KeyCode::KeyK => 'K',
                    KeyCode::KeyL => 'L',
                    KeyCode::KeyM => 'M',
                    KeyCode::KeyN => 'N',
                    KeyCode::KeyO => 'O',
                    KeyCode::KeyP => 'P',
                    KeyCode::KeyQ => 'Q',
                    KeyCode::KeyR => 'R',
                    KeyCode::KeyS => 'S',
                    KeyCode::KeyT => 'T',
                    KeyCode::KeyU => 'U',
                    KeyCode::KeyV => 'V',
                    KeyCode::KeyW => 'W',
                    KeyCode::KeyX => 'X',
                    KeyCode::KeyY => 'Y',
                    KeyCode::KeyZ => 'Z',
                    _ => return None,
                })
            };

            let mut changed = false;

            if keyboard.just_pressed(KeyCode::Backspace) {
                if !resources.name_entry.name.is_empty() {
                    resources.name_entry.name.pop();
                    changed = true;
                }
            }

            for &kc in keyboard.get_just_pressed() {
                let Some(ch) = keycode_to_letter(kc) else { continue; };

                if resources.name_entry.name.len() < 3 {
                    resources.name_entry.name.push(ch);
                    changed = true;
                }
            }

            resources.name_entry.cursor_pos = resources.name_entry.name.len().min(3);

            if changed {
                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }

                spawn_name_entry_ui(
                    &mut commands,
                    w,
                    h,
                    imgs,
                    resources.name_entry.rank,
                    &resources.name_entry.name,
                );
            }

            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter) {
                let name = resources.name_entry.name.clone();
                let score = resources.name_entry.score;
                let episode_num = resources.name_entry.episode;

                resources.high_scores.add(name, score, episode_num);

                resources.name_entry.active = false;
                resources.name_entry.name.clear();
                resources.name_entry.cursor_pos = 0;

                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }

                let high_scores = &*resources.high_scores;
                spawn_scores_ui(&mut commands, asset_server.as_ref(), w, h, imgs, high_scores);

                *resources.step = SplashStep::Scores;
            }
        }

        SplashStep::Scores => {
            if resources.name_entry.active {
                resources.name_entry.active = false;
                resources.name_entry.name.clear();
                resources.name_entry.cursor_pos = 0;
            }

            if any_key {
                let Some(imgs) = resources.imgs.as_ref() else { return; };

                let back_to_pause = episode.from_pause;
                episode.from_pause = false;

                for e in q.q_splash_roots.iter() {
                    commands.entity(e).despawn();
                }

                spawn_menu_hint(&mut commands, &asset_server, w, h, imgs, back_to_pause);
                menu.reset();

                *resources.step = if back_to_pause { SplashStep::PauseMenu } else { SplashStep::Menu };
                resources.lock.0 = true;
                resources.music_mode.0 = MusicModeKind::Menu;
            }
        }

        SplashStep::EpisodeVictory => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Scores;

            let Some(imgs) = resources.imgs.as_ref() else { return; };
            let Some(episode_end) = resources.episode_end.as_ref() else { return; };

            let episode_num = {
                let from_flow = resources.name_entry.episode;
                if (1..=6).contains(&from_flow) {
                    from_flow
                } else if (1..=6).contains(&resources.episode_stats.episode) {
                    resources.episode_stats.episode
                } else {
                    skill.episode_num.max(1).min(6)
                }
            };

            resources.name_entry.episode = episode_num;

            if q.q_splash_roots.iter().next().is_none() {
                spawn_episode_score_ui(
                    &mut commands,
                    imgs,
                    episode_end,
                    &*resources.episode_stats,
                    episode_num,
                    w,
                    h,
                    resources.hud.score,
                );

                let summary = resources.episode_stats.summary_for_episode(episode_num);
                episode_tally.begin(summary);
                return;
            }

            if any_key {
                if episode_tally.active {
                    episode_tally.force_finish();
                    return;
                }

                clear_splash_ui(&mut commands, &q.q_splash_roots);
                *resources.step = SplashStep::EpisodeEndText0;
            }
        }

        SplashStep::EpisodeEndText0 => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Scores;

            let Some(imgs) = resources.imgs.as_ref() else { return; };
            let Some(episode_end) = resources.episode_end.as_ref() else { return; };

            let episode_num = resources.name_entry.episode.max(1).min(6);

            if q.q_splash_roots.iter().next().is_none() {
                spawn_episode_end_text_ui(&mut commands, w, h, imgs, episode_end, episode_num, 0);
                return;
            }

            if any_key {
                clear_splash_ui(&mut commands, &q.q_splash_roots);
                *resources.step = SplashStep::EpisodeEndText1;
            }
        }

        SplashStep::EpisodeEndText1 => {
            resources.lock.0 = true;
            resources.music_mode.0 = MusicModeKind::Scores;

            let Some(imgs) = resources.imgs.as_ref() else { return; };
            let Some(episode_end) = resources.episode_end.as_ref() else { return; };

            let episode_num = resources.name_entry.episode.max(1).min(6);

            if q.q_splash_roots.iter().next().is_none() {
                spawn_episode_end_text_ui(&mut commands, w, h, imgs, episode_end, episode_num, 1);
                return;
            }

            if any_key {
                clear_splash_ui(&mut commands, &q.q_splash_roots);

                let score = resources.hud.score;

                if resources.high_scores.qualifies(score) {
                    resources.name_entry.active = true;
                    resources.name_entry.rank = high_score_rank_for(&resources.high_scores, score);
                    resources.name_entry.score = score;
                    resources.name_entry.episode = episode_num;
                    resources.name_entry.name.clear();
                    resources.name_entry.cursor_pos = 0;

                    *resources.step = SplashStep::NameEntry;
                } else {
                    spawn_scores_ui(&mut commands, asset_server.as_ref(), w, h, imgs, &resources.high_scores);
                    *resources.step = SplashStep::Scores;
                }
            }
        }

        SplashStep::Done => {
            if resources.death_overlay.active || resources.game_over.0 || resources.lock.0 {
                return;
            }

            if keyboard.just_pressed(KeyCode::Escape) {
                let Some(imgs) = resources.imgs.as_ref() else { return; };

                sfx.write(PlaySfx { kind: SfxKind::MenuBack, pos: Vec3::ZERO });

                resources.lock.0 = true;
                resources.music_mode.0 = MusicModeKind::Menu;

                for e in q.q_splash_roots.iter() { commands.entity(e).despawn(); }

                spawn_menu_hint(&mut commands, &asset_server, w, h, imgs, true);
                menu.reset();
                *resources.step = SplashStep::PauseMenu;
            }
        }
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
    let episode_thumbs_atlas = asset_server.load(EPISODE_THUMBS_ATLAS_PATH);

    let menu_font_white = asset_server.load(MENU_FONT_WHITE_PATH);
    let menu_font_gray = asset_server.load(MENU_FONT_GRAY_PATH);
    let menu_font_yellow = asset_server.load(MENU_FONT_YELLOW_PATH);
    let menu_font_black = asset_server.load(MENU_FONT_BLACK_PATH);

    let skill_face_0 = asset_server.load(SKILL_FACE_0_PATH);
    let skill_face_1 = asset_server.load(SKILL_FACE_1_PATH);
    let skill_face_2 = asset_server.load(SKILL_FACE_2_PATH);
    let skill_face_3 = asset_server.load(SKILL_FACE_3_PATH);

    commands.insert_resource(SplashImages {
        splash0,
        splash1,
        episode_thumbs_atlas,
        menu_font_white,
        menu_font_gray,
        menu_font_yellow,
        menu_font_black,
        skill_faces: [skill_face_0, skill_face_1, skill_face_2, skill_face_3],
    });

    commands.insert_resource(EpisodeEndImages {
        bj_victory_walk: [
            asset_server.load("textures/ui/episode_end/bj_victory_walk_0.png"),
            asset_server.load("textures/ui/episode_end/bj_victory_walk_1.png"),
            asset_server.load("textures/ui/episode_end/bj_victory_walk_2.png"),
            asset_server.load("textures/ui/episode_end/bj_victory_walk_3.png"),
        ],
        bj_victory_jump: [
            asset_server.load("textures/ui/episode_end/bj_victory_jump_0.png"),
            asset_server.load("textures/ui/episode_end/bj_victory_jump_1.png"),
            asset_server.load("textures/ui/episode_end/bj_victory_jump_2.png"),
            asset_server.load("textures/ui/episode_end/bj_victory_jump_3.png"),
        ],
        you_win: asset_server.load("textures/ui/episode_end/you_win.png"),
        chaingun_belt: asset_server.load("textures/ui/episode_end/bj_chaingun_belt.png"),
        episode_page1_pic: asset_server.load("textures/ui/episode_end/bj_chaingun.png"),
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

    // While GET PSYCHED is up, force controls locked (prevents mouse clicks from acting
    // on gameplay or UI underneath), even if other systems temporarily unlock
    lock.0 = true;

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
