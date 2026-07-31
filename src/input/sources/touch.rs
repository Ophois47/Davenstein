/*
Davenstein - by David Petnick

The touch source reads Bevy's Touches resource and merges a contribution into the
shared PlayerIntent accumulator, exactly like the keyboard and gamepad sources. It
runs last so keyboard and gamepad keep move_wish and weapon_select priority

Touch Turns. It Never Pitches
The drag region writes yaw and nothing else, always, whatever mouselook_enabled
says. Three reasons, in ascending order of how much they matter:

1. Wolfenstein 3-D had no vertical look. Its renderer drew the world as vertical
   columns and had no way to express a pitched view, so pitch is a Davenstein
   addition rather than something being ported

2. Pitch is not cosmetic here. apply_look writes pitch onto the PLAYER transform
   and hud.rs fires along that transform's forward vector, so a pitched view aims
   the gun at the ceiling. Nothing in Wolf3D is above or below the player, so every
   degree of pitch is pure aiming error with no target it could ever help reach

3. A thumb pivots from a knuckle, so a "horizontal" drag on glass is really an arc
   and always carries vertical travel. Feeding that to pitch means the player drifts
   off the horizon while aiming and starts missing shots that look lined up, with no
   crosshair feedback to explain why. The mouse does not have this problem because a
   mouse slides on a flat surface

Vertical drag travel is therefore discarded outright rather than scaled down, and
the stored per-finger state is a single f32 X coordinate so no vertical component
can leak back in later by accident

Note this is independent of mouselook_enabled, which is a MOUSE setting. Turning
must never be unavailable to a touch player, for the same reason keyboard turning
is always live in keyboard_mouse.rs: the game has to stay fully playable on the
device in the player's hands

Roles Are Pinned to Touch IDs, Not to Screen Regions
A touch claims its job once, on the frame it lands, from where it landed. It then
keeps that job until the finger leaves the glass, however far it wanders. This is
the single most important property of the module. Classifying by current position
every frame looks equivalent and is not: lift the turning finger while strafing and
the surviving stick finger gets re-read as a turn drag, so the player lurches and
the view snaps. Pinning by ID makes a two-thumb hold behave like two independent
controllers, which is also what lets fire be held while the other thumb turns

Layout ownership belongs to TouchLayout, not here. This file asks "does this point
fall in that rectangle" and never decides where the rectangle is

Only Fire Needs Stored State
Held controls (the stick, the turn drag, fire) need an ID pinned across frames.
Edge controls (use, weapon select, pause) are read straight from
iter_just_pressed, which is inherently one frame wide, so a resting finger cannot
repeat them and there is nothing to store or release
*/

use bevy::input::touch::Touches;
use bevy::prelude::*;

use crate::input::intent::PlayerIntent;
use crate::input::menu::MenuNav;
use crate::input::touch_layout::TouchLayout;
use crate::options::ControlSettings;

// Base Sensitivity Applied on Top of ControlSettings.touch_turn_sensitivity
// Touch Deltas Arrive in Logical Pixels Just Like Mouse Deltas, so This Is the
// Same Kind of Constant as keyboard_mouse::BASE_SENSITIVITY and Is Set a Little
// Higher Because a Thumb Drag Covers Far Less Distance Than a Mouse Sweep
// Expect to Retune This Once It Has Been Felt on Real Hardware
const BASE_SENSITIVITY: f32 = 0.0035;

// Inner Deadzone for the Virtual Stick, as a Fraction of Its Travel Radius
// A Thumb Resting on Glass Jitters by a Pixel or Two Constantly. Without This the
// Player Drifts While Standing Still. Not Exposed as a Setting Because Unlike a
// Worn Physical Stick There Is No Per-Device Variation to Compensate For
const STICK_DEADZONE: f32 = 0.12;

// Fraction of Stick Travel Past Which Run Engages
// Folding Run Into the Stick's Outer Ring Instead of Giving It a Button Keeps the
// Left Thumb on One Control and Matches How Players Already Expect to Sprint
const RUN_RING: f32 = 0.8;

// Which Set of On-Screen Controls Is Live Right Now
//
// Written Once per Frame by the UI Layer (ui::touch_overlay), Which Is the Only
// Place That Knows Whether the Player Is Looking at Gameplay, a Menu, or an
// Any-Input Screen. Read Here to Decide Which Rectangles Touches Are Tested
// Against, and Read by the Overlay to Decide Which Rectangles to Draw - the Same
// Value Gates Both Halves so a Control Can Never Be Tappable While Invisible or
// Visible While Dead
//
// Derived From the PREVIOUS Frame's UI State Because the Sync System Runs Before
// InputGather While the Menu Machine Runs After It. One Frame of Lag Is Safe
// Here for the Same Reason Role Pinning Is: Only Just-Pressed Touches Claim
// Anything, so a Finger Held Across a Mode Change Cannot Acquire a New Job
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TouchUiMode {
    // Live Gameplay: Stick, Turn Drag, Fire, Use, Weapons, Pause
    Gameplay,

    // A Navigable Menu: the Direction Cluster, Confirm, and Back
    Menu,

    // A Screen Any Deliberate Input Advances (Splash, Scores, Victory, Game
    // Over, the Intermission Tally): Every Tap Becomes Confirm. Safe Precisely
    // Because These Screens Have No Highlighted Item a Stray Thumb Could
    // Activate - Menus Never Get This Treatment, Per the contribute_menu Note.
    // The Default Because the Application Opens on the First Splash Screen
    #[default]
    Advance,

    // No Touch UI at All (Death Fizzle, the Get-Psyched Loader, Touch Disabled)
    Off,
}

// Which Touch ID Currently Owns Each Held Control, and the State That Control Needs
// Cleared Automatically as Fingers Leave, Including Touches the OS Cancels
#[derive(Resource, Debug, Default)]
pub struct TouchAssignments {
    // Stick Finger and Its Current Origin
    // The Origin Starts Where the Finger Landed, so the Stick Floats to the Thumb
    // Rather Than Forcing the Thumb to a Fixed Spot, and It Trails the Finger Once
    // Travel Is Exceeded (See the Contribution Pass)
    pub stick: Option<(u64, Vec2)>,

    // Turning Finger and the Last X Coordinate This Module Observed for It
    // Stored Rather Than Read Back From Bevy on Purpose; See the Long Note in the
    // Contribution Pass. Deliberately an f32 and Not a Vec2: Touch Turning Is
    // Yaw-Only, and a Type That Cannot Hold a Y Coordinate Cannot Later Grow One
    // by Accident
    pub turn: Option<(u64, f32)>,

    // Finger Holding the Fire Button
    pub fire: Option<u64>,
}

impl TouchAssignments {
    // True When No Finger Owns Any Held Control
    // Used by gather to Avoid Writing Through ResMut (and Needlessly Tripping
    // Change Detection) on Every Frame That Touch Is Disabled
    pub fn is_idle(&self) -> bool {
        self.stick.is_none() && self.turn.is_none() && self.fire.is_none()
    }

    // Drop Every Assignment
    // Called When Touch Is Switched Off Mid-Hold so a Held Fire or a Pushed Stick
    // Cannot Survive as Phantom Input the Player Can No Longer Release
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

// Merge Every Active Touch into the Shared PlayerIntent Accumulator
// Runs After Keyboard and Gamepad, so move_wish Is Filled Only When Still Zero and
// weapon_select Only When No Earlier Source Claimed It, Per the gather Merge Contract
//
// Returns Whether Touch Supplied Any DELIBERATE Input This Frame, for the
// ActiveInputDevice Arbitration in gather. A Finger Merely RESTING on the Glass Returns
// False, Which Is the Whole Point: a Palm on a Touchscreen Laptop Must Not Be Mistaken
// for Somebody Choosing to Play by Touch. Motion, a Held Button, or an Edge Is Required
pub fn contribute(
    acc: &mut PlayerIntent,
    touches: &Touches,
    assign: &mut TouchAssignments,
    layout: &TouchLayout,
    controls: &ControlSettings,
    mode: TouchUiMode,
) -> bool {
    // No Measured Window Yet Means No Meaningful Geometry to Test Against
    if !layout.is_ready() {
        return false;
    }

    let mut driven = false;

    // PASS 1 - RELEASE
    // A Touch Is Gone When It Is No Longer Pressed. Bevy Removes Both Ended and
    // OS-Canceled Touches From the Pressed Set, so This One Check Covers the Finger
    // Lifting Normally and the System Snatching It Away for a Notification Swipe or
    // an Incoming Call. Releasing First Matters: the Same ID Can Be Recycled by the
    // Platform for a New Finger in the Same Frame, and a Stale Assignment Would
    // Hand That New Finger the Old Job
    if let Some((id, _)) = assign.stick {
        if touches.get_pressed(id).is_none() {
            assign.stick = None;
        }
    }
    if let Some((id, _)) = assign.turn {
        if touches.get_pressed(id).is_none() {
            assign.turn = None;
        }
    }
    if let Some(id) = assign.fire {
        if touches.get_pressed(id).is_none() {
            assign.fire = None;
        }
    }

    // Outside Gameplay the Screen Belongs to the Menu Controls: Nothing Claims
    // and Nothing Contributes. Held Assignments Are Deliberately Kept (Not
    // Cleared) so a Thumb Resting on the Stick Keeps Its Job Across a Quick
    // Pause and Movement Resumes the Instant the Menu Closes - the Same Promise
    // gather Makes for Held Keys, and Fire Stays Safe Under the Same Latch
    //
    // The One Piece of State That Must Stay Current Is the Turn Finger's Stored
    // X: It Is a Delta Baseline, and a Baseline Frozen at Menu-Open Would Turn
    // Everything the Finger Drifted While the Menu Was Up Into One Violent View
    // Snap on the First Live Frame. Re-Anchoring Every Paused Frame Makes the
    // Return Delta Start From Wherever the Finger Actually Is
    if mode != TouchUiMode::Gameplay {
        if let Some((id, _)) = assign.turn {
            if let Some(touch) = touches.get_pressed(id) {
                assign.turn = Some((id, touch.position().x));
            }
        }
        return false;
    }

    // PASS 2 - CLAIM
    // Only Touches That Landed This Frame Are Considered, Which Is What Pins a Role
    // for the Life of the Finger. Buttons Are Tested Before Regions Because They
    // Draw on Top: Where a Button Overlaps the Turning Area, the Player Sees the
    // Button and Expects the Button
    //
    // Simultaneous Landings Iterate in Hash Order. That Only Matters if Two Fingers
    // Land Inside the Stick Region on the Very Same Frame, in Which Case Either One
    // Is a Defensible Stick and the Other Turns
    for touch in touches.iter_just_pressed() {
        let id = touch.id();
        let point = touch.position();

        // Pause Is Menu Navigation, Not Gameplay Intent, so It Is Consumed by
        // contribute_menu. Swallowed Here Only So It Cannot Also Start a Turn Drag
        if layout.pause.contains(point) {
            continue;
        }

        // Fire Is Held: Pin the ID for the Hold and Emit the Press Edge Now
        if layout.fire.contains(point) {
            assign.fire = Some(id);
            acc.fire_pressed = true;
            driven = true;
            continue;
        }

        // Use / Open Door Is a Pure Edge, One Door per Tap
        if layout.use_door.contains(point) {
            acc.use_pressed = true;
            driven = true;
            continue;
        }

        // Weapon Select Keeps the First Source That Set It, Matching Keyboard Priority
        if let Some(slot) = layout.weapon_slot_at(point) {
            acc.weapon_select = acc.weapon_select.or(Some(slot));
            driven = true;
            continue;
        }

        // Movement Claims the Left Region, but Only if the Stick Is Free. The Landing
        // Point Becomes the Stick Origin
        if assign.stick.is_none() && layout.stick_region.contains(point) {
            assign.stick = Some((id, point));
            continue;
        }

        // Everything Left Over Turns. Only the Landing X Is Kept, Because Only
        // Horizontal Travel Will Ever Be Read. Falling Through Rather Than Demanding
        // the Right Half Means a Second Finger on the LEFT Side Still Turns Once the
        // Stick Is Taken, Which Is How Left-Handed Players Actually Hold a Phone
        if assign.turn.is_none() {
            assign.turn = Some((id, point.x));
        }
    }

    // PASS 3 - CONTRIBUTE
    // Virtual Stick to move_wish
    if let Some((id, origin)) = assign.stick {
        if let Some(touch) = touches.get_pressed(id) {
            let position = touch.position();
            let mut offset = position - origin;
            let distance = offset.length();

            // Drag the Origin Along Behind a Finger That Has Run Past Full Travel.
            // With a Fixed Origin, a Thumb That Slid Too Far Has to Cover That Dead
            // Slack Again Before the Player Slows Down, Which Feels Like Lag on the
            // Only Control That Has to Respond Instantly
            if distance > layout.stick_radius {
                let direction = offset / distance;
                assign.stick = Some((id, position - direction * layout.stick_radius));
                offset = direction * layout.stick_radius;
            }

            // 4-Way D-Pad Snap
            //
            // The Left Thumb Now Reads Like a D-Pad Rather Than an Analog Stick:
            // Past a Small Centre Deadzone the Dominant Axis Wins and Movement Is
            // Emitted at Full Travel on That ONE Cardinal. No Diagonals, No
            // Half-Speed Drift - Press a Direction, Get Exactly That Direction,
            // Which Suits Wolf3D's Blocky Corridors Far Better Than a Mushy Lean.
            // The Floating Origin Is Kept (Touch Down Sets Centre, Slide to Pick a
            // Direction), so the Thumb Never Has to Find a Fixed Pad First
            //
            // (8-Way, With Diagonals, Is a Planned Follow-Up)
            let raw = offset / layout.stick_radius;
            let wish = if raw.length() < STICK_DEADZONE {
                Vec2::ZERO
            } else if raw.x.abs() >= raw.y.abs() {
                // Horizontal Wins: Strafe. X Already Matches move_wish's Strafe Axis
                Vec2::new(raw.x.signum(), 0.0)
            } else {
                // Vertical Wins: Forward or Back. Screen Y Grows Downward, so an
                // Upward Slide (Negative Y) Becomes Positive Forward
                Vec2::new(0.0, -raw.y.signum())
            };

            // A Cardinal Was Chosen, so the Thumb Is Genuinely Driving Movement
            if wish != Vec2::ZERO {
                driven = true;
            }

            // Keyboard and Gamepad Priority: Fill move_wish Only if Still Untouched
            if acc.move_wish == Vec2::ZERO && wish != Vec2::ZERO {
                acc.move_wish = wish;
            }

            // Push Toward the Outer Ring to Run, the Same Threshold the Analog
            // Stick Used, Measured on Raw Travel Since There Is No Longer a Rescale
            acc.run |= raw.length() >= RUN_RING;
        }
    }

    // Turn Drag to Yaw. Horizontal Travel Only; See the Module Header for Why There
    // Is No Pitch Term Here and Why There Should Never Be One
    if let Some((id, last_x)) = assign.turn {
        if let Some(touch) = touches.get_pressed(id) {
            let x = touch.position().x;

            // Deliberately NOT Touch::delta()
            // Bevy Only Refreshes previous_position on Frames Where at Least One
            // Touch Event Arrived (Upstream Issue 12442). A Finger Held Perfectly
            // Still Sends No Move Events, so previous_position Stops Advancing and
            // delta() Keeps Returning the Last Real Movement Forever: the View Spins
            // On Its Own While the Thumb Sits Motionless. Differencing Against a
            // Coordinate This Module Stored Itself Is Correct on Every Frame, and
            // Costs One f32. Do Not "Simplify" This Back to delta()
            let delta_x = x - last_x;
            assign.turn = Some((id, x));

            if delta_x != 0.0 {
                driven = true;

                // Sign Matches the Mouse, Gamepad, and Keyboard Turn Paths: Dragging
                // Right Turns Right. invert_y Is Not Consulted Because There Is No
                // Vertical Axis to Invert
                acc.look_delta.x -= controls.scaled_touch_turn(delta_x) * BASE_SENSITIVITY;
            }
        }
    }

    // Held Fire. The Press Edge Was Emitted in the Claim Pass; This Is the Hold, and
    // It Survives the Finger Sliding Off the Button Because the Role Is Pinned by ID
    acc.fire |= assign.fire.is_some();

    // A Held Fire Button Counts as Driving Even on Frames With No Movement, Because
    // Standing Still Leaning on the Trigger Is a Deliberate Act
    driven || assign.fire.is_some()
}

// Merge Touch Menu Navigation Into the Shared MenuNav Accumulator, Returning
// Whether Any Tap Actually Landed on a Control so the Caller Can Count Touch as
// the Driving Device and Reveal the Overlay
//
// What a Tap Means Depends Entirely on the Mode:
//   Gameplay - the Pause Button Is the Only Menu Control on the Glass
//   Menu     - the Direction Cluster Plus Confirm and Back, Nothing Else
//   Advance  - Any Tap Anywhere Is Confirm, Because These Screens Have No
//              Highlighted Item and Exist Only to Be Advanced
//   Off      - the Glass Is Dead
//
// There Is Still Intentionally No Tap-Anywhere-Confirms in MENU Mode. It Would Be
// One Line and It Would Be a Bug: MenuNav.confirm Activates Whatever Item Is
// Highlighted, so a Stray Thumb Anywhere on the Glass Could Pick "Quit" Out of
// the Pause Menu. Confirm in a Menu Requires the One Visible, Labeled Button
pub fn contribute_menu(
    nav: &mut MenuNav,
    touches: &Touches,
    layout: &TouchLayout,
    mode: TouchUiMode,
) -> bool {
    if !layout.is_ready() {
        return false;
    }

    let mut driven = false;
    for touch in touches.iter_just_pressed() {
        driven |= apply_menu_touch(nav, layout, mode, touch.position());
    }

    driven
}

// Map One Just-Pressed Touch Point Onto the Menu Accumulator for the Given Mode
// Pure Geometry Against the Layout, Split Out From contribute_menu so the
// Mapping Can Be Unit Tested Without Constructing a Live Touches Resource
fn apply_menu_touch(
    nav: &mut MenuNav,
    layout: &TouchLayout,
    mode: TouchUiMode,
    point: Vec2,
) -> bool {
    match mode {
        TouchUiMode::Gameplay => {
            let hit = layout.pause.contains(point);
            nav.pause |= hit;
            hit
        }

        TouchUiMode::Menu => {
            // First Hit Wins. The Layout Tests Guarantee These Rectangles Are
            // Pairwise Disjoint at Normal Scale, so the Order Only Matters on
            // Saturated Extreme-Scale Windows Where Neighbours Can Abut
            let hit_up = layout.menu_up.contains(point);
            let hit_down = !hit_up && layout.menu_down.contains(point);
            let hit_left = !hit_up && !hit_down && layout.menu_left.contains(point);
            let hit_right =
                !hit_up && !hit_down && !hit_left && layout.menu_right.contains(point);
            let hit_confirm = layout.menu_confirm.contains(point);
            let hit_back = !hit_confirm && layout.menu_back.contains(point);

            nav.up |= hit_up;
            nav.down |= hit_down;
            nav.left |= hit_left;
            nav.right |= hit_right;
            nav.confirm |= hit_confirm;
            nav.cancel |= hit_back;

            hit_up || hit_down || hit_left || hit_right || hit_confirm || hit_back
        }

        TouchUiMode::Advance => {
            nav.confirm = true;
            true
        }

        TouchUiMode::Off => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A Real Computed Layout Rather Than Hand-Built Rectangles, so These Tests
    // Also Break if the Geometry and the Mapping Ever Drift Apart
    fn layout() -> TouchLayout {
        TouchLayout::compute(Vec2::new(998.0, 448.0), 1.0)
    }

    #[test]
    fn menu_mode_maps_each_button_to_its_nav_bit() {
        let l = layout();
        let cases: [(Vec2, Box<dyn Fn(&MenuNav) -> bool>); 6] = [
            (l.menu_up.center(), Box::new(|n: &MenuNav| n.up)),
            (l.menu_down.center(), Box::new(|n: &MenuNav| n.down)),
            (l.menu_left.center(), Box::new(|n: &MenuNav| n.left)),
            (l.menu_right.center(), Box::new(|n: &MenuNav| n.right)),
            (l.menu_confirm.center(), Box::new(|n: &MenuNav| n.confirm)),
            (l.menu_back.center(), Box::new(|n: &MenuNav| n.cancel)),
        ];

        for (point, read_bit) in cases {
            let mut nav = MenuNav::default();
            assert!(apply_menu_touch(&mut nav, &l, TouchUiMode::Menu, point));
            assert!(read_bit(&nav), "wrong bit for tap at {point}");
        }
    }

    #[test]
    fn menu_mode_never_confirms_from_a_stray_tap() {
        // The Accidental-Quit Guard: Empty Glass in a Menu Must Contribute
        // Nothing at All, No Matter Where the Thumb Lands
        let l = layout();
        let mut nav = MenuNav::default();
        assert!(!apply_menu_touch(&mut nav, &l, TouchUiMode::Menu, l.stick_region.center()));
        assert_eq!(nav, MenuNav::default());
    }

    #[test]
    fn gameplay_mode_maps_pause_and_only_pause() {
        let l = layout();

        let mut nav = MenuNav::default();
        assert!(apply_menu_touch(&mut nav, &l, TouchUiMode::Gameplay, l.pause.center()));
        assert!(nav.pause);

        // The Fire Button and the Menu Cluster's Home Do Nothing Here
        let mut nav = MenuNav::default();
        assert!(!apply_menu_touch(&mut nav, &l, TouchUiMode::Gameplay, l.fire.center()));
        assert!(!apply_menu_touch(&mut nav, &l, TouchUiMode::Gameplay, l.menu_up.center()));
        assert_eq!(nav, MenuNav::default());
    }

    #[test]
    fn advance_mode_turns_any_tap_into_confirm() {
        let l = layout();
        let mut nav = MenuNav::default();
        assert!(apply_menu_touch(&mut nav, &l, TouchUiMode::Advance, Vec2::new(3.0, 3.0)));
        assert!(nav.confirm);
        assert!(!nav.pause && !nav.up && !nav.down && !nav.left && !nav.right && !nav.cancel);
    }

    #[test]
    fn off_mode_ignores_the_glass_entirely() {
        let l = layout();
        let mut nav = MenuNav::default();
        assert!(!apply_menu_touch(&mut nav, &l, TouchUiMode::Off, l.menu_confirm.center()));
        assert!(!apply_menu_touch(&mut nav, &l, TouchUiMode::Off, l.pause.center()));
        assert_eq!(nav, MenuNav::default());
    }
}
