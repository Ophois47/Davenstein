/*
Davenstein - by David Petnick

The gamepad source reads every connected gamepad and merges its contribution
into the shared PlayerIntent accumulator. Deadzone is applied upstream by the
GamepadSettings system in options.rs so the stick reads arrive pre-filtered
*/

use bevy::prelude::*;

use crate::input::intent::PlayerIntent;
use crate::options::ControlSettings;

// Look Rate for the Right Stick in Radians per Second
// Applied Each Frame as a Rate so Turning Speed is Framerate Independent
// Promote to ControlSettings if It Should Be Exposed in the Options Menu, Like KEY_TURN_SPEED
const GAMEPAD_LOOK_RATE: f32 = 2.5;

// Merge Every Connected Gamepad into the Shared PlayerIntent Accumulator
// Runs After Keyboard and Mouse so Keyboard Keeps move_wish and weapon_select Priority
// Edges Use just_pressed and are Read in Update so They Never Double Fire in FixedUpdate
pub fn contribute(
    acc: &mut PlayerIntent,
    time: &Time,
    gamepads: &Query<&Gamepad>,
    controls: &ControlSettings,
) {
    let dt = time.delta_secs();

    for gp in gamepads.iter() {
        // Movement From the Left Stick in the Local Player Frame
        // X = Strafe (+ = Right), Y = Forward (+ = Forward), Matching move_wish
        // Keyboard Priority: Fill move_wish Only When No Keyboard Movement This Frame
        let stick = gp.left_stick();
        if acc.move_wish == Vec2::ZERO && stick != Vec2::ZERO {
            acc.move_wish = stick.clamp_length_max(1.0);
        }

        // Look From the Right Stick, Applied as a Rate Because the Stick is a Position
        let rs = gp.right_stick();
        if rs != Vec2::ZERO {
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
    }
}
