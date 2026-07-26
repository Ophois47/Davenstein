/*
Davenstein - by David Petnick

Touch Control Layout - the Single Source of Truth for On-Screen Touch Geometry

Every Touch Rectangle in the Game Is Computed Here, Once, From the Primary
Window's Logical Size. The Touch Source Hit-Tests Against These Rectangles and
the On-Screen Overlay Positions Its Nodes From the Same Rectangles, so What the
Player Sees and What the Game Tests Can Never Drift Apart. Moving a Button Is a
One-File Edit and Both Halves Follow

Coordinate Space
All Rectangles Are Logical Window Pixels With the Origin at the Top-Left and Y
Increasing Downward. That Is Exactly What 'Touch::position()' Reports and Exactly
What Bevy UI Absolute 'Node' Positioning Uses, so No Conversion Is Needed at
Either End

Deliberately NOT the World Canvas
Touch Targets Are Laid Out Against the Window, Not the 320x240 Low-Resolution
Canvas. A Finger Is a Fixed Physical Size, so a Button Must Stay Finger-Sized in
Real Pixels No Matter What 'render_scale' Is Set To. This Is Also Why the Overlay
Belongs on the Persistent Window-Space MenuUiCamera Rather Than the Canvas Camera
*/

use bevy::math::Rect;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::options::ControlSettings;

// Button Edge Lengths as a Fraction of the Window's SHORTER Side
// Fractions of the Shorter Side Rather Than the Width so a Landscape Phone and a
// Tablet Both Size Their Buttons Against the Dimension the Thumb Has to Cover
const FIRE_SIZE_FRAC: f32 = 0.20;
const ACTION_SIZE_FRAC: f32 = 0.13;
const WEAPON_SIZE_FRAC: f32 = 0.10;

// Gap Between Neighbouring Buttons, and Inset From the Window Edge
const GAP_FRAC: f32 = 0.02;
const MARGIN_FRAC: f32 = 0.03;

// Virtual Stick Travel Radius as a Fraction of the Shorter Side
// This Is the Drag Distance That Means Full Speed, Not the Size of a Drawn Circle
const STICK_RADIUS_FRAC: f32 = 0.15;

// Fraction of the Window Width Reserved for the Movement Stick
const STICK_REGION_FRAC: f32 = 0.5;

// Floor for Every Computed Touch Target, in Logical Pixels
// Roughly 7 mm on a Typical Phone, the Usual Accessibility Minimum. Without It a
// Small Window (a Desktop Browser Tab Dragged Narrow) Produces Untappable Boxes
const MIN_TOUCH_PX: f32 = 44.0;

// Bounds Applied to ControlSettings.touch_ui_scale Before Anything Is Sized
// Clamped Here Rather Than Trusting the Settings File, Which the Player Can Edit
const UI_SCALE_MIN: f32 = 0.5;
const UI_SCALE_MAX: f32 = 2.0;

// Screen-Space Geometry of Every Touch Control for the Current Window Size
// Recomputed Only When the Window Size or the Player's UI Scale Actually Changes,
// so the Resource's Change Detection Stays Meaningful for the Overlay to Watch
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct TouchLayout {
    // Logical Window Size This Layout Was Built For
    // Also the Staleness Key: a Different Size Means Rebuild
    pub window_size: Vec2,

    // Clamped Copy of ControlSettings.touch_ui_scale Used for These Sizes
    pub ui_scale: f32,

    // Region Where a New Touch Claims the Movement Stick
    // Never Drawn: the Stick Floats to Wherever the Thumb Actually Lands
    pub stick_region: Rect,

    // Drag Distance From the Stick Origin That Means Full Speed
    pub stick_radius: f32,

    // Held Fire Button, the Largest Target and Nearest the Right Thumb
    pub fire: Rect,

    // Use / Open Door, One Edge per Tap
    pub use_door: Rect,

    // Weapon Slots 1 Through 4 in Ascending Order (Knife, Pistol, MG, Chaingun)
    pub weapons: [Rect; 4],

    // Opens the Pause Menu, Kept Away From Both Thumbs' Resting Arcs
    pub pause: Rect,
}

impl Default for TouchLayout {
    fn default() -> Self {
        // Rect::EMPTY Has an Inverted Min and Max, so contains() Is False for Every
        // Point. A Layout That Has Not Seen a Window Yet Therefore Swallows Nothing
        // Instead of Claiming Touches at the Origin
        Self {
            window_size: Vec2::ZERO,
            ui_scale: 1.0,
            stick_region: Rect::EMPTY,
            stick_radius: 0.0,
            fire: Rect::EMPTY,
            use_door: Rect::EMPTY,
            weapons: [Rect::EMPTY; 4],
            pause: Rect::EMPTY,
        }
    }
}

impl TouchLayout {
    // True Once a Real Window Size Has Been Measured
    // The Touch Source Checks This Before Dividing by stick_radius
    pub fn is_ready(&self) -> bool {
        self.window_size.x > 0.0 && self.window_size.y > 0.0
    }

    // Weapon Slot 1..=4 Whose Button Contains point, if Any
    // Returns the Device-Neutral Slot Index PlayerIntent Expects, Not an Array Index
    pub fn weapon_slot_at(&self, point: Vec2) -> Option<u8> {
        self.weapons
            .iter()
            .position(|slot| slot.contains(point))
            .map(|index| index as u8 + 1)
    }

    // Build Every Rectangle for a Given Window Size and Player UI Scale
    // Pure Arithmetic With No World Access so It Can Be Unit Tested Directly
    pub fn compute(window_size: Vec2, ui_scale: f32) -> Self {
        let scale = ui_scale.clamp(UI_SCALE_MIN, UI_SCALE_MAX);

        // max(1.0) Guards the Degenerate Zero-Size Window a Minimized or
        // Freshly-Created Window Can Report for a Frame
        let short = window_size.min_element().max(1.0);

        // Every Size Derives From the Shorter Side, Scales by Player Preference,
        // Then Takes the Accessibility Floor
        let sized = |frac: f32| (short * frac * scale).max(MIN_TOUCH_PX);

        let fire_size = sized(FIRE_SIZE_FRAC);
        let action_size = sized(ACTION_SIZE_FRAC);
        let weapon_size = sized(WEAPON_SIZE_FRAC);
        let gap = short * GAP_FRAC;
        let margin = short * MARGIN_FRAC;

        let right = window_size.x - margin;
        let bottom = window_size.y - margin;

        // Fire Sits in the Bottom-Right Thumb Arc and Is the Largest Target Because
        // It Is Held Rather Than Tapped and Must Not Be Missed Under Pressure
        let fire = square_from_bottom_right(right, bottom, fire_size);

        // Use / Open Door Stacks Directly Above Fire. Same Column so the Thumb
        // Travels Straight Up Rather Than Diagonally Across the Look Area
        let use_door = square_from_bottom_right(right, fire.min.y - gap, action_size);

        // Weapon Slots Run Left to Right Along the Bottom Edge, Ending Just Left of
        // Fire, so Slot 4 (Chaingun) Is the Shortest Reach From the Firing Thumb
        //
        // CAVEAT: On a Very Narrow Window This Row Can Extend Left Past
        // STICK_REGION_FRAC and, Because Buttons Hit-Test First, Eat Part of the
        // Stick Area. Harmless on Any Real Phone or Tablet Aspect; Revisit When the
        // Overlay Lands and the Encroachment Becomes Visible
        let weapons_right = fire.min.x - gap;
        let weapons_left = weapons_right - (4.0 * weapon_size + 3.0 * gap);
        let mut weapons = [Rect::EMPTY; 4];
        for (index, slot) in weapons.iter_mut().enumerate() {
            let left = weapons_left + index as f32 * (weapon_size + gap);
            *slot = Rect::new(left, bottom - weapon_size, left + weapon_size, bottom);
        }

        // Pause Sits Top-Right, Clear of Both Thumbs so It Is Never Hit Mid-Firefight
        let pause = square_from_bottom_right(right, margin + action_size, action_size);

        // The Stick Owns the Left Portion of the Window, Full Height. A Touch That
        // Lands Here and Finds the Stick Free Becomes the Stick; Everything the
        // Buttons and the Stick Do Not Claim Aims Instead
        let stick_region = Rect::new(
            0.0,
            0.0,
            window_size.x * STICK_REGION_FRAC,
            window_size.y,
        );

        Self {
            window_size,
            ui_scale: scale,
            stick_region,
            stick_radius: (short * STICK_RADIUS_FRAC * scale).max(MIN_TOUCH_PX),
            fire,
            use_door,
            weapons,
            pause,
        }
    }
}

// Square Rect Anchored by Its Bottom-Right Corner, Growing Up and to the Left
// Every Control Here Is Anchored to a Window Corner, and Anchoring by the Corner
// Nearest That Window Edge Keeps the Margin Exact Under Any Size Change
fn square_from_bottom_right(right: f32, bottom: f32, size: f32) -> Rect {
    Rect::new(right - size, bottom - size, right, bottom)
}

// Rebuild TouchLayout Whenever the Window Size or the Player's UI Scale Changes
// Registered Before the InputGather Set so the Touch Source Always Hit-Tests
// Against Geometry That Matches This Frame's Window
//
// The Early Return Is Not a Micro-Optimization: Writing Through ResMut Marks the
// Resource Changed, and the Overlay Will Respawn Its Nodes on That Signal. Without
// the Guard It Would Rebuild Its UI Tree Every Single Frame
pub fn update_touch_layout(
    controls: Res<ControlSettings>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut layout: ResMut<TouchLayout>,
) {
    let Some(window) = q_window.iter().next() else {
        return;
    };

    let window_size = Vec2::new(window.width(), window.height());
    let ui_scale = controls.touch_ui_scale.clamp(UI_SCALE_MIN, UI_SCALE_MAX);

    if layout.window_size == window_size && layout.ui_scale == ui_scale {
        return;
    }

    *layout = TouchLayout::compute(window_size, ui_scale);
}
