/*
Davenstein - by David Petnick

Device Arbitration - Deciding WHICH Physical Device Is Driving the Game

Two Related Problems Live Here, Both of Which Come Down to the Same Thing: the
Input Pipeline Merges Every Source Every Frame and Until Now Had No Notion of a
Device Being Present but Not Actually In Use

1. ActiveGamepad
Bevy's Gamepad Backend Is gilrs, Which Enumerates HID GAME CONTROLLERS - and That
Set Includes Flight Sticks, Throttle Quadrants, Rudder Pedals, and Button Boxes.
The Gamepad Source Used to Loop Over Every Connected Device and Merge All of Them,
Which Is Fine Right Up Until One of Them Is a Throttle Lever

A Throttle Does Not Self-Centre. It Rests Wherever It Was Left, Usually at an Axis
Extreme. If gilrs Maps That Lever Onto a Stick Axis the Game Reads Permanent Full
Deflection, and Because look_delta ACCUMULATES Across Sources the Result Is Camera
Rotation That Cannot Be Stopped. Because move_wish Is Fill-When-Zero, a Parked Axis
Can Also Lock the Keyboard Out of Movement Entirely. The gamepad_deadzone Setting
Cannot Help: It Defaults to 0.1 and the Axis Is Sitting at 1.0

The Fix Is to Bind ONE Device and Read Only That One. Critically, the Binding Is
Claimed by DELIBERATE Input - a Button Press, or a Stick Deflection From a Device
That Has Been Observed at Rest - and Never Merely by Connecting. A Parked Throttle
Announces Itself Constantly and Must Never Be Allowed to Win That Race

2. ActiveInputDevice
Systems Outside the Input Layer Sometimes Need to Know Which Device Class the
Player Is Actually Using, Not Merely Which Ones Exist. The Motivating Case Is
'level_pitch_without_mouselook', Which Was Keyed on a Touch Being PRESENT - so on a
Touchscreen Laptop a Palm Resting on the Glass Flattened the Pitch of Somebody
Playing With a Mouse. Presence Is the Wrong Question; Activity Is the Right One

The Same Signal Is What the On-Screen Touch Overlay Will Use to Show and Hide
Itself, Which Is Why It Lives Here as Shared State Rather Than Inside the Touch
Source
*/

use std::collections::HashSet;

use bevy::prelude::*;

use crate::options::ControlSettings;

// Stick Magnitude Past Which a Deflection Counts as a Deliberate Claim
// Set High on Purpose. This Is Not a Deadzone, It Is "the Player Clearly Meant
// That", so It Wants to Sit Well Above Any Plausible Resting Drift
const CLAIM_DEFLECTION: f32 = 0.5;

// Stick Magnitude Within Which a Device Counts as Resting at Centre
// A Self-Centring Thumbstick Falls Inside This Within a Frame of Being Released.
// A Throttle Lever, Rudder Pedal, or Trim Wheel Parked at Its Travel Limit Never
// Will, Which Is Exactly the Discrimination This Whole Module Rests On
const CENTRE_BAND: f32 = 0.15;

// Device Class Currently Supplying Deliberate Input
//
// Updated by 'gather' Once per Frame From What the Sources Actually Contributed,
// Not From What Hardware Happens to Be Attached. Holds Its Value When Nothing Is
// Being Driven, so the Last Real Driver Stays the Owner Through Idle Frames
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveInputDevice {
    #[default]
    KeyboardMouse,
    Gamepad,
    Touch,
}

// The One Gamepad Whose Input Is Read, Plus the Bookkeeping That Picks It
#[derive(Resource, Debug, Default)]
pub struct ActiveGamepad {
    // Entity Whose Input Is Merged. None Until Something Deliberate Happens
    pub bound: Option<Entity>,

    // Devices Seen Resting at Centre at Least Once Since They Connected
    //
    // Membership Is the Gate on Claiming by Stick Deflection. It Is NOT Required for
    // Claiming by Button Press, Because a Button Press Is Unambiguous Intent no
    // Matter What the Axes Are Doing - Which Also Means a HOTAS Stick Can Still Be
    // Chosen Deliberately by Pressing One of Its Buttons
    centred: HashSet<Entity>,
}

impl ActiveGamepad {
    // True When This Device Has Been Observed at Rest and Its Axes Can Be Trusted
    pub fn has_centred(&self, gamepad: Entity) -> bool {
        self.centred.contains(&gamepad)
    }

    // Release the Binding and Forget Every Device Observation
    // Used When Gamepad Input Is Switched Off so a Held Button or a Deflected Stick
    // Cannot Survive as Phantom Input the Player Can No Longer Release
    pub fn clear(&mut self) {
        self.bound = None;
        self.centred.clear();
    }

    // True When Any Device Observation Is Being Held
    // Split Out so the Disabled-Path Guard in bind_active_gamepad Reads as One Intent
    fn has_any_observation(&self) -> bool {
        !self.centred.is_empty()
    }
}

// Maintain the Gamepad Binding: Prune, Observe, Then Claim
//
// Registered Before the InputGather Set so the Binding Is Current by the Time the
// Gamepad Source Reads It. Deliberately Cheap Enough to Run Unconditionally: the
// Whole Body Is Skipped When There Are No Gamepads, Which Is the Common Case
pub fn bind_active_gamepad(
    controls: Res<ControlSettings>,
    q_gamepads: Query<(Entity, &Gamepad, Option<&Name>)>,
    mut active: ResMut<ActiveGamepad>,
) {
    if !controls.gamepad_enabled {
        // The is_none Guard Keeps This From Writing Through ResMut - and Tripping
        // Change Detection - on Every Frame That Gamepads Are Simply Switched Off
        if active.bound.is_some() || active.has_any_observation() {
            active.clear();
        }
        return;
    }

    // PASS 1 - PRUNE
    // Drop the Binding if Its Device Is Gone, and Forget Observations for Devices
    // That Have Disconnected
    //
    // The Forgetting Matters More Than It Looks: Bevy Recycles Entity Indices, so a
    // Stale Entry Left Behind by an Unplugged Thumbstick Could Mark a Newly Attached
    // Throttle as Already Centred and Hand It the Very Trust This Module Withholds
    if let Some(bound) = active.bound {
        if q_gamepads.get(bound).is_err() {
            active.bound = None;
        }
    }

    if !active.centred.is_empty() {
        let live: HashSet<Entity> = q_gamepads.iter().map(|(entity, _, _)| entity).collect();
        active.centred.retain(|entity| live.contains(entity));
    }

    // PASS 2 - OBSERVE
    // Note Every Device Currently Resting at Centre. Once Recorded This Never
    // Expires for the Life of the Connection: a Stick Only Has to Prove It Can Rest
    // Once, and Demanding It Prove So Again Later Would Reject a Stick Being Held
    for (entity, gamepad, _) in q_gamepads.iter() {
        if active.centred.contains(&entity) {
            continue;
        }

        let resting = gamepad.left_stick().length() <= CENTRE_BAND
            && gamepad.right_stick().length() <= CENTRE_BAND;

        if resting {
            active.centred.insert(entity);
        }
    }

    // PASS 3 - CLAIM
    // Nothing Below Runs While a Binding Is Held, so the Player Keeps the Device They
    // Are Using Until They Put It Down and Pick Up Another
    if active.bound.is_some() {
        return;
    }

    for (entity, gamepad, name) in q_gamepads.iter() {
        // A Button Press Is Unambiguous Intent and Needs No Axis Vetting
        let button = gamepad.get_just_pressed().next().is_some();

        // A Stick Deflection Only Counts From a Device That Has Proven It Can Rest.
        // This Single Condition Is What Stops a Throttle Parked at Full Travel From
        // Claiming the Binding on the First Frame of Every Session
        let deflection = active.centred.contains(&entity)
            && (gamepad.left_stick().length() >= CLAIM_DEFLECTION
                || gamepad.right_stick().length() >= CLAIM_DEFLECTION);

        if button || deflection {
            active.bound = Some(entity);

            // Logged Because on a Multi-Device Rig This Is the Single Most Useful
            // Line in the Console When Input Behaves Unexpectedly
            let label = name.map(|n| n.as_str().to_string()).unwrap_or_else(|| format!("{entity}"));
            info!(
                "##==> Gamepad Bound: {} (claimed by {})",
                label,
                if button { "button" } else { "stick" },
            );
            return;
        }
    }
}
