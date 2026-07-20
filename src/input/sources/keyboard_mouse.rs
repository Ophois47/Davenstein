/*
Davenstein - by David Petnick

The keyboard and mouse source reads raw devices and ControlSettings, then writes
a fresh PlayerIntent every frame
*/

use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::input::intent::PlayerIntent;
use crate::options::ControlSettings;
use crate::player::cursor_is_captured;

// Base Sensitivity Applied on Top of ControlSettings.mouse_sensitivity
// Moved Verbatim from the Old player::mouse_look
const BASE_SENSITIVITY: f32 = 0.002;

// Keyboard Turn Speed in Radians per Second
// Provides Keyboard Yaw When Mouselook is Disabled or Supplements Mouse Input
// Promote to ControlSettings if It Should Be Exposed in the Options Menu
const KEY_TURN_SPEED: f32 = 2.6;

// Read Keyboard and Mouse Input and Overwrite PlayerIntent for This Frame
// Writes a Full Snapshot Every Call so Unpressed Inputs are Cleared
// Prevents Stale Input Regardless of How Consumers are Gated
pub fn gather_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    q_cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    controls: Res<ControlSettings>,
    mut intent: ResMut<PlayerIntent>,
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

    *intent = PlayerIntent {
        move_wish: wish,
        run,
        look_delta: look,
        fire,
        fire_pressed,
        use_pressed,
        weapon_select,
    };
}
