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

    if window.focused {
        // While Focused We Always Want the Cursor Hidden, and Locked if Mouselook Is
        // On (None Otherwise for Keyboard-Only Play, so the Mouse Stops Turning).
        //
        // Re-Assert BOTH Every Frame Rather Than Only on Change. macOS Can Silently
        // Show the Cursor or Drop the Lock Out From Under winit - a System Event, a
        // Momentary Focus Flicker, the Pointer Reaching a Screen Edge - Without
        // Bevy's Cached CursorOptions Noticing. The Old "Write Only When != Desired"
        // Guard Then Never Re-Applied, Because the Stale Cache Still Matched the
        // Desired Value, and the Cursor Stayed Up Until Focus Toggled. Writing the
        // Fields Unconditionally Marks CursorOptions Changed, so Bevy Re-Issues the
        // winit Grab / Hide and Recovers Within a Frame. Re-Locking When Already
        // Locked Is Idempotent, so There Is No Cursor Jump During Normal Play
        cursor.visible = false;
        // Capture (Lock) the Cursor Whenever the Mouse Mode Needs Raw Relative
        // Motion -- Look (Yaw/Pitch) and Move (Turn/Walk) Both Do; Off Does Not.
        // Gating on the Mode's needs_capture Keeps This in One Place and Means
        // Switching Between Look and Move Never Drops the Grab
        cursor.grab_mode = if controls.mouse_mode.needs_capture() {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
    } else {
        // Not Focused: the OS Owns the Cursor (Alt-Tab, Notification, Mission
        // Control). Show It and Release the Grab - on Change Only, so We Do Not
        // Thrash winit While the App Sits in the Background
        if !cursor.visible {
            cursor.visible = true;
        }
        if cursor.grab_mode != CursorGrabMode::None {
            cursor.grab_mode = CursorGrabMode::None;
        }
    }
}
