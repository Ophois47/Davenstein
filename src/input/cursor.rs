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
        // Lock the Cursor Whenever ANY Feature Needs Raw Relative Motion: Mouselook
        // (Turning/Looking) OR Mouse Push-to-Move (Mouse-Y Walking). Previously This
        // Gated on mouselook_enabled Alone, so Turning Mouselook OFF to Use Mouse-Move
        // Released the Grab and Left mouse_move With No Captured Cursor to Read -- the
        // "Mouse Move Does Nothing" Bug. Reading Motion Requires a Locked Cursor
        // Either Way, so Either Toggle Being On Locks It. With Both Off the Cursor Is
        // Freed for Keyboard-Only Play
        cursor.grab_mode = if controls.mouselook_enabled || controls.mouse_move_enabled {
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
