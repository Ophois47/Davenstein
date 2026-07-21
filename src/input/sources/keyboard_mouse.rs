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

// Keyboard Turn Speed in Radians per Second
// Provides Keyboard Yaw When Mouselook is Disabled or Supplements Mouse Input
// Promote to ControlSettings if It Should Be Exposed in the Options Menu
const KEY_TURN_SPEED: f32 = 2.6;

// Merge Keyboard and Mouse Input into the Shared PlayerIntent Accumulator
// Called by the Neutral gather System as the Base Source Each Frame
// Freshness is Owned by gather Which Resets the Accumulator to Default
pub fn contribute(
    acc: &mut PlayerIntent,
    time: &Time,
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    mouse_motion: &AccumulatedMouseMotion,
    q_cursor: &Query<&CursorOptions, With<PrimaryWindow>>,
    controls: &ControlSettings,
) {
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

    // Keyboard Turning is Always Available so the Game is Fully Playable Without a Mouse
    // Uses Variable Delta Time Because Look is Applied Every Render Frame
    let dt = time.delta_secs();
    if keys.pressed(kb.turn_left) {
        look.x += KEY_TURN_SPEED * dt;
    }
    if keys.pressed(kb.turn_right) {
        look.x -= KEY_TURN_SPEED * dt;
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
