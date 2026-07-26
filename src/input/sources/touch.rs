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
pub fn contribute(
    acc: &mut PlayerIntent,
    touches: &Touches,
    assign: &mut TouchAssignments,
    layout: &TouchLayout,
    controls: &ControlSettings,
) {
    // No Measured Window Yet Means No Meaningful Geometry to Test Against
    if !layout.is_ready() {
        return;
    }

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
            continue;
        }

        // Use / Open Door Is a Pure Edge, One Door per Tap
        if layout.use_door.contains(point) {
            acc.use_pressed = true;
            continue;
        }

        // Weapon Select Keeps the First Source That Set It, Matching Keyboard Priority
        if let Some(slot) = layout.weapon_slot_at(point) {
            acc.weapon_select = acc.weapon_select.or(Some(slot));
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

            // Normalize to Unit Travel, Then Apply the Same Radial Deadzone Shape the
            // Gamepad Uses so Both Sticks Ramp Identically From Their Deadzone Edge
            let raw = offset / layout.stick_radius;
            let stick = super::apply_deadzone(raw, STICK_DEADZONE);

            // Screen Y Grows Downward, so Dragging UP Is Forward. Negating Y Here Is
            // the Whole Conversion Into move_wish's Local Player Frame. Stick X Stays
            // Strafe, Matching the Keyboard and the Left Stick, Because Turning Lives
            // on the Drag Where It Gets 1:1 Positional Control Instead of a Rate
            let wish = Vec2::new(stick.x, -stick.y);

            // Keyboard and Gamepad Priority: Fill move_wish Only if Still Untouched
            if acc.move_wish == Vec2::ZERO && wish != Vec2::ZERO {
                acc.move_wish = wish;
            }

            // Run in the Outer Ring. Tested After the Deadzone Rescale, so RUN_RING
            // Is a Fraction of Usable Travel Rather Than of Raw Distance
            acc.run |= stick.length() >= RUN_RING;
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
}

// Merge Touch Menu Navigation Into the Shared MenuNav Accumulator
// Currently the Pause Button Only, Which Is Enough for a Touch-Only Player to Open
// the Menu and Reach Every Existing Escape Path
//
// There Is Intentionally No Tap-Anywhere-Confirms Here. It Would Be One Line and It
// Would Be a Bug: MenuNav.confirm Activates Whatever Item Is Highlighted, so a
// Stray Thumb Anywhere on the Glass Could Pick "Quit" Out of the Pause Menu. Menu
// Navigation Gets Real Buttons When the Overlay Lands and They Can Be Seen and Aimed
// At. Until Then Menus Stay on Keyboard and Gamepad
pub fn contribute_menu(nav: &mut MenuNav, touches: &Touches, layout: &TouchLayout) {
    if !layout.is_ready() {
        return;
    }

    for touch in touches.iter_just_pressed() {
        nav.pause |= layout.pause.contains(touch.position());
    }
}
