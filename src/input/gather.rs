/*
Davenstein - by David Petnick

Neutral Per-Frame Intent Gather

This System is the Single Writer of PlayerIntent. It Resets to Default Each
Frame, Lets Every Source Merge a Contribution, Then Commits One Merged Result
Resetting to Default First is What Keeps Unpressed Inputs From Going Stale

Merge Contract Honored by Each Source contribute Function
- Vectors move_wish and look_delta Accumulate Additively
- Booleans run and fire and fire_pressed and use_pressed Combine by OR
- weapon_select Keeps the First Source That Sets it, so Call Order is Priority
- move_wish Uses Keyboard Priority, so Later Sources Fill Only When Still Zero

Keyboard and Mouse Runs First and Establishes the Base. Gamepad and Touch
Merge on Top of the Base in Later Milestones
*/

use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::input::intent::PlayerIntent;
use crate::input::sources::keyboard_mouse;
use crate::options::ControlSettings;

// Read Every Input Source and Commit One Merged PlayerIntent for This Frame
// Resetting the Accumulator to Default Each Frame Clears Unpressed Inputs
pub fn gather(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    q_cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    controls: Res<ControlSettings>,
    mut intent: ResMut<PlayerIntent>,
) {
    let mut acc = PlayerIntent::default();

    // Keyboard and Mouse Establishes the Base Intent for This Frame
    keyboard_mouse::contribute(
        &mut acc,
        &time,
        &keys,
        &mouse_buttons,
        &mouse_motion,
        &q_cursor,
        &controls,
    );

    // Additional Input Sources Merge on Top of the Base Here
    // Gamepad Contributes Next, Then Touch in a Later Milestone

    *intent = acc;
}
