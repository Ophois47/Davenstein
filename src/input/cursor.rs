/*
Davenstein - by David Petnick
*/

//! Cursor Capture Policy. This Is a Device + Window Concern, so It Lives in the
//! Input Module Rather Than the Player Controller.
//!
//! Faithful Wolf3D Behavior: There Is No OS Cursor During Play or in Menus. The
//! Cursor Stays Hidden and Locked Whenever Mouselook Is On and the Window Is
//! Focused, and Releases Only When Mouselook Is Off (Keyboard-Only Play) or the
//! Window Loses Focus (Alt-Tab), Where the OS Reclaims the Pointer Anyway. It
//! Re-Captures on Its Own Once Focus Returns, With No Manual Release Key.
//!
//! NOTE (Platform): Programmatic Pointer Lock Works on Native Desktop, but the
//! Browser Requires a User Gesture to Lock. Before the WASM Push, Adapt This so
//! the First Capture Happens on a Click (Touch Devices Have No Cursor at All).

use bevy::prelude::*;
use bevy::window::{
    CursorGrabMode,
    CursorOptions,
    PrimaryWindow,
    Window,
};

use crate::options::ControlSettings;

pub fn grab_mouse(
    controls: Res<ControlSettings>,
    mut q_cursor: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
) {
    let Some((window, mut cursor)) = q_cursor.iter_mut().next() else {
        return;
    };

    // Capture Only While Mouselook Is On and the Window Is Focused. This Holds
    // in Both Gameplay and Menus, Matching the Original's Always-Hidden Cursor
    let want_capture = controls.mouselook_enabled && window.focused;
    let captured = cursor.grab_mode != CursorGrabMode::None;

    if want_capture && !captured {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    } else if !want_capture && captured {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
}
