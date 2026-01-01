/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use std::collections::HashSet;

use davelib::{
    map::DoorTile,
    player::{
        Player,
        PlayerControlLock,
        PlayerDeathLatch,
    },
    pushwalls::PushwallVisual,
};
use crate::level_complete::LevelComplete;
use crate::{
    ui::{
        sync::{
            DeathDelay,
            RestartRequested,
            NewGameRequested
        }, 
        DeathOverlay,
        GameOver,
        HudState,
    },
};

// Despawn what should NOT persist across a life restart.
// Goal: leave UI/resources alone, rebuild the entire 3D world + actors.
pub fn restart_despawn_level(
    mut commands: Commands,
    q_mesh_roots: Query<Entity, (With<Mesh3d>, Without<ChildOf>)>,
    q_player: Query<Entity, With<Player>>,
    q_doors: Query<Entity, (With<DoorTile>, Without<ChildOf>)>,
    q_pushwalls: Query<Entity, (With<PushwallVisual>, Without<ChildOf>)>,
    q_lights: Query<Entity, With<PointLight>>,
) {
    let mut kill: HashSet<Entity> = HashSet::new();

    kill.extend(q_mesh_roots.iter());
    kill.extend(q_player.iter());
    kill.extend(q_doors.iter());
    kill.extend(q_pushwalls.iter());
    kill.extend(q_lights.iter());

    for e in kill {
        commands.entity(e).despawn();
    }
}

pub fn restart_finish(
    mut restart: ResMut<RestartRequested>,
    mut lock: ResMut<PlayerControlLock>,
    mut latch: ResMut<PlayerDeathLatch>,
    mut death: ResMut<DeathDelay>,
    mut hud: ResMut<HudState>,
    mut win: ResMut<LevelComplete>,
) {
    // Keep lives + score; reset everything else to “fresh life”.
    let lives = hud.lives;
    let score = hud.score;

    *hud = HudState::default();
    hud.lives = lives;
    hud.score = score;

    // Clear death/restart bookkeeping + win state.
    *death = Default::default();
    latch.0 = false;
    lock.0 = false;
    win.0 = false;

    // Consume the request so it runs once.
    restart.0 = false;

    bevy::log::info!("Restart: finished (controls unlocked, HUD reset)");
}

pub fn new_game_finish(
    mut new_game: ResMut<NewGameRequested>,
    mut lock: ResMut<PlayerControlLock>,
    mut latch: ResMut<PlayerDeathLatch>,
    mut death: ResMut<DeathDelay>,
    mut hud: ResMut<HudState>,
    mut game_over: ResMut<GameOver>,
    mut death_overlay: ResMut<DeathOverlay>,
    mut win: ResMut<LevelComplete>,
) {
    if !new_game.0 {
        return;
    }

    *hud = HudState::default();

    // Clear death/restart bookkeeping + win state.
    *death = Default::default();
    latch.0 = false;
    lock.0 = false;
    game_over.0 = false;
    win.0 = false;
    *death_overlay = DeathOverlay::default();

    // Consume the request so it runs once.
    new_game.0 = false;

    bevy::log::info!("New Game: finished (fresh HUD, controls unlocked)");
}

pub fn advance_level_finish(
    mut advance: ResMut<crate::ui::sync::AdvanceLevelRequested>,
    mut lock: ResMut<davelib::player::PlayerControlLock>,
    mut latch: ResMut<davelib::player::PlayerDeathLatch>,
    mut death: ResMut<crate::ui::sync::DeathDelay>,
    mut hud: ResMut<crate::ui::HudState>,
    mut win: ResMut<crate::level_complete::LevelComplete>,
    mut q_vitals: Query<&mut davelib::player::PlayerVitals, With<davelib::player::Player>>,
) {
    // Preserve run stats (ammo/score/lives/weapons) by NOT resetting HudState.
    // Wolf behavior: keys do not carry across levels.
    hud.key_gold = false;
    hud.key_silver = false;

    // setup() spawns PlayerVitals::default(); restore HP from HUD so it carries over.
    if let Some(mut vitals) = q_vitals.iter_mut().next() {
        vitals.hp = hud.hp.clamp(0, vitals.hp_max);
    }

    // Clear mission-success state and unlock gameplay.
    win.0 = false;
    lock.0 = false;

    // Clear death flow bookkeeping (safe, even if we weren't dying).
    *death = Default::default();
    latch.0 = false;

    // Consume the request.
    advance.0 = false;

    bevy::log::info!("Advance Level: finished (HUD preserved, controls unlocked)");
}
