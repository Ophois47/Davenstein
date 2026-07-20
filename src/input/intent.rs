/*
Davenstein - by David Petnick
*/

use bevy::prelude::*;

// Device-Neutral Snapshot of What the Player Wants to Do This Frame
// Every Input Source Writes Into This Resource
// Gameplay Systems Only Read It and Never Touch a Physical Device
// This Lets New Input Methods Add One Source System Without Changing Gameplay Code
// Rewritten in Full Every Frame by gather_input so It Never Becomes Stale
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PlayerIntent {
    // Desired Movement in the Player's Local Frame
    // X = Strafe (+ = Right), Y = Forward (+ = Forward)
    // Range is Approximately [-1, 1] per Axis
    // Analog Inputs Can Use Fractional Magnitudes Honored by player_move
    pub move_wish: Vec2,

    // Hold-to-Run
    pub run: bool,

    // Look Delta Already Scaled by Sensitivity and Invert Settings
    // Ready to Add Directly to LookAngles
    // X = Yaw Delta in Radians, Y = Pitch Delta in Radians
    pub look_delta: Vec2,

    // Fields Populated Now but Not Yet Consumed
    // These Keep the Struct Stable for Drop-In Door Use, Weapon Select, and Fire Migration
    // See the Notes in keyboard_mouse.rs
    // Fire Held
    pub fire: bool,

    // Fire Pressed This Frame for Semi-Automatic Weapons
    pub fire_pressed: bool,

    // Use or Open Door Pressed This Frame
    pub use_pressed: bool,

    // Weapon Slot Requested This Frame from 1 Through 4
    // Uses a Device-Neutral Index Rather Than the Binary's WeaponSlot Enum
    // This Keeps davelib Free of a Dependency on the Binary Crate
    pub weapon_select: Option<u8>,
}
