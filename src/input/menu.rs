/*
Davenstein - by David Petnick
*/

use bevy::prelude::*;

// Device-Neutral Snapshot of Menu Navigation for This Frame
// Every Input Source Writes Into This Resource, Menu Code Only Reads It
// This Lets Keyboard, Gamepad, and Later Touch Drive Menus Through One Vocabulary
// Rewritten in Full Every Frame by gather so It Never Becomes Stale
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct MenuNav {
    // Move the Highlight Up or Down a List
    pub up: bool,
    pub down: bool,

    // Adjust the Focused Option Left or Right, Like a Slider or Toggle
    pub left: bool,
    pub right: bool,

    // Activate the Focused Item
    pub confirm: bool,

    // Go Back One Level or Close a Submenu
    pub cancel: bool,

    // Open the Pause Menu From Gameplay
    pub pause: bool,
}
