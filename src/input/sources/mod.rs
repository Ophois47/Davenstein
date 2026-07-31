/*
Davenstein - by David Petnick

Input sources read physical devices and write into PlayerIntent
Multiple sources can contribute in the same frame, including mouse, stick, and
touch look input

Current and Planned Sources
- Keyboard and Mouse - Implemented
- Gamepad - Implemented with the gamepad_sensitivity and gamepad_deadzone Settings
- Touch - Implemented with Floating Movement and Turn Sticks and On-Screen
  Buttons. Geometry Lives in input::touch_layout, and ui::touch_overlay Draws the
  Corresponding Controls
*/

pub mod keyboard_mouse;
pub mod gamepad;
pub mod touch;

use bevy::prelude::*;

// Apply a Radial Deadzone to a Raw Stick Vector and Rescale the Remainder
// Returns Zero Inside the Deadzone so a Resting Stick Produces No Input
// Outside the Deadzone Magnitude Ramps From Zero to One Preserving Direction
// Without This a Resting Residual Would Normalize to Full Speed in player_move
//
// Shared by Every Stick-Shaped Source so They All Ramp Identically: a Physical
// Gamepad Stick Using ControlSettings.gamepad_deadzone, and the Touch Virtual
// Stick Using Its Own Fixed Constant. A Thumb on Glass and a Worn Analog Stick
// Need Different Deadzone Sizes but the Same Response Curve Past the Edge
pub fn apply_deadzone(raw: Vec2, deadzone: f32) -> Vec2 {
    let len = raw.length();
    if len <= deadzone {
        return Vec2::ZERO;
    }
    // A Deadzone at or Past Full Travel Leaves No Live Range to Rescale Into and
    // Would Divide by Zero. Callers Clamp Well Below This, so It Is Pure Belt and
    // Braces for a Hand-Edited Settings File
    if deadzone >= 1.0 {
        return Vec2::ZERO;
    }
    // Rescale the Live Range so Motion Starts at Zero Just Past the Deadzone
    let scaled = ((len - deadzone) / (1.0 - deadzone)).min(1.0);
    raw / len * scaled
}
