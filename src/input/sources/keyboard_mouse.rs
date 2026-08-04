/*
Davenstein - by David Petnick

The keyboard and mouse source reads raw devices and ControlSettings, then writes
a fresh PlayerIntent every frame
*/

use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::input::intent::PlayerIntent;
use crate::input::menu::MenuNav;
use crate::options::ControlSettings;
use crate::player::cursor_is_captured;

// Base Sensitivity Applied on Top of ControlSettings.mouse_sensitivity
// Moved Verbatim from the Old player::mouse_look
const BASE_SENSITIVITY: f32 = 0.002;

// Wolf3D-Style Mouse Push-to-Move Threshold, in Pixels per Second of Vertical
// Mouse Motion. Pushing the Mouse Faster Than This Walks Forward, Pulling It
// Back Walks Backward. A RATE Threshold Rather Than a Per-Frame Pixel Count so
// the Feel Does Not Change With Framerate, and High Enough That the Incidental
// Y Drift of Ordinary Aiming Does Not Creep the Player Off Their Spot.
// move_wish Is Normalized Downstream in player.rs, so Crossing the Threshold
// Gives Normal Walk Speed - Exactly the Original's Push-to-Walk Feel
const MOUSE_MOVE_RATE_THRESHOLD: f32 = 150.0;

// Keyboard Turn Speeds in Radians per Second. Two-Tier, Faithful to Wolf3D Where
// Holding Run Turned Faster as Well as Moved Faster. The Base Rate Is Deliberately
// Low so a Tap Makes a Fine Aiming Adjustment Instead of Whipping Past the Target,
// While Holding Run Gives a Fast Spin for Getting Around. (Wolf3D Reserved Its
// Turn *Ramp* for the Joystick; Keyboard Turning Was a Flat Rate Scaled by Run.)
// Promote to ControlSettings if These Should Be Exposed in the Options Menu
const KEY_TURN_SPEED: f32 = 1.4;     // ~80 deg/s, Precise for Aiming
const KEY_TURN_SPEED_RUN: f32 = 3.0; // ~172 deg/s, Fast Spin While Running

// Merge Keyboard and Mouse Input into the Shared PlayerIntent Accumulator
// Called by the Neutral gather System as the Base Source Each Frame
// Freshness is Owned by gather Which Resets the Accumulator to Default
//
// Returns Whether Anything Was Actually Pressed or Moved This Frame, Which gather Uses
// to Maintain ActiveInputDevice. Keyboard and Mouse Win That Arbitration Over the Other
// Classes, so This Return Value Is What Lets Somebody at a Desk Reclaim the View
// Instantly From a Palm Resting on a Touchscreen
pub fn contribute(
    acc: &mut PlayerIntent,
    time: &Time,
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    mouse_motion: &AccumulatedMouseMotion,
    q_cursor: &Query<&CursorOptions, With<PrimaryWindow>>,
    controls: &ControlSettings,
) -> bool {
    let kb = &controls.key_bindings;

    // Movement in the Local Player Frame: X = Strafe, Y = Forward
    let mut wish = Vec2::ZERO;
    if keys.pressed(kb.move_forward) || keys.pressed(KeyCode::ArrowUp) {
        wish.y += 1.0;
    }
    if keys.pressed(kb.move_backward) || keys.pressed(KeyCode::ArrowDown) {
        wish.y -= 1.0;
    }
    if keys.pressed(kb.strafe_right) {
        wish.x += 1.0;
    }
    if keys.pressed(kb.strafe_left) {
        wish.x -= 1.0;
    }

    let run = keys.pressed(kb.run) || keys.pressed(KeyCode::ShiftRight);

    // Look Input
    let mut look = Vec2::ZERO;

    // Mouse Look Only When the Cursor is Captured and Mouselook is Enabled
    let captured = q_cursor
        .iter()
        .next()
        .is_some_and(|c| cursor_is_captured(c.grab_mode));

    if controls.mouselook_enabled && captured {
        let delta = mouse_motion.delta;
        if delta != Vec2::ZERO {
            // Apply the Sensitivity Multiplier and Invert Y Setting
            let (dx, dy) = controls.scaled_mouse_look(delta);
            look.x -= dx * BASE_SENSITIVITY; // Yaw
            look.y -= dy * BASE_SENSITIVITY; // Pitch
        }
    }

    // Mouse Push-to-Move (Wolf3D-Style): Mouse Y Drives Forward / Back Walking
    //
    // Sits Alongside Mouselook Rather Than Replacing It - Both Read the Same
    // Raw Delta, so With Both Enabled a Forward Push Walks AND Pitches, Which
    // Is How the Original Felt With Its Mouse Enabled. Deliberately Ignores
    // invert_y: That Is a LOOK Preference, and Pushing the Mouse Away From You
    // Should Always Mean Walking Forward. Raw (Unscaled) Delta Is Used so
    // mouse_sensitivity Keeps Meaning Look Speed Only; the Threshold Constant
    // Above Is the Movement Knob
    if controls.mouse_move_enabled && captured {
        let dt = time.delta_secs();
        if dt > 0.0 {
            // Bevy Mouse Delta Is Positive DOWNWARD (Toward the Player), so a
            // Forward Push Reads Negative and Must Map to +Forward
            let rate = -mouse_motion.delta.y / dt;
            if rate.abs() >= MOUSE_MOVE_RATE_THRESHOLD {
                wish.y += rate.signum();
            }
        }
    }

    // Keyboard Turning is Always Available so the Game is Fully Playable Without a Mouse
    // Uses Variable Delta Time Because Look is Applied Every Render Frame
    // Run Speeds the Turn Up (Faithful to Wolf3D), so the Low Base Rate Can Stay
    // Precise for Aiming Without Making Full Turns Feel Sluggish
    let dt = time.delta_secs();
    let turn_speed = if run { KEY_TURN_SPEED_RUN } else { KEY_TURN_SPEED };
    if keys.pressed(kb.turn_left) {
        look.x += turn_speed * dt;
    }
    if keys.pressed(kb.turn_right) {
        look.x -= turn_speed * dt;
    }

    // Action Edges Populated Now and Consumed Later
    // When the Fire Path Moves to Intent, Separate Left Click from Cursor Capture
    // The Default Fire Binding is ControlLeft, Which Also Releases the Cursor
    let fire = keys.pressed(kb.fire) || mouse_buttons.pressed(MouseButton::Left);
    let fire_pressed =
        keys.just_pressed(kb.fire) || mouse_buttons.just_pressed(MouseButton::Left);
    let use_pressed = keys.just_pressed(kb.use_door);

    let weapon_select = if keys.just_pressed(kb.weapon_1) {
        Some(1)
    } else if keys.just_pressed(kb.weapon_2) {
        Some(2)
    } else if keys.just_pressed(kb.weapon_3) {
        Some(3)
    } else if keys.just_pressed(kb.weapon_4) {
        Some(4)
    } else {
        None
    };

    // Merge This Frame Contribution into the Shared Accumulator
    // move_wish and look_delta Accumulate, Booleans Combine by OR
    // weapon_select Keeps the First Source That Sets it, so Keyboard Wins Here
    acc.move_wish += wish;
    acc.run |= run;
    acc.look_delta += look;
    acc.fire |= fire;
    acc.fire_pressed |= fire_pressed;
    acc.use_pressed |= use_pressed;
    acc.weapon_select = acc.weapon_select.or(weapon_select);

    // Deliberate Activity for ActiveInputDevice Arbitration
    //
    // Deliberately Broad: ANY Key or Mouse Button Down, or Any Pointer Motion at All,
    // Counts as the Player Being at the Desk. It Does Not Matter Whether the Key Is
    // Bound to Anything - Somebody Typing a Save Name or Hammering an Unmapped Key Is
    // Plainly Not Playing on a Phone
    //
    // Mouse Motion Is Checked Unconditionally Rather Than Through the Cursor-Capture
    // Gate Used for Look, Because Moving the Mouse in a Menu Should Reclaim Ownership
    // Just as Firmly as Moving It in Game
    keys.get_pressed().next().is_some()
        || mouse_buttons.get_pressed().next().is_some()
        || mouse_motion.delta != Vec2::ZERO
}

// Merge Keyboard Menu Navigation Into the Shared MenuNav Accumulator
// Arrows or WASD Move, Enter or Space Confirms, Escape Cancels
pub fn contribute_menu(nav: &mut MenuNav, keys: &ButtonInput<KeyCode>) {
    nav.up |= keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW);
    nav.down |= keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS);
    nav.left |= keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA);
    nav.right |= keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD);
    nav.confirm |= keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::NumpadEnter);
    nav.cancel |= keys.just_pressed(KeyCode::Escape);
}
