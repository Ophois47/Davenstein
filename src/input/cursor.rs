/*
Davenstein - by David Petnick

Cursor capture and release are device and window concerns, so they live in the
input module rather than the player controller

This was moved verbatim from 'player::grab_mouse' with behavior unchanged to
keep the relocation behavior-preserving

Known rough edges to address in a later pass
- 'ControlLeft' releases the cursor here and is also the default 'fire' binding
- Left or right click grabs the cursor, but once captured a click should fire
  Both behaviors should be separated when the fire path moves to 'PlayerIntent'

Platform Note
The startup auto-grab and pointer-lock logic below is desktop-only
Before the WASM and mobile work, gate this system with
'#[cfg(not(target_arch = "wasm32"))]'
Browsers require a user gesture to lock the pointer and touch devices have no
cursor
*/

use bevy::prelude::*;
use bevy::window::{
    CursorGrabMode,
    CursorOptions,
    PrimaryWindow,
    Window,
};

use crate::player::PlayerControlLock;

pub fn grab_mouse(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    lock: Res<PlayerControlLock>,
    mut startup_frames: Local<u8>,
    mut startup_grab_done: Local<bool>,
    mut q_cursor: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
) {
    let Some((window, mut cursor)) = q_cursor.iter_mut().next() else {
        return;
    };

    // Release Cursor with Left Control
    // Works Even While Menus are Open or Controls are Locked
    if keys.just_pressed(KeyCode::ControlLeft) {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
        *startup_grab_done = true;
        return;
    }

    // Startup Grab Must Wait Until the OS Has Mapped and Focused the Window
    if !*startup_grab_done {
        if *startup_frames < 3 {
            *startup_frames += 1;
            return;
        }

        if window.focused {
            cursor.visible = false;
            cursor.grab_mode = CursorGrabMode::Locked;
            *startup_grab_done = true;
            return;
        }
    }

    // Do Not Auto-Release Just Because a Menu is Open or Controls are Locked
    // Menus are Keyboard Only
    if lock.0 {
        return;
    }

    // Grab Cursor by Clicking the Window After Manual Release
    if mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right) {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    }
}
