/*
Davenstein - by David Petnick

Input sources read physical devices and write into PlayerIntent
Multiple sources can contribute in the same frame, including mouse, stick, and
touch look input

Current and Planned Sources
- Keyboard and Mouse - Implemented
- Gamepad - Planned with Existing gamepad_sensitivity and gamepad_deadzone Settings
- Touch - Planned with a Virtual Stick, Look Drag Region, and On-Screen Buttons
*/

pub mod keyboard_mouse;
