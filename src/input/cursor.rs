/*
Davenstein - by David Petnick
*/

//! Cursor Capture Policy. This Is a Device + Window Concern, so It Lives in the
//! Input Module Rather Than the Player Controller
//!
//! Faithful Wolf3D Behavior: There Is No OS Cursor While the Window Is Focused,
//! in Gameplay or in Menus Alike. The Cursor Is Hidden Whenever Focused and
//! Reappears Only on Focus Loss (Alt-Tab), Where the OS Reclaims It Anyway.
//! Mouselook Only Controls Whether the Mouse Is Locked for Relative Look. With
//! Mouselook Off (Keyboard-Only Play) the Cursor Stays Hidden but Unlocked, so
//! the Mouse No Longer Turns the Player
//!
//! NOTE (Platform): Programmatic Pointer Lock Works on Native Desktop, but the
//! Browser Requires a User Gesture to Lock. Before the WASM Push, Adapt This so
//! the First Lock Happens on a Click (Touch Devices Have No Cursor at All)

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

    // Hide the Cursor Whenever the Window Is Focused (Gameplay and Menus). Only
    // Show It Again on Focus Loss so Alt-Tab Behaves Normally
    let want_visible = !window.focused;

    // Lock the Mouse for Relative Look Only When Mouselook Is On and Focused.
    // With Mouselook Off the Cursor Stays Hidden but Unlocked (Keyboard-Only)
    let want_grab = if controls.mouselook_enabled && window.focused {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };

    if cursor.visible != want_visible {
        cursor.visible = want_visible;
    }
    if cursor.grab_mode != want_grab {
        cursor.grab_mode = want_grab;
    }
}
