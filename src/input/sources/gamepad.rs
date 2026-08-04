/*
Davenstein - by David Petnick

The gamepad source reads the ONE bound gamepad and merges its contribution into
the shared PlayerIntent accumulator. It applies its own radial deadzone from
ControlSettings because the GamepadSettings deadzone only applies on a settings
change and not when a pad connects mid-session

It reads a single device rather than every connected one because gilrs enumerates
flight sticks, throttle quadrants and rudder pedals alongside actual gamepads, and
a throttle lever resting at its travel limit reads as a permanently deflected stick.
Which device is bound, and why binding is claimed by deliberate input rather than by
connection, is documented in input::devices
*/

use bevy::prelude::*;

use crate::input::devices::ActiveGamepad;
use crate::input::intent::PlayerIntent;
use crate::input::menu::MenuNav;
use crate::options::ControlSettings;

// The Radial Deadzone Lives in the Parent sources Module Because the Touch Virtual
// Stick Needs the Identical Response Curve Past the Deadzone Edge, With Only the
// Deadzone Size Differing. Behavior on This Path Is Unchanged by the Move
use super::apply_deadzone;

// Look Rate for the Right Stick in Radians per Second
// Applied Each Frame as a Rate so Turning Speed is Framerate Independent
// Promote to ControlSettings if It Should Be Exposed in the Options Menu, Like KEY_TURN_SPEED
const GAMEPAD_LOOK_RATE: f32 = 2.5;

// Fixed Turn Rates for the D-Pad, in Radians per Second, Mirroring the Keyboard
// Arrows (KEY_TURN_SPEED / KEY_TURN_SPEED_RUN). The D-Pad Is Digital, so Unlike the
// Analog Right Stick It Turns at a Constant Rate While Held Rather Than Scaling With
// Deflection. Running Turns Faster, Matching the Keyboard Feel. This Is What Lets a
// Stickless Pad Aim by Turning ("Mouselook Off"): the View Sweeps at a Steady Speed
const DPAD_TURN_RATE: f32 = 1.4;
const DPAD_TURN_RATE_RUN: f32 = 3.0;

// Retrolink's 0079:0011 Retro Pads Expose the Physical D-Pad as Two Axes After
// the macOS Mapping Repair: Vertical on Left Stick Y, Horizontal on Right Stick X
const RETROLINK_VENDOR_ID: u16 = 0x0079;
const RETROLINK_PRODUCT_ID: u16 = 0x0011;
const MENU_AXIS_PRESS: f32 = 0.75;
const MENU_AXIS_RELEASE: f32 = 0.25;

// The Repaired Retro D-Pad Reports Full Deflection on Every Horizontal Press
// Halve Only This Profile's Yaw so Individual Taps Make Finer Turns
const RETROLINK_TURN_SCALE: f32 = 0.5;

// Held-State Latches Turn Continuous Retro-Pad Axes Into One Menu Edge per Press
// The Axis Must Return Near Centre Before the Same Direction Can Fire Again
#[derive(Default)]
pub struct RetroMenuAxisLatch {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl RetroMenuAxisLatch {
    fn clear(&mut self) {
        *self = Self::default();
    }
}

fn is_retrolink_retro_pad(gamepad: &Gamepad) -> bool {
    gamepad.vendor_id() == Some(RETROLINK_VENDOR_ID)
        && gamepad.product_id() == Some(RETROLINK_PRODUCT_ID)
}

fn axis_just_pressed(latched: &mut bool, value: f32) -> bool {
    if *latched {
        if value <= MENU_AXIS_RELEASE {
            *latched = false;
        }
        false
    } else if value >= MENU_AXIS_PRESS {
        *latched = true;
        true
    } else {
        false
    }
}

// Merge the Bound Gamepad into the Shared PlayerIntent Accumulator
// Runs After Keyboard and Mouse so Keyboard Keeps move_wish and weapon_select Priority
// Edges Use just_pressed and are Read in Update so They Never Double Fire in FixedUpdate
//
// Returns Whether This Source Supplied Any DELIBERATE Input This Frame, Which gather
// Uses to Maintain ActiveInputDevice. A Bound Pad Sitting Untouched Returns False
pub fn contribute(
    acc: &mut PlayerIntent,
    time: &Time,
    gamepads: &Query<&Gamepad>,
    active: &ActiveGamepad,
    controls: &ControlSettings,
) -> bool {
    let dt = time.delta_secs();
    let dz = controls.gamepad_deadzone;

    // No Binding Means No Gamepad Input At All. This Is the Normal State Until the
    // Player Actually Touches a Pad, and It Is What Makes an Attached Throttle Inert
    let Some(bound) = active.bound else {
        return false;
    };

    let Ok(gp) = gamepads.get(bound) else {
        return false;
    };

    let mut driven = false;

    // A Bare Block, Not a Loop
    //
    // This Used to Be 'for gp in gamepads.iter()'. Keeping the Braces Means the Body
    // Below Is Byte-for-Byte What It Always Was, so the Diff That Removed Multi-Device
    // Merging Shows Only the Scoping Change and Not a Reindent of Sixty Lines
    {
        // Run Is Held on the Left Shoulder (LB) for Pads That Have One, or the Left
        // Stick Click for Dual-Stick Pads. A Bare NES Pad Has Neither and Simply
        // Walks - Period-Accurate for a 1992 Console Shooter. Read Early Because the
        // D-Pad Turn Rate Below Depends on It
        let run = gp.pressed(GamepadButton::LeftTrigger) || gp.pressed(GamepadButton::LeftThumb);
        acc.run |= run;

        // Holding North Turns the D-Pad's Left/Right From TURNING Into STRAFING - the
        // Only Way a Stickless Pad Can Sidestep. Pads Too Sparse to Spare a Face
        // Button (NES) Simply Never Strafe
        let strafe_mod = gp.pressed(GamepadButton::North);

        // Movement Comes From Two Places, in Priority Order Behind the Keyboard:
        //   1. The Left Stick (Dual-Stick Pads), Analog, X = Strafe, Y = Forward
        //   2. The D-Pad (Every Pad), Digital: Up/Down = Forward/Back, and Left/Right
        //      = Strafe ONLY While North Is Held (Otherwise Left/Right Turns, Below)
        // Whichever Is Active Fills move_wish; the Stick Wins if Both Are Pushed
        let stick = apply_deadzone(gp.left_stick(), dz);

        let dpad_right = gp.pressed(GamepadButton::DPadRight);
        let dpad_left = gp.pressed(GamepadButton::DPadLeft);
        let dpad_lr = (dpad_right as i32 - dpad_left as i32) as f32;

        let mut dpad_move = Vec2::ZERO;
        if gp.pressed(GamepadButton::DPadUp) {
            dpad_move.y += 1.0;
        }
        if gp.pressed(GamepadButton::DPadDown) {
            dpad_move.y -= 1.0;
        }
        if strafe_mod {
            dpad_move.x += dpad_lr;
        }
        // Cap the Diagonal so Forward + Strafe Is Not Faster Than a Single Direction,
        // Matching the Stick Path Whose Magnitude Is Already Bounded by the Deadzone
        // Curve. A Pure Cardinal Stays at Full Speed
        dpad_move = dpad_move.clamp_length_max(1.0);

        if stick != Vec2::ZERO || dpad_move != Vec2::ZERO {
            driven = true;
        }
        if acc.move_wish == Vec2::ZERO {
            if stick != Vec2::ZERO {
                acc.move_wish = stick;
            } else if dpad_move != Vec2::ZERO {
                acc.move_wish = dpad_move;
            }
        }

        // Look From the Right Stick, Applied as a Rate Because the Stick is a Position
        let rs = apply_deadzone(gp.right_stick(), dz);
        if rs != Vec2::ZERO {
            driven = true;
            let (mut look_x, look_y) = controls.scaled_gamepad_look(rs.x, rs.y);

            // The Repaired Retro D-Pad Reports Full Deflection on Every Press, so
            // Reduce Only Its Horizontal Turn Rate and Leave Analog Pads Unchanged
            if is_retrolink_retro_pad(gp) {
                look_x *= RETROLINK_TURN_SCALE;
            }

            // Yaw: Pushing the Stick Right Turns Right, Matching Mouse and Keyboard Signs
            acc.look_delta.x -= look_x * GAMEPAD_LOOK_RATE * dt;
            // Pitch: Pushing Up Looks Up by Default and invert_y Flips the Sign
            // This is the Sign Most Likely to Need a Flip After Testing on Hardware
            let pitch = if controls.invert_y { -look_y } else { look_y };
            acc.look_delta.y += pitch * GAMEPAD_LOOK_RATE * dt;
        }

        // Turn From the D-Pad's Left/Right at the Fixed Rate, Unless North Is Held
        // (in Which Case Left/Right Already Strafed Above). Same Yaw Sign as the
        // Right Stick: Right Turns Right
        if !strafe_mod && dpad_lr != 0.0 {
            driven = true;
            let rate = if run { DPAD_TURN_RATE_RUN } else { DPAD_TURN_RATE };
            acc.look_delta.x -= dpad_lr * rate * dt;
        }

        // Fire on the South Face Button (A on Xbox) OR the Right Trigger, Each Held
        // Plus a One-Frame Edge. RT Is the Analog Trigger - RightTrigger2 in Bevy's
        // Naming, Where RightTrigger Is the Bumper - and Bevy Reports It as a Button
        // Once Its Pull Crosses the Default Press Threshold, so pressed / just_pressed
        // Behave Just Like a Face Button. South Stays Bound so It Keeps Doubling as
        // Menu Confirm and So Players Used to It Are Not Disrupted
        acc.fire |= gp.pressed(GamepadButton::South)
            || gp.pressed(GamepadButton::RightTrigger2);
        acc.fire_pressed |= gp.just_pressed(GamepadButton::South)
            || gp.just_pressed(GamepadButton::RightTrigger2);

        // Use / Open Door / Elevator on West (X on Xbox) OR East (B), a One-Frame
        // Edge. Bound to Both so "Use" Lands on a Natural Button Whatever the Pad:
        // East Is the NES B Button, West the Convenient Second Face Button Elsewhere.
        // Use Is Never Automatic - Doors and Secrets Are the Core Verb of the Game
        acc.use_pressed |= gp.just_pressed(GamepadButton::West)
            || gp.just_pressed(GamepadButton::East);

        // Weapon Switching Moved OFF the D-Pad (Which Now Drives Movement) and Onto
        // the Shoulders as a Relative Cycle Through Owned Weapons:
        //   Right Shoulder (RB) or Select = Next Weapon
        //   Left Trigger (LT) = Previous Weapon (Dual-Trigger Pads Only)
        // Select Gives a Bare NES Pad - Which Has No Shoulders - a Way to Change
        // Weapons at All. or() Keeps Keyboard's Absolute Select Priority the Same
        // Frame. The Consumer Skips Unowned Slots (See HudState::cycle_weapon)
        let mut step: i8 = 0;
        if gp.just_pressed(GamepadButton::RightTrigger)
            || gp.just_pressed(GamepadButton::Select)
        {
            step += 1;
        }
        if gp.just_pressed(GamepadButton::LeftTrigger2) {
            step -= 1;
        }
        if step != 0 {
            driven = true;
            acc.weapon_step = step;
        }

        // Any Button at All Counts as Driving, Including Ones This Source Does Not
        // Bind. A Player Mashing an Unmapped Shoulder Button Is Still Plainly Using
        // the Pad, and ActiveInputDevice Should Reflect That
        if gp.get_pressed().next().is_some() {
            driven = true;
        }
    }

    driven
}

// Merge Bound Gamepad Menu Navigation Into the Shared MenuNav Accumulator
// D-Pad Moves, South Confirms, East Cancels, Start Opens the Pause Menu
//
// Reads the Bound Device for the Same Reason 'contribute' Does. A Flight Stick's Hat
// Switch Very Commonly Maps Onto the D-Pad, and Looping Over Every Device Here Meant
// a Hat Nudge Could Walk the Menu Selection or Confirm an Item the Player Never Chose
//
// Note This Cannot Bootstrap a Binding: Menus Are Reachable by Keyboard, and the
// Binding Is Claimed by 'bind_active_gamepad' From Any Button Press Regardless of
// Whether the Game Is in a Menu, so Pressing South on an Unbound Pad Binds It That
// Frame and Navigates on the Next
pub fn contribute_menu(
    nav: &mut MenuNav,
    gamepads: &Query<&Gamepad>,
    active: &ActiveGamepad,
    retro_axes: &mut RetroMenuAxisLatch,
) {
    let Some(bound) = active.bound else {
        retro_axes.clear();
        return;
    };

    let Ok(gp) = gamepads.get(bound) else {
        retro_axes.clear();
        return;
    };

    nav.up |= gp.just_pressed(GamepadButton::DPadUp);
    nav.down |= gp.just_pressed(GamepadButton::DPadDown);
    nav.left |= gp.just_pressed(GamepadButton::DPadLeft);
    nav.right |= gp.just_pressed(GamepadButton::DPadRight);

    // The 0079:0011 macOS Repair Must Use Axes to Preserve Both Halves of Its
    // Physical D-Pad, so Recreate Digital Menu Edges With Hysteresis Here
    if is_retrolink_retro_pad(gp) {
        let vertical = gp.left_stick().y;
        let horizontal = gp.right_stick().x;

        nav.up |= axis_just_pressed(&mut retro_axes.up, vertical);
        nav.down |= axis_just_pressed(&mut retro_axes.down, -vertical);
        nav.left |= axis_just_pressed(&mut retro_axes.left, -horizontal);
        nav.right |= axis_just_pressed(&mut retro_axes.right, horizontal);
    } else {
        retro_axes.clear();
    }

    nav.confirm |= gp.just_pressed(GamepadButton::South);
    nav.cancel |= gp.just_pressed(GamepadButton::East);
    nav.pause |= gp.just_pressed(GamepadButton::Start);
}
