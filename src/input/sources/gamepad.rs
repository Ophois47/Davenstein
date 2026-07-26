/*
Davenstein - by David Petnick

The gamepad source reads the ONE bound gamepad and merges its contribution into
the shared PlayerIntent accumulator. It applies its own radial deadzone from
ControlSettings because the GamepadSettings deadzone only applies on a settings
change and not when a pad connects mid-session

It reads a single device rather than every connected one because gilrs enumerates
flight sticks, throttle quadrants and rudder pedals alongside actual gamepads, and
a throttle lever resting at its travel limit reads as a permanently deflected stick.
Which device is bound, and why binding is claimed by deliberate input rather than by
connection, is documented in input::devices
*/

use bevy::prelude::*;

use crate::input::devices::ActiveGamepad;
use crate::input::intent::PlayerIntent;
use crate::input::menu::MenuNav;
use crate::options::ControlSettings;

// The Radial Deadzone Lives in the Parent sources Module Because the Touch Virtual
// Stick Needs the Identical Response Curve Past the Deadzone Edge, With Only the
// Deadzone Size Differing. Behavior on This Path Is Unchanged by the Move
use super::apply_deadzone;

// Look Rate for the Right Stick in Radians per Second
// Applied Each Frame as a Rate so Turning Speed is Framerate Independent
// Promote to ControlSettings if It Should Be Exposed in the Options Menu, Like KEY_TURN_SPEED
const GAMEPAD_LOOK_RATE: f32 = 2.5;

// Merge the Bound Gamepad into the Shared PlayerIntent Accumulator
// Runs After Keyboard and Mouse so Keyboard Keeps move_wish and weapon_select Priority
// Edges Use just_pressed and are Read in Update so They Never Double Fire in FixedUpdate
//
// Returns Whether This Source Supplied Any DELIBERATE Input This Frame, Which gather
// Uses to Maintain ActiveInputDevice. A Bound Pad Sitting Untouched Returns False
pub fn contribute(
    acc: &mut PlayerIntent,
    time: &Time,
    gamepads: &Query<&Gamepad>,
    active: &ActiveGamepad,
    controls: &ControlSettings,
) -> bool {
    let dt = time.delta_secs();
    let dz = controls.gamepad_deadzone;

    // No Binding Means No Gamepad Input At All. This Is the Normal State Until the
    // Player Actually Touches a Pad, and It Is What Makes an Attached Throttle Inert
    let Some(bound) = active.bound else {
        return false;
    };

    let Ok(gp) = gamepads.get(bound) else {
        return false;
    };

    let mut driven = false;

    // A Bare Block, Not a Loop
    //
    // This Used to Be 'for gp in gamepads.iter()'. Keeping the Braces Means the Body
    // Below Is Byte-for-Byte What It Always Was, so the Diff That Removed Multi-Device
    // Merging Shows Only the Scoping Change and Not a Reindent of Sixty Lines
    {
        // Movement From the Left Stick in the Local Player Frame
        // X = Strafe (+ = Right), Y = Forward (+ = Forward), Matching move_wish
        // Keyboard Priority: Fill move_wish Only When No Keyboard Movement This Frame
        let stick = apply_deadzone(gp.left_stick(), dz);
        if stick != Vec2::ZERO {
            driven = true;
        }
        if acc.move_wish == Vec2::ZERO && stick != Vec2::ZERO {
            acc.move_wish = stick;
        }

        // Look From the Right Stick, Applied as a Rate Because the Stick is a Position
        let rs = apply_deadzone(gp.right_stick(), dz);
        if rs != Vec2::ZERO {
            driven = true;
            let (look_x, look_y) = controls.scaled_gamepad_look(rs.x, rs.y);
            // Yaw: Pushing the Stick Right Turns Right, Matching Mouse and Keyboard Signs
            acc.look_delta.x -= look_x * GAMEPAD_LOOK_RATE * dt;
            // Pitch: Pushing Up Looks Up by Default and invert_y Flips the Sign
            // This is the Sign Most Likely to Need a Flip After Testing on Hardware
            let pitch = if controls.invert_y { -look_y } else { look_y };
            acc.look_delta.y += pitch * GAMEPAD_LOOK_RATE * dt;
        }

        // Run While the Left Stick is Clicked In
        acc.run |= gp.pressed(GamepadButton::LeftThumb);

        // Fire on the South Face Button (A on Xbox), Held Plus a One Frame Edge
        acc.fire |= gp.pressed(GamepadButton::South);
        acc.fire_pressed |= gp.just_pressed(GamepadButton::South);

        // Use or Open Door on the West Face Button (X on Xbox), One Frame Edge
        acc.use_pressed |= gp.just_pressed(GamepadButton::West);

        // Weapon Select on the D-Pad, Absolute 1..=4 Matching the Keyboard Slots
        // Up = 1 Knife, Right = 2 Pistol, Down = 3 MachineGun, Left = 4 Chaingun
        // or() Keeps Keyboard Priority When Both Fire the Same Frame
        let weapon = if gp.just_pressed(GamepadButton::DPadUp) {
            Some(1)
        } else if gp.just_pressed(GamepadButton::DPadRight) {
            Some(2)
        } else if gp.just_pressed(GamepadButton::DPadDown) {
            Some(3)
        } else if gp.just_pressed(GamepadButton::DPadLeft) {
            Some(4)
        } else {
            None
        };
        acc.weapon_select = acc.weapon_select.or(weapon);

        // Any Button at All Counts as Driving, Including Ones This Source Does Not
        // Bind. A Player Mashing an Unmapped Shoulder Button Is Still Plainly Using
        // the Pad, and ActiveInputDevice Should Reflect That
        if gp.get_pressed().next().is_some() {
            driven = true;
        }
    }

    driven
}

// Merge Bound Gamepad Menu Navigation Into the Shared MenuNav Accumulator
// D-Pad Moves, South Confirms, East Cancels, Start Opens the Pause Menu
//
// Reads the Bound Device for the Same Reason 'contribute' Does. A Flight Stick's Hat
// Switch Very Commonly Maps Onto the D-Pad, and Looping Over Every Device Here Meant
// a Hat Nudge Could Walk the Menu Selection or Confirm an Item the Player Never Chose
//
// Note This Cannot Bootstrap a Binding: Menus Are Reachable by Keyboard, and the
// Binding Is Claimed by 'bind_active_gamepad' From Any Button Press Regardless of
// Whether the Game Is in a Menu, so Pressing South on an Unbound Pad Binds It That
// Frame and Navigates on the Next
pub fn contribute_menu(nav: &mut MenuNav, gamepads: &Query<&Gamepad>, active: &ActiveGamepad) {
    let Some(bound) = active.bound else {
        return;
    };

    let Ok(gp) = gamepads.get(bound) else {
        return;
    };

    nav.up |= gp.just_pressed(GamepadButton::DPadUp);
    nav.down |= gp.just_pressed(GamepadButton::DPadDown);
    nav.left |= gp.just_pressed(GamepadButton::DPadLeft);
    nav.right |= gp.just_pressed(GamepadButton::DPadRight);
    nav.confirm |= gp.just_pressed(GamepadButton::South);
    nav.cancel |= gp.just_pressed(GamepadButton::East);
    nav.pause |= gp.just_pressed(GamepadButton::Start);
}
