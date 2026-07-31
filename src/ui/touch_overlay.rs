/*
Davenstein - by David Petnick

On-Screen Touch Controls

The Touch Input Source Has Always Hit-Tested Invisible Rectangles; This Module
Finally Draws Them. Every Rectangle Comes Straight From input::TouchLayout - the
Same Resource gather Tests Fingers Against - so a Control Can Never Be Drawn
Anywhere but Exactly Where It Works. The Overlay Renders and the Touch Source
Decides; Nothing Here Consumes, Claims, or Filters a Touch

One Mode Value Gates Everything
input::TouchUiMode Is Written Here Once per Frame, Before InputGather Runs, by
Reading the Same UI State the Menus Themselves Run On (SplashStep, Game Over, the
Intermission, the Death Fizzle, the Cheat Modal, the Control Lock). The Touch
Source Reads That Value to Decide Which Rectangles Are Live and This Module Reads
It to Decide Which Rectangles to Draw, so Visible and Tappable Cannot Drift Apart

Visibility Belongs to the Device, Existence Belongs to the Mode
The Tree Is Rebuilt When the Mode or the Layout Changes; Whether the Player SEES
It Follows ActiveInputDevice. A Desktop Player Never Meets These Buttons, a Phone
Player Boots Straight Into Them (See ActiveInputDevice::default), and Plugging a
Gamepad Into a Tablet Hides Them Until the Glass Is Touched Again

Drawn on the MenuUiCamera
Roots Spawned Here Carry No UiTargetCamera; route_window_ui_to_menu_camera Sends
Every Untargeted Top-Level Root to the Persistent Window-Space Menu Camera in
PostUpdate, Before Layout. Finger-Sized Means Window Pixels, Never Canvas Pixels
*/

use bevy::prelude::*;
// Imported Explicitly Rather Than Relying on the Prelude Glob, the Same Way
// hud.rs Pulls In UiTargetCamera: These ui_node Types Are Not Guaranteed to
// Reach Us Through bevy::prelude on This Version
use bevy::ui::{BorderColor, BorderRadius, GlobalZIndex};

use davelib::input::{
    ActiveInputDevice,
    TouchAssignments,
    TouchLayout,
    TouchUiMode,
};
use davelib::options::ControlSettings;
use davelib::player::PlayerControlLock;

use super::cheat_message::CheatMessageState;
use super::{DeathOverlay, GameOver, SplashStep};
use crate::level_complete::LevelComplete;

// Resting Look: Dark Glass With a Pale Border so the Controls Read on Both a
// Bright Splash Screen and a Dark Corridor Without Hiding Either
const BUTTON_FILL: Srgba = Srgba::new(0.0, 0.0, 0.0, 0.30);
const BUTTON_BORDER: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.55);

// Pressed Look: the Fill Flips Light and the Border Goes Solid, Which Survives
// Being Half-Covered by the Pressing Thumb Better Than a Colour Change Would
const BUTTON_FILL_PRESSED: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.25);
const BUTTON_BORDER_PRESSED: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.95);

const LABEL_COLOR: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.85);

// Region Hints Are Deliberately Fainter Than Buttons: They Teach, They Are Not
// Targets, and They Sit on Top of the Live Game View
const HINT_COLOR: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.30);

const STICK_BASE_FILL: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.08);
const STICK_BASE_BORDER: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.35);
const STICK_KNOB_FILL: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.45);

const BUTTON_BORDER_PX: f32 = 2.0;
const BUTTON_CORNER_PX: f32 = 10.0;

// Above Every Menu, Splash, and Intermission Root on the Menu Camera (They All
// Sit at the Default Global Index of Zero) but With Room Left Above for Anything
// That Must One Day Outrank the Controls
const OVERLAY_Z: i32 = 50;

// Root of the Whole Overlay Tree; Despawned and Respawned as One Unit
#[derive(Component)]
pub(super) struct TouchOverlayRoot;

// Which Layout Rectangle a Drawn Button Mirrors, Plus the Last Pressed State so
// Colour Writes Happen Only on Edges Instead of Dirtying the Renderer Every Frame
#[derive(Component)]
pub(super) struct TouchControlButton {
    kind: TouchControlKind,
    pressed: bool,
}

// The Floating Stick's Outer Ring and Inner Knob, Positioned Every Frame While a
// Finger Owns the Stick and Hidden the Instant It Lifts
#[derive(Component)]
pub(super) struct TouchStickBase;

#[derive(Component)]
pub(super) struct TouchStickKnob;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TouchControlKind {
    Fire,
    UseDoor,
    Weapon(u8),
    Pause,
    MenuUp,
    MenuDown,
    MenuLeft,
    MenuRight,
    MenuConfirm,
    MenuBack,
}

impl TouchControlKind {
    // The Single Source of Truth for Where a Button Lives Is the Layout the
    // Touch Source Hit-Tests; This Is Only the Lookup
    fn rect(self, layout: &TouchLayout) -> Rect {
        match self {
            TouchControlKind::Fire => layout.fire,
            TouchControlKind::UseDoor => layout.use_door,
            TouchControlKind::Weapon(slot) => {
                layout.weapons[usize::from(slot.saturating_sub(1)).min(3)]
            }
            TouchControlKind::Pause => layout.pause,
            TouchControlKind::MenuUp => layout.menu_up,
            TouchControlKind::MenuDown => layout.menu_down,
            TouchControlKind::MenuLeft => layout.menu_left,
            TouchControlKind::MenuRight => layout.menu_right,
            TouchControlKind::MenuConfirm => layout.menu_confirm,
            TouchControlKind::MenuBack => layout.menu_back,
        }
    }
}

// Decide Which Control Set the Glass Carries, From the Same State the Screens
// Themselves Run On. Pure so the Priority Ladder Below Can Be Unit Tested
//
// The Done Ladder Is Ordered by What the Player Is Actually Looking At:
//   - The Cheat Modal Freezes Play Behind It and Dismisses on Any Input
//   - Game Over Is Checked Before the Death Fizzle Because the Fizzle's Backdrop
//     Stays Active Underneath the Game Over Text, and Game Over Wants a Tap
//   - The Intermission Tally (LevelComplete) Advances on Confirm
//   - The Fizzle Itself Takes No Input at All, so the Glass Goes Dark
//   - Any Remaining Lock Is a Loading State (the Get-Psyched Screen): Dark
//   - Otherwise the Player Is Playing
fn derive_touch_ui_mode(
    step: SplashStep,
    touch_enabled: bool,
    cheat_active: bool,
    game_over: bool,
    mission_won: bool,
    death_active: bool,
    locked: bool,
) -> TouchUiMode {
    if !touch_enabled {
        return TouchUiMode::Off;
    }

    match step {
        SplashStep::Menu
        | SplashStep::PauseMenu
        | SplashStep::EpisodeSelect
        | SplashStep::SkillSelect
        | SplashStep::LoadSelect
        | SplashStep::SaveSelect
        | SplashStep::NameEntry
        | SplashStep::ChangeView
        | SplashStep::SoundOptions
        | SplashStep::ControlOptions
        | SplashStep::GameplayOptions
        | SplashStep::KeyBindings => TouchUiMode::Menu,

        SplashStep::Splash0
        | SplashStep::Splash1
        | SplashStep::Scores
        | SplashStep::EpisodeVictory
        | SplashStep::EpisodeEndText0
        | SplashStep::EpisodeEndText1 => TouchUiMode::Advance,

        SplashStep::Done => {
            if cheat_active {
                TouchUiMode::Advance
            } else if game_over {
                TouchUiMode::Advance
            } else if mission_won {
                TouchUiMode::Advance
            } else if death_active {
                TouchUiMode::Off
            } else if locked {
                TouchUiMode::Off
            } else {
                TouchUiMode::Gameplay
            }
        }
    }
}

// Write the Mode the Rest of the Frame Runs On. Ordered Before InputGather so the
// Touch Source and the Overlay Agree Within a Frame; Reads the Previous Frame's
// UI State, Which Is Safe for the Reasons Documented on TouchUiMode Itself
//
// The Write Is Guarded so Change Detection on the Resource Stays Meaningful -
// the Tree Rebuild Below Keys Off It
pub(super) fn sync_touch_ui_mode(
    step: Res<SplashStep>,
    controls: Res<ControlSettings>,
    cheat: Res<CheatMessageState>,
    game_over: Res<GameOver>,
    win: Res<LevelComplete>,
    death: Res<DeathOverlay>,
    lock: Option<Res<PlayerControlLock>>,
    mut mode: ResMut<TouchUiMode>,
) {
    let next = derive_touch_ui_mode(
        *step,
        controls.touch_enabled,
        cheat.is_active(),
        game_over.0,
        win.0,
        death.active,
        lock.map(|l| l.0).unwrap_or(false),
    );

    if *mode != next {
        *mode = next;
    }
}

// Rebuild the Overlay Tree Whenever What It Shows Would Change: a Different
// Control Set, or the Same Set on Different Geometry (Resize, UI Scale). Despawn
// and Respawn as a Unit, the Same Pattern Every Menu Screen in splash.rs Uses -
// These Are a Handful of Nodes and the Edges Are Rare
pub(super) fn sync_touch_overlay_tree(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mode: Res<TouchUiMode>,
    layout: Res<TouchLayout>,
    device: Res<ActiveInputDevice>,
    q_roots: Query<Entity, With<TouchOverlayRoot>>,
) {
    if !mode.is_changed() && !layout.is_changed() && !q_roots.is_empty() {
        return;
    }

    for root in q_roots.iter() {
        commands.entity(root).despawn();
    }

    // Spawned Visible or Hidden According to the Device That Is Already Driving,
    // Rather Than Always-Hidden, so a Touch Player Never Sees a One-Frame Blink
    // of Missing Controls on Every Rebuild
    let visibility = if *device == ActiveInputDevice::Touch {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    let root = commands
        .spawn((
            TouchOverlayRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(OVERLAY_Z),
            visibility,
        ))
        .id();

    // No Geometry Yet Means Nothing Sensible to Draw; the Layout's First Real
    // Compute Marks the Resource Changed and Rebuilds This Tree With Contents
    if !layout.is_ready() {
        return;
    }

    let font: Handle<Font> = asset_server.load("fonts/font.ttf");

    match *mode {
        TouchUiMode::Gameplay => {
            spawn_button(&mut commands, root, &font, TouchControlKind::Fire, &layout, "FIRE");
            spawn_button(&mut commands, root, &font, TouchControlKind::UseDoor, &layout, "USE");
            spawn_button(&mut commands, root, &font, TouchControlKind::Pause, &layout, "MENU");
            for slot in 1..=4u8 {
                spawn_button(
                    &mut commands,
                    root,
                    &font,
                    TouchControlKind::Weapon(slot),
                    &layout,
                    match slot {
                        1 => "1",
                        2 => "2",
                        3 => "3",
                        _ => "4",
                    },
                );
            }

            spawn_gameplay_hints(&mut commands, root, &font, &layout);
            spawn_stick_visual(&mut commands, root, &layout);
        }

        TouchUiMode::Menu => {
            spawn_button(&mut commands, root, &font, TouchControlKind::MenuUp, &layout, "UP");
            spawn_button(&mut commands, root, &font, TouchControlKind::MenuDown, &layout, "DOWN");
            spawn_button(&mut commands, root, &font, TouchControlKind::MenuLeft, &layout, "LEFT");
            spawn_button(&mut commands, root, &font, TouchControlKind::MenuRight, &layout, "RIGHT");
            spawn_button(&mut commands, root, &font, TouchControlKind::MenuConfirm, &layout, "ENTER");
            spawn_button(&mut commands, root, &font, TouchControlKind::MenuBack, &layout, "BACK");
        }

        // Any-Input Screens (Splash, Score Tables, Level-Complete Tally, Game Over,
        // Episode Text): the Whole Screen Advances on a Tap, so No Hint or Button Is
        // Drawn. The Bare Root Just Waits for the Tap, Same as Off. The Old "TAP TO
        // CONTINUE" Chip Sat Bottom-Centre and Crowded the Tally Numbers and Splash
        // Art; Since the Gesture Is Discoverable and Universal Here, the Cleanest UI
        // Is None
        TouchUiMode::Advance => {}

        // A Dark Glass Draws Nothing; the Bare Root Waits for the Next Mode
        TouchUiMode::Off => {}
    }
}

// One Drawn Button Mirroring One Layout Rectangle, With a Centred Label Sized
// From the Rectangle so the Same Code Serves a Huge Fire Pad and a Small Slot Key
fn spawn_button(
    commands: &mut Commands,
    root: Entity,
    font: &Handle<Font>,
    kind: TouchControlKind,
    layout: &TouchLayout,
    label: &str,
) {
    let rect = kind.rect(layout);
    let label_px = (rect.height() * 0.26).clamp(12.0, 30.0);

    let button = commands
        .spawn((
            TouchControlButton { kind, pressed: false },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(rect.min.x),
                top: Val::Px(rect.min.y),
                width: Val::Px(rect.width()),
                height: Val::Px(rect.height()),
                border: UiRect::all(Val::Px(BUTTON_BORDER_PX)),
                // In Bevy 0.19 the Corner Radius Is a Field on Node, Not a
                // Standalone Component: BorderRadius Does Not Derive Component
                border_radius: BorderRadius::all(Val::Px(BUTTON_CORNER_PX)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_FILL.into()),
            BorderColor::all(Color::from(BUTTON_BORDER)),
            ChildOf(root),
        ))
        .id();

    commands.spawn((
        Text::new(label),
        TextFont {
            font: FontSource::Handle(font.clone()),
            font_size: FontSize::Px(label_px),
            ..default()
        },
        TextColor(LABEL_COLOR.into()),
        TextLayout::justify(Justify::Center),
        ChildOf(button),
    ));
}

// Faint Teaching Labels for the Two Invisible Regions: Where the Stick Will
// Appear Under the Thumb, and That the Rest of the Glass Turns the View. Pure
// Text, No Chrome, so They Read as Ground Markings Rather Than Buttons
fn spawn_gameplay_hints(
    commands: &mut Commands,
    root: Entity,
    font: &Handle<Font>,
    layout: &TouchLayout,
) {
    let window = layout.window_size;

    let hints = [
        ("MOVE", layout.stick_region.center().x, window.y * 0.62),
        (
            "LOOK",
            (layout.stick_region.max.x + window.x) * 0.5,
            window.y * 0.45,
        ),
    ];

    for (text, centre_x, centre_y) in hints {
        let hint = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(centre_x - 110.0),
                    top: Val::Px(centre_y - 12.0),
                    width: Val::Px(220.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(root),
            ))
            .id();

        commands.spawn((
            Text::new(text),
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(HINT_COLOR.into()),
            TextLayout::justify(Justify::Center),
            ChildOf(hint),
        ));
    }
}

// The Floating Stick's Two Circles, Spawned Hidden. sync_touch_stick_visual
// Places Them Under the Owning Finger Each Frame; They Are Circles by Way of a
// Corner Radius Larger Than Any Possible Node, Not an Image Asset
fn spawn_stick_visual(commands: &mut Commands, root: Entity, layout: &TouchLayout) {
    let base_size = layout.stick_radius * 2.0;
    let knob_size = layout.stick_radius * 0.9;

    commands.spawn((
        TouchStickBase,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(base_size),
            height: Val::Px(base_size),
            border: UiRect::all(Val::Px(BUTTON_BORDER_PX)),
            // A Radius Larger Than Any Side Rounds the Square Into a Circle. In
            // Bevy 0.19 This Lives on Node, Not as a Standalone Component
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(STICK_BASE_FILL.into()),
        BorderColor::all(Color::from(STICK_BASE_BORDER)),
        Visibility::Hidden,
        ChildOf(root),
    ));

    commands.spawn((
        TouchStickKnob,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(knob_size),
            height: Val::Px(knob_size),
            // Circle by Way of a Radius Past Half the Side; a Node Field in 0.19
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(STICK_KNOB_FILL.into()),
        Visibility::Hidden,
        ChildOf(root),
    ));
}

// Show the Overlay to the Player Actually Using the Glass and to No One Else.
// Existence Follows the Mode Above; Visibility Follows the Device, so a Desktop
// Never Shows Phone Buttons and a Tablet With a Gamepad Plugged In Hides Them
// the Moment the Pad Becomes the Driver
pub(super) fn sync_touch_overlay_visibility(
    device: Res<ActiveInputDevice>,
    mut q_roots: Query<&mut Visibility, With<TouchOverlayRoot>>,
) {
    let want = if *device == ActiveInputDevice::Touch {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut visibility in q_roots.iter_mut() {
        if *visibility != want {
            *visibility = want;
        }
    }
}

// Light Up Whatever a Finger Is Actually Holding. Fire Trusts the Assignment
// Rather Than the Rectangle Because the Firing Finger Is Allowed to Slide Off
// the Button and Keep Shooting - the Highlight Must Keep That Same Promise or
// the Overlay Would Claim the Trigger Was Released While the Gun Kept Firing
pub(super) fn sync_touch_button_feedback(
    touches: Res<Touches>,
    assign: Res<TouchAssignments>,
    layout: Res<TouchLayout>,
    mut q_buttons: Query<(&mut TouchControlButton, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (mut button, mut fill, mut border) in q_buttons.iter_mut() {
        let rect = button.kind.rect(&layout);

        let held = match button.kind {
            TouchControlKind::Fire => {
                assign.fire.is_some()
                    || touches.iter().any(|t| rect.contains(t.position()))
            }
            _ => touches.iter().any(|t| rect.contains(t.position())),
        };

        // Colour Writes Only on the Edge so the Renderer Is Not Dirtied Every
        // Frame by a Dozen Idle Buttons
        if held != button.pressed {
            button.pressed = held;
            if held {
                *fill = BackgroundColor(BUTTON_FILL_PRESSED.into());
                *border = BorderColor::all(Color::from(BUTTON_BORDER_PRESSED));
            } else {
                *fill = BackgroundColor(BUTTON_FILL.into());
                *border = BorderColor::all(Color::from(BUTTON_BORDER));
            }
        }
    }
}

// Place the Floating Stick Under Its Owning Finger. The Origin Comes From the
// Assignment (Which Trails When the Finger Overruns the Radius, Exactly as the
// Movement Math Does) and the Knob Is the Finger's Offset Clamped to the Ring,
// so What the Player Sees Is Precisely the Deflection the Player Character Gets
pub(super) fn sync_touch_stick_visual(
    mode: Res<TouchUiMode>,
    touches: Res<Touches>,
    assign: Res<TouchAssignments>,
    layout: Res<TouchLayout>,
    mut q_base: Query<
        (&mut Node, &mut Visibility),
        (With<TouchStickBase>, Without<TouchStickKnob>),
    >,
    mut q_knob: Query<
        (&mut Node, &mut Visibility),
        (With<TouchStickKnob>, Without<TouchStickBase>),
    >,
) {
    // The Stick Only Exists While Playing and While a Finger Owns It
    let live = if *mode == TouchUiMode::Gameplay {
        assign
            .stick
            .and_then(|(id, origin)| touches.get_pressed(id).map(|t| (origin, t.position())))
    } else {
        None
    };

    let Some((origin, finger)) = live else {
        for (_, mut visibility) in q_base.iter_mut() {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        for (_, mut visibility) in q_knob.iter_mut() {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };

    let radius = layout.stick_radius;
    let offset = finger - origin;
    let knob_centre = if offset.length() > radius {
        origin + offset.normalize_or_zero() * radius
    } else {
        origin + offset
    };

    for (mut node, mut visibility) in q_base.iter_mut() {
        node.left = Val::Px(origin.x - radius);
        node.top = Val::Px(origin.y - radius);
        // Inherited Rather Than Visible so the Root's Device Gate Still Wins;
        // a Forced Visible Child Would Punch Through a Hidden Parent
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }

    let knob_half = radius * 0.45;
    for (mut node, mut visibility) in q_knob.iter_mut() {
        node.left = Val::Px(knob_centre.x - knob_half);
        node.top = Val::Px(knob_centre.y - knob_half);
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shorthand: Everything Off, Playing Normally
    fn gameplay(step: SplashStep) -> TouchUiMode {
        derive_touch_ui_mode(step, true, false, false, false, false, false)
    }

    #[test]
    fn menus_and_name_entry_carry_the_menu_cluster() {
        for step in [
            SplashStep::Menu,
            SplashStep::PauseMenu,
            SplashStep::EpisodeSelect,
            SplashStep::SkillSelect,
            SplashStep::LoadSelect,
            SplashStep::SaveSelect,
            SplashStep::NameEntry,
            SplashStep::ChangeView,
            SplashStep::SoundOptions,
            SplashStep::ControlOptions,
            SplashStep::GameplayOptions,
            SplashStep::KeyBindings,
        ] {
            assert_eq!(gameplay(step), TouchUiMode::Menu, "step {step:?}");
        }
    }

    #[test]
    fn any_input_screens_advance_on_a_tap() {
        for step in [
            SplashStep::Splash0,
            SplashStep::Splash1,
            SplashStep::Scores,
            SplashStep::EpisodeVictory,
            SplashStep::EpisodeEndText0,
            SplashStep::EpisodeEndText1,
        ] {
            assert_eq!(gameplay(step), TouchUiMode::Advance, "step {step:?}");
        }
    }

    #[test]
    fn live_play_gets_the_gameplay_controls() {
        assert_eq!(gameplay(SplashStep::Done), TouchUiMode::Gameplay);
    }

    #[test]
    fn the_done_ladder_ranks_screens_over_states() {
        // Game Over Wants a Tap Even Though the Fizzle Backdrop Is Still Active
        // Underneath It and the Lock Is Held - the Order of the Ladder Is the Test
        let game_over =
            derive_touch_ui_mode(SplashStep::Done, true, false, true, false, true, true);
        assert_eq!(game_over, TouchUiMode::Advance);

        // The Intermission Tally Advances by Tap While Locked
        let tally =
            derive_touch_ui_mode(SplashStep::Done, true, false, false, true, false, true);
        assert_eq!(tally, TouchUiMode::Advance);

        // The Cheat Modal Freezes Play and Dismisses on Any Input
        let cheat =
            derive_touch_ui_mode(SplashStep::Done, true, true, false, false, false, true);
        assert_eq!(cheat, TouchUiMode::Advance);

        // The Fizzle Itself Takes No Input
        let dying =
            derive_touch_ui_mode(SplashStep::Done, true, false, false, false, true, true);
        assert_eq!(dying, TouchUiMode::Off);

        // Any Other Lock Is a Loading State: Dark Glass
        let loading =
            derive_touch_ui_mode(SplashStep::Done, true, false, false, false, false, true);
        assert_eq!(loading, TouchUiMode::Off);
    }

    #[test]
    fn disabling_touch_darkens_the_glass_everywhere() {
        for step in [SplashStep::Menu, SplashStep::Splash0, SplashStep::Done] {
            let mode = derive_touch_ui_mode(step, false, false, false, false, false, false);
            assert_eq!(mode, TouchUiMode::Off, "step {step:?}");
        }
    }
}
