/*
Davenstein - by David Petnick

The Original's Cheat Warning Box, Restored as a Modal Overlay.

When the MIL Chord Turns God Mode On, WL_INLOOP's Message() Drew a Grey Box Over the
Frozen Frame and IN_Ack() Held the Whole Game Until Any Key or Button Was Pressed.
This Module Reproduces That: the Rising Edge of GodMode Spawns the Box and Takes
PlayerControlLock (Which is What Freezes the World -- Every Gameplay System is
Already Gated on it), and Any Fresh Input Releases Both.

Two Edges Needed Deliberate Handling:

- The MIL Chord Itself is Three just_pressed Keys, so a Naive "Dismiss on Any Key"
  Would Close the Box on the Very Frame That Opened it. A One-Frame Guard Absorbs
  the Opening Chord.
- The Splash Machine's Gameplay Branch Bails While the Lock is Held, so Escape and
  the Gamepad Start Button Cannot Open the Pause Menu Over This Box -- They Simply
  Dismiss it Like Any Other Key. The SplashStep Check Before Releasing the Lock is
  Kept Anyway as a Cheap Guard: Should That Bail Ever Be Removed, a Press That Also
  Opened a Menu Would Still Hand the Lock Over Cleanly Instead of Yanking it Back
*/

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use davelib::input::MenuNav;
use davelib::player::{GodMode, PlayerControlLock};

use super::splash::{self, SplashStep};

/// Marker on the Overlay Root so Dismissal Can Find and Despawn the Whole Box
#[derive(Component)]
pub struct CheatMessageUi;

/// Lifecycle State. prev_god Detects the Off-to-On Edge; just_opened Absorbs the
/// MIL Chord's Own Keypresses so They Cannot Dismiss the Box They Summoned
#[derive(Resource, Default)]
pub struct CheatMessageState {
    active: bool,
    just_opened: bool,
    prev_god: bool,
}

/// What the Box Says. The Original's Text Also Announced 100% Health, 99 Ammo, and
/// Both Keys -- Grants This Port's God Mode Does Not (Yet) Make, so the Text Only
/// Claims What is True. When the Full MLI Grant is Added, This String is the Only
/// Place the Wording Changes
const CHEAT_MESSAGE_TEXT: &str =
    "God Mode Enabled!\n\nNote that you have basically\neliminated your chances of\ngetting a high score!";

/// Watch for God Mode Turning On and Raise the Modal. The Edge Can Only Occur During
/// Live Gameplay Because toggle_god_mode is Gated on the Lock Being Free, Which Also
/// Means Taking the Lock Here Can Never Steal it From a Menu
pub fn trigger_cheat_message(
    mut commands: Commands,
    god: Res<GodMode>,
    mut state: ResMut<CheatMessageState>,
    mut lock: ResMut<PlayerControlLock>,
    imgs: Option<Res<splash::SplashImages>>,
    q_win: Query<&Window, With<PrimaryWindow>>,
) {
    let rising = god.0 && !state.prev_god;
    state.prev_god = god.0;

    if !rising || state.active || lock.0 {
        return;
    }

    // Menu Images Load at Startup; if They Are Somehow Absent, Skip the Box Rather
    // Than Take a Lock Nothing Could Ever Release
    let Some(imgs) = imgs else { return; };
    let Some(win) = q_win.iter().next() else { return; };

    state.active = true;
    state.just_opened = true;
    lock.0 = true;

    splash::spawn_cheat_message_ui(
        &mut commands,
        &imgs,
        win.width(),
        win.height(),
        CHEAT_MESSAGE_TEXT,
    );
}

/// Hold Until Any Fresh Input, Then Release -- the IN_Ack() Half
pub fn dismiss_cheat_message(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    nav: Res<MenuNav>,
    step: Res<SplashStep>,
    mut state: ResMut<CheatMessageState>,
    mut lock: ResMut<PlayerControlLock>,
    q_ui: Query<Entity, With<CheatMessageUi>>,
) {
    if !state.active {
        return;
    }

    // Absorb the Frame the MIL Chord Opened the Box, or its Own Keys Dismiss it
    if state.just_opened {
        state.just_opened = false;
        return;
    }

    // Any Key, Any Mouse Button, or Any Gamepad Face / Start Press (Which Arrive
    // Through MenuNav Precisely so They Keep Working While Control is Locked)
    let any_input = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || nav.confirm
        || nav.cancel
        || nav.pause;

    if !any_input {
        return;
    }

    for e in q_ui.iter() {
        commands.entity(e).try_despawn();
    }
    state.active = false;

    // Only Hand Control Back to Gameplay When Nothing Else Claimed the Lock While
    // the Box Was Up. Today Nothing Can (the Splash Gameplay Branch Bails Under the
    // Lock), so This is a Guard for the Future Rather Than a Live Code Path
    if *step == SplashStep::Done {
        lock.0 = false;
    }
}
