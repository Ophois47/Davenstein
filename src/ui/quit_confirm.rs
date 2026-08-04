/*
Davenstein - by David Petnick

The Original's Sarcastic Quit Prompt, Restored as a Modal Overlay.

When You Chose QUIT in Wolf3D's WL_MENU.C, Message() Drew a Grey Box Over the
Frozen Frame Bearing One of Nine Random Taunts, and the Game Waited on a Y / N
Answer: Y Quit to DOS, N (or Escape) Returned You to the Menu. This Module
Reproduces That Modal Directly on Top of the Cheat-Warning Machinery, Which
Already Solved the Hard Parts (Freeze via PlayerControlLock, Draw Over the Last
Frame, Release Cleanly).

The One Structural Difference From cheat_message.rs Is the Verdict: the Cheat Box
Dismisses on ANY Input, Whereas This Box Branches. Y or a Gamepad Confirm Quits;
N, Escape, a Gamepad Cancel, or Any Other Key Backs Out. That Split Is the Whole
Reason This Is Its Own Module Rather Than a Second CHEAT_MESSAGE_TEXT.

Two Edges Handled the Same Way the Cheat Box Handles Them:

- just_opened Absorbs the Frame the Prompt Was Raised, so the Enter or Gamepad
  South That Confirmed the QUIT Menu Row Cannot Be Read as a Y on the Very Frame
  the Box Appears. Without It, Selecting Quit With Enter Would Quit Instantly.
- The Splash Machine's Menu Branches Bail While the Lock Is Held, so Escape and
  the Gamepad Start Button Cannot Reopen a Menu Underneath This Box -- They Are
  Simply Read as "No". The Lock Is Handed Back on a No; on a Yes the App Exits
  Before Anyone Cares About the Lock.
*/

use bevy::prelude::*;

use rand::RngExt;

use davelib::input::MenuNav;
use davelib::player::PlayerControlLock;

use super::splash::{self, SplashStep};

/// Marker on the Overlay Root so the Y / N Handler Can Find and Despawn the Box
#[derive(Component)]
pub struct QuitConfirmUi;

/// The Nine Official Wolfenstein 3-D Quit Taunts, Verbatim From WL_MENU.C's
/// endStrings[]. One Is Chosen at Random Each Time the Prompt Opens
const QUIT_MESSAGES: [&str; 9] = [
    "Dost thou wish to\nleave with such hasty\nabandon?",
    "Chickening out...\nalready?",
    "Press N for more carnage.\nPress Y to be a weenie.",
    "So, you think you can\nquit this easily, huh?",
    "Press N to save the world.\nPress Y to abandon it in\nits hour of need.",
    "Press N if you are brave.\nPress Y to cower in shame.",
    "Heroes, press N.\nWimps, press Y.",
    "You are at an intersection.\nA sign says, 'Press Y to quit.'\n>",
    "For guns and glory, press N.\nFor work and worry, press Y.",
];

/// Lifecycle State. active Marks the Box Up; just_opened Absorbs the Keypress
/// That Selected the QUIT Row so It Cannot Be Read as the Y / N Answer
#[derive(Resource, Default)]
pub struct QuitConfirmState {
    active: bool,
    just_opened: bool,
    /// Holds the Dismissal Input Until It Has Been Released so the Same Escape,
    /// Mouse, or Gamepad Press Cannot Also Act on the Menu Under the Modal
    input_latched: bool,
}

impl QuitConfirmState {
    /// Whether the Prompt Is Currently Up. Read by the Touch Overlay's Mode
    /// Sync, Which Must Treat the Frozen Game Behind This Box as an Any-Input
    /// Screen Rather Than Live Gameplay, Exactly as It Does the Cheat Box
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// Whether the Menu State Machine Must Ignore Input for the Modal
    pub(crate) fn blocks_menu_input(&self) -> bool {
        self.active || self.input_latched
    }
}

/// Raise the Quit Prompt: Freeze the World and Draw the Box With a Random Taunt.
/// Called From the Splash Machine's Quit Action Instead of Exiting Immediately,
/// so There Is No prev-Edge Watcher Here the Way the Cheat Box Watches GodMode --
/// the Menu Selection Itself Is the Trigger
pub fn open_quit_confirm(
    commands: &mut Commands,
    state: &mut QuitConfirmState,
    lock: &mut PlayerControlLock,
    imgs: &splash::SplashImages,
    win_w: f32,
    win_h: f32,
) {
    if state.active {
        return;
    }

    // Choose Uniformly Among the Original Nine Wolf3D endStrings[] Entries
    let idx = rand::rng().random_range(0..QUIT_MESSAGES.len() as u32) as usize;

    state.active = true;
    state.just_opened = true;
    state.input_latched = false;
    lock.0 = true;

    splash::spawn_quit_confirm_ui(commands, imgs, win_w, win_h, QUIT_MESSAGES[idx]);
}

/// Wait for the Y / N Verdict. Y or a Gamepad Confirm Exits the App; N, Escape,
/// a Gamepad Cancel, or Any Other Key Backs Out and Hands Control Back
pub fn resolve_quit_confirm(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    nav: Res<MenuNav>,
    step: Res<SplashStep>,
    mut state: ResMut<QuitConfirmState>,
    mut lock: ResMut<PlayerControlLock>,
    mut app_exit: MessageWriter<bevy::app::AppExit>,
    q_ui: Query<Entity, With<QuitConfirmUi>>,
) {
    if !state.active {
        if state.input_latched {
            let input_still_down = keys.get_pressed().next().is_some()
                || mouse.get_pressed().next().is_some()
                || nav.confirm
                || nav.cancel
                || nav.pause
                || nav.up
                || nav.down
                || nav.left
                || nav.right;

            if !input_still_down {
                state.input_latched = false;
            }
        }

        return;
    }

    // Absorb the Frame the QUIT Row Was Confirmed, so Its Own Enter / South Press
    // Is Not Immediately Re-Read as a Yes
    if state.just_opened {
        state.just_opened = false;
        return;
    }

    // Yes: the Y Key or a Gamepad Confirm (South, Which MenuNav Surfaces Even
    // While Control Is Locked). Enter Deliberately Does NOT Count as Yes -- It Is
    // How the Menu Row Was Selected, and Treating It as Yes Would Feel Like the
    // Prompt Was Skipped. The Player Must Make the Deliberate Y / Confirm Choice
    let want_quit = keys.just_pressed(KeyCode::KeyY) || nav.confirm;

    // No: N, Escape (nav.cancel), a Gamepad Cancel / Start, a Mouse Click, or
    // Any Other Key at All. Matching the Cheat Box, Anything the Player Does That
    // Is Not an Explicit Yes Is Read as Backing Out
    let want_cancel = keys.just_pressed(KeyCode::KeyN)
        || nav.cancel
        || nav.pause
        || mouse.get_just_pressed().next().is_some()
        || keys.get_just_pressed().next().is_some();

    if want_quit {
        app_exit.write(bevy::app::AppExit::Success);
        return;
    }

    if !want_cancel {
        return;
    }

    // Backed Out: Tear the Box Down and Unfreeze
    for e in q_ui.iter() {
        commands.entity(e).try_despawn();
    }
    state.active = false;
    state.input_latched = true;

    // Hand Control Back Only if Nothing Else Claimed the Lock While the Box Was
    // Up. Mirrors the Cheat Box: a Guard for the Future Rather Than a Live Path,
    // Since the Menu Branches Bail Under the Lock and Cannot Take It Themselves.
    // On the Main Menu (SplashStep::Menu) the Lock Was Already Held by the Menu
    // Itself, so We Only Clear It When Gameplay Owns the Screen
    if *step == SplashStep::Done {
        lock.0 = false;
    }
}
