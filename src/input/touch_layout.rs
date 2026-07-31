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

Two Control Sets Share One Layout
Gameplay Controls (Stick, Fire, Use, Weapons, Pause) and Menu Controls (the
Direction Cluster, Confirm, Back) Are Both Computed Here, Even Though Only One
Set Is Ever Live at a Time (See input::TouchUiMode). Confirm Reuses Fire's Exact
Rectangle and Back Reuses Use's, on Purpose: the Right Thumb Learns ONE Resting
Arc and the Most Important Action in Both States Lives Under It
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

// Menu Direction Cluster Cell Edge Length as a Fraction of the Shorter Side
// Matches ACTION_SIZE_FRAC Today but Is Named Separately Because the Cluster Is
// Tapped Repeatedly While Scrolling Long Menus and May Want Its Own Tuning
const MENU_NAV_SIZE_FRAC: f32 = 0.13;

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

    // Menu Direction Cluster, Bottom-Left in a Cross: Up Over Left/Right Over Down
    // Live Only While input::TouchUiMode Is Menu; See the Touch Source
    pub menu_up: Rect,
    pub menu_down: Rect,
    pub menu_left: Rect,
    pub menu_right: Rect,

    // Activates the Highlighted Menu Item. Deliberately the Same Rectangle as Fire
    // so the Right Thumb Confirms From Its Gameplay Resting Position
    pub menu_confirm: Rect,

    // Backs Out One Menu Level. Same Rectangle as Use, Directly Above Confirm
    pub menu_back: Rect,
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
            menu_up: Rect::EMPTY,
            menu_down: Rect::EMPTY,
            menu_left: Rect::EMPTY,
            menu_right: Rect::EMPTY,
            menu_confirm: Rect::EMPTY,
            menu_back: Rect::EMPTY,
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
        //
        // Capped at the Weapon Row: Buttons Hit-Test First, so on Narrower Aspects
        // (a 4:3 Tablet) an Uncapped Half-Width Region Would Silently Lose Its
        // Rightmost Sliver to Weapon Slot 1 and the Overlay Would Draw a Button on
        // Top of Ground the Player Was Told Belongs to Movement. The Floor Keeps a
        // Usable Region on Degenerate Desktop Windows Where MIN_TOUCH_PX Inflation
        // Pushes the Row Far Left; There the Row Wins the Overlap, as Documented
        let stick_right = (window_size.x * STICK_REGION_FRAC)
            .min(weapons_left - gap)
            .max(window_size.x * 0.25);
        let stick_region = Rect::new(0.0, 0.0, stick_right, window_size.y);

        // Menu Direction Cluster, Bottom-Left, in a Cross so Each Direction Sits
        // Where a D-Pad Would Put It. Bottom-Left Mirrors the Movement Stick's Home
        // so Menu Navigation and Movement Live Under the Same Thumb
        let nav_size = sized(MENU_NAV_SIZE_FRAC);
        let col0 = margin;
        let col1 = col0 + nav_size + gap;
        let col2 = col1 + nav_size + gap;
        let row_down_top = window_size.y - margin - nav_size;
        let row_mid_top = row_down_top - gap - nav_size;
        let row_up_top = row_mid_top - gap - nav_size;

        let menu_down = Rect::new(col1, row_down_top, col1 + nav_size, row_down_top + nav_size);
        let menu_left = Rect::new(col0, row_mid_top, col0 + nav_size, row_mid_top + nav_size);
        let menu_right = Rect::new(col2, row_mid_top, col2 + nav_size, row_mid_top + nav_size);
        let menu_up = Rect::new(col1, row_up_top, col1 + nav_size, row_up_top + nav_size);

        // Confirm and Back Deliberately Reuse Fire's and Use's Exact Geometry (See
        // the Module Header). Computed Through the Same Helper Calls Rather Than
        // Copied From the Fields so a Future Retune of Either Pair Cannot Silently
        // Drag the Other Along
        let menu_confirm = square_from_bottom_right(right, bottom, fire_size);
        let menu_back = square_from_bottom_right(right, menu_confirm.min.y - gap, action_size);

        Self {
            window_size,
            ui_scale: scale,
            stick_region,
            stick_radius: (short * STICK_RADIUS_FRAC * scale).max(MIN_TOUCH_PX),
            fire,
            use_door,
            weapons,
            pause,
            menu_up,
            menu_down,
            menu_left,
            menu_right,
            menu_confirm,
            menu_back,
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

#[cfg(test)]
mod tests {
    use super::*;

    // Representative Logical Window Sizes: a Tall-Aspect Landscape Phone (Pixel 8
    // Pro Class), a 4:3 Tablet, a Desktop Window, and a Deliberately Tiny Window
    const PHONE: Vec2 = Vec2::new(998.0, 448.0);
    const TABLET: Vec2 = Vec2::new(1024.0, 768.0);
    const DESKTOP: Vec2 = Vec2::new(1280.0, 720.0);

    // Overlap Meaning "a Finger Could Land in Both", Not "the Floats Touched".
    // Neighbouring Buttons Legitimately Abut at the Maximum UI Scale, and the
    // Seam Coordinate Computed Two Ways Differs by ~1e-14 Pixels, so a Strict
    // Emptiness Test Would Flake on Pure Float Noise
    fn rects_overlap(a: Rect, b: Rect) -> bool {
        let overlap = a.intersect(b);
        !overlap.is_empty() && overlap.width() > 0.5 && overlap.height() > 0.5
    }

    // Every Rectangle a Finger Is Expected to Hit, Gameplay and Menu Alike
    fn all_buttons(l: &TouchLayout) -> Vec<Rect> {
        let mut out = vec![
            l.fire,
            l.use_door,
            l.pause,
            l.menu_up,
            l.menu_down,
            l.menu_left,
            l.menu_right,
            l.menu_confirm,
            l.menu_back,
        ];
        out.extend_from_slice(&l.weapons);
        out
    }

    #[test]
    fn every_target_meets_the_accessibility_floor() {
        // The Small Epsilon Absorbs Float Noise: a Cell Placed at
        // margin + 2 * (size + gap) Can Measure a Hair Under Its Exact Size
        const FLOOR: f32 = MIN_TOUCH_PX - 0.001;

        for size in [PHONE, TABLET, DESKTOP] {
            for scale in [UI_SCALE_MIN, 1.0, UI_SCALE_MAX] {
                let l = TouchLayout::compute(size, scale);
                for rect in all_buttons(&l) {
                    assert!(rect.width() >= FLOOR, "target too narrow at {size} x{scale}");
                    assert!(rect.height() >= FLOOR, "target too short at {size} x{scale}");
                }
                assert!(l.stick_radius >= FLOOR);
            }
        }
    }

    #[test]
    fn ui_scale_is_clamped_before_sizing() {
        let low = TouchLayout::compute(PHONE, 0.01);
        let high = TouchLayout::compute(PHONE, 50.0);
        assert_eq!(low.ui_scale, UI_SCALE_MIN);
        assert_eq!(high.ui_scale, UI_SCALE_MAX);

        // A Clamped Extreme Must Produce the Same Geometry as the Clamp Boundary,
        // Proving the Clamp Happens Before Anything Is Sized From the Value
        assert_eq!(low, TouchLayout::compute(PHONE, UI_SCALE_MIN));
        assert_eq!(high, TouchLayout::compute(PHONE, UI_SCALE_MAX));
    }

    #[test]
    fn weapon_slots_hit_test_to_their_one_based_index() {
        let l = TouchLayout::compute(PHONE, 1.0);
        for (index, slot) in l.weapons.iter().enumerate() {
            assert_eq!(l.weapon_slot_at(slot.center()), Some(index as u8 + 1));
        }

        // A Point Clearly Outside Every Slot Selects Nothing
        assert_eq!(l.weapon_slot_at(Vec2::new(1.0, 1.0)), None);
    }

    #[test]
    fn gameplay_buttons_never_overlap_each_other() {
        for size in [PHONE, TABLET, DESKTOP] {
            let l = TouchLayout::compute(size, 1.0);
            let buttons = [l.fire, l.use_door, l.pause, l.weapons[0], l.weapons[1], l.weapons[2], l.weapons[3]];
            for (i, a) in buttons.iter().enumerate() {
                for b in buttons.iter().skip(i + 1) {
                    assert!(!rects_overlap(*a, *b), "gameplay overlap at {size}");
                }
            }
        }
    }

    #[test]
    fn menu_controls_never_overlap_each_other() {
        for size in [PHONE, TABLET, DESKTOP] {
            let l = TouchLayout::compute(size, 1.0);
            let buttons = [l.menu_up, l.menu_down, l.menu_left, l.menu_right, l.menu_confirm, l.menu_back];
            for (i, a) in buttons.iter().enumerate() {
                for b in buttons.iter().skip(i + 1) {
                    assert!(!rects_overlap(*a, *b), "menu overlap at {size}");
                }
            }
        }
    }

    #[test]
    fn confirm_and_back_share_the_firing_thumb_arc() {
        // The Muscle-Memory Guarantee the Module Header Promises: Confirm Sits
        // Exactly on Fire and Back Sits Exactly on Use
        let l = TouchLayout::compute(PHONE, 1.0);
        assert_eq!(l.menu_confirm, l.fire);
        assert_eq!(l.menu_back, l.use_door);
    }

    #[test]
    fn stick_region_is_capped_at_the_weapon_row() {
        // A 4:3 Tablet Is Narrow Enough That an Uncapped Half-Width Region Would
        // Run Under Weapon Slot 1. The Cap Must Keep Them Disjoint There While
        // Leaving the Phone's Region at Its Full Half Width
        let tablet = TouchLayout::compute(TABLET, 1.0);
        for slot in tablet.weapons {
            assert!(!rects_overlap(tablet.stick_region, slot), "cap failed on tablet");
        }

        let phone = TouchLayout::compute(PHONE, 1.0);
        assert_eq!(phone.stick_region.max.x, PHONE.x * STICK_REGION_FRAC);
    }

    #[test]
    fn unmeasured_layout_swallows_nothing() {
        // Rect::EMPTY Must Reject Every Point so a Layout That Has Not Seen a
        // Window Cannot Claim Touches, Menu Rectangles Included
        let l = TouchLayout::default();
        for rect in all_buttons(&l) {
            assert!(!rect.contains(Vec2::ZERO));
            assert!(!rect.contains(Vec2::new(50.0, 50.0)));
        }
        assert!(!l.is_ready());
    }
}
