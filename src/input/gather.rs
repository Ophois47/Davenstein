/*
Davenstein - by David Petnick

Neutral Per-Frame Intent Gather

This System is the Single Writer of PlayerIntent. It Resets to Default Each
Frame, Lets Every Source Merge a Contribution, Then Commits One Merged Result
Resetting to Default First is What Keeps Unpressed Inputs From Going Stale

While PlayerControlLock is Held (Menus, Death, Level End) the Gameplay Intent is
Silenced HERE, at the Source, Rather Than Trusting Every Downstream Consumer to
Remember its Lock Gate. MenuNav is Deliberately Exempt or Menus Could Not be Driven.
A Release Latch Then Keeps Fire Suppressed Across the Unlock Edge Until the Trigger
is Actually Let Go: on a Gamepad the South Button is Both Menu Confirm and Fire, so
the Press That Chose "Back to Game" Was Still Held on the First Unlocked Frames and
Fired the Weapon. Keyboard Only Escaped the Same Bug by Accident of Binding (Enter
Confirms, Ctrl Fires); a Clicked Menu Would Leak the Same Way Through Left Mouse

Merge Contract Honored by Each Source contribute Function
- Vectors move_wish and look_delta Accumulate Additively
- Booleans run and fire and fire_pressed and use_pressed Combine by OR
- weapon_select Keeps the First Source That Sets it, so Call Order is Priority
- move_wish Uses Keyboard Priority, so Later Sources Fill Only When Still Zero

Keyboard and Mouse Runs First and Establishes the Base. Gamepad Merges on Top of
It, Then Touch Merges Last, so the Two Device Classes a Desktop Player Uses Keep
Priority Over the On-Screen Controls
*/

use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input::touch::Touches;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::input::intent::PlayerIntent;
use crate::input::menu::MenuNav;
use crate::input::devices::{ActiveGamepad, ActiveInputDevice};
use crate::input::sources::keyboard_mouse;
use crate::input::sources::gamepad;
use crate::input::sources::touch;
use crate::input::sources::touch::TouchAssignments;
use crate::input::touch_layout::TouchLayout;
use crate::options::ControlSettings;

// Read Every Input Source and Commit One Merged PlayerIntent for This Frame
// Resetting the Accumulator to Default Each Frame Clears Unpressed Inputs
pub fn gather(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    q_cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    q_gamepads: Query<&Gamepad>,
    touches: Res<Touches>,
    controls: Res<ControlSettings>,
    // Screen Geometry of the On-Screen Controls, Rebuilt Before This Set Runs
    touch_layout: Res<TouchLayout>,
    // Which Finger Owns Which Held Touch Control. Persists Across Frames, So It
    // Cannot Be a Local: the Overlay Reads It to Draw the Stick Under the Thumb
    mut touch_assign: ResMut<TouchAssignments>,
    // The One Gamepad Whose Input Is Read, Maintained by devices::bind_active_gamepad
    active_gamepad: Res<ActiveGamepad>,
    // Which Device Class Is Actually Driving. Written at the End of This System
    mut active_device: ResMut<ActiveInputDevice>,
    // Optional Because the Binary's main Initializes the Resource; a Bare davelib
    // App (Tests, Tools) Simply Runs Unlocked
    lock: Option<Res<crate::player::PlayerControlLock>>,
    // Armed While Control is Locked; Keeps Fire Suppressed After Unlock Until the
    // Trigger is Released Once. A Local Because it is This System's Private Edge State
    mut fire_release_latch: Local<bool>,
    mut intent: ResMut<PlayerIntent>,
    mut menu: ResMut<MenuNav>,
) {
    let mut acc = PlayerIntent::default();

    // Keyboard and Mouse Establishes the Base Intent for This Frame
    let km_driven = keyboard_mouse::contribute(
        &mut acc,
        &time,
        &keys,
        &mouse_buttons,
        &mouse_motion,
        &q_cursor,
        &controls,
    );

    // Gamepad Merges on Top of the Base so Keyboard Keeps Priority
    // Skipped Entirely When the Gamepad Toggle is Off
    let gp_driven = if controls.gamepad_enabled {
        gamepad::contribute(&mut acc, &time, &q_gamepads, &active_gamepad, &controls)
    } else {
        false
    };

    // Touch Merges Last so Keyboard and Gamepad Both Keep move_wish and
    // weapon_select Priority. Skipped Entirely When the Touch Toggle Is Off, and the
    // Assignments Are Dropped on the Way Out so a Finger That Was Holding Fire When
    // Touch Was Switched Off Cannot Leave the Trigger Stuck Down. The is_idle Guard
    // Keeps That Cleanup From Tripping Change Detection Every Frame on Desktop
    let touch_driven = if controls.touch_enabled {
        touch::contribute(
            &mut acc,
            &touches,
            &mut touch_assign,
            &touch_layout,
            &controls,
        )
    } else {
        if !touch_assign.is_idle() {
            touch_assign.clear();
        }
        false
    };

    // ARBITRATION - Which Device Class Is Actually Driving
    //
    // Priority Within a Single Frame Is Keyboard/Mouse, Then Gamepad, Then Touch. That
    // Order Is Not Arbitrary: Somebody at a Desk Must Be Able to Reclaim the View
    // Instantly, Even While a Palm Is Resting on a Touchscreen or a Bound Pad Sits With
    // a Thumb On It. Losing Ownership to a Device You Are Not Looking At Is Far More
    // Annoying Than Keeping It a Frame Too Long
    //
    // Nothing Is Written When No Source Is Being Driven, so the Last Real Driver Stays
    // the Owner Through Idle Frames. That Persistence Is What Makes This Usable as a
    // Steady Signal Rather Than One That Flickers Between Frames - a Player Who Lifts
    // Their Thumb to Read the HUD Has Not Stopped Playing by Touch
    //
    // Written Through a Change-Detected Guard Because Consumers Are Expected to React
    // to Transitions, Not to Poll
    let driving = if km_driven {
        Some(ActiveInputDevice::KeyboardMouse)
    } else if gp_driven {
        Some(ActiveInputDevice::Gamepad)
    } else if touch_driven {
        Some(ActiveInputDevice::Touch)
    } else {
        None
    };

    if let Some(driving) = driving {
        if *active_device != driving {
            *active_device = driving;
        }
    }

    let locked = lock.map(|l| l.0).unwrap_or(false);

    if locked {
        // Menus, Death, and Level End: the Input Layer Itself Goes Quiet Instead of
        // Every Consumer Individually Remembering to Check the Lock. Arming the Latch
        // Here is What Makes the Unlock Edge Safe Below
        *fire_release_latch = true;
        acc = PlayerIntent::default();
    } else if *fire_release_latch {
        // First Unlocked Frames: the Finger That Confirmed "Back to Game" is Usually
        // Still Down on a Button That Doubles as Fire. Hold Fire at False Until Every
        // Fire Input Reads Released for a Frame, Then Disarm. Movement and Look Pass
        // Through Untouched -- Walking Out of a Menu Should Feel Instant, Only the
        // Trigger Needs the Release
        if acc.fire || acc.fire_pressed {
            acc.fire = false;
            acc.fire_pressed = false;
        } else {
            *fire_release_latch = false;
        }
    }

    *intent = acc;

    // Menu Navigation Uses the Same Reset-Then-Merge Pattern as PlayerIntent
    let mut nav = MenuNav::default();
    keyboard_mouse::contribute_menu(&mut nav, &keys);
    if controls.gamepad_enabled {
        gamepad::contribute_menu(&mut nav, &q_gamepads, &active_gamepad);
    }
    if controls.touch_enabled {
        touch::contribute_menu(&mut nav, &touches, &touch_layout);
    }
    *menu = nav;
}
