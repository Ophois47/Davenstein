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
    pushwalls::{
        PushwallVisual,
        PushwallState,
        PushwallOcc,
        PushwallClock,
    },
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

// Despawn what should NOT persist across a life restart
// Leave UI / resources alone, rebuild entire 3D world + actors
pub fn restart_despawn_level(
    mut commands: Commands,
    q_mesh_roots: Query<Entity, (With<Mesh3d>, Without<ChildOf>)>,
    q_player: Query<Entity, With<Player>>,
    q_doors: Query<Entity, (With<DoorTile>, Without<ChildOf>)>,
    q_pushwalls: Query<Entity, (With<PushwallVisual>, Without<ChildOf>)>,
    q_lights: Query<Entity, With<PointLight>>,
    q_children: Query<&Children>,
) {
    fn despawn_tree(commands: &mut Commands, q_children: &Query<&Children>, e: Entity) {
        if let Ok(children) = q_children.get(e) {
            // Children::iter() yields Entity in this Bevy version
            let kids: Vec<Entity> = children.iter().collect();
            for child in kids {
                despawn_tree(commands, q_children, child);
            }
        }
        commands.entity(e).try_despawn();
    }

    let mut kill: HashSet<Entity> = HashSet::new();

    kill.extend(q_mesh_roots.iter());
    kill.extend(q_player.iter());
    kill.extend(q_doors.iter());
    kill.extend(q_pushwalls.iter());
    kill.extend(q_lights.iter());

    for e in kill {
        despawn_tree(&mut commands, &q_children, e);
    }
}

pub fn restart_finish(
    mut restart: ResMut<RestartRequested>,
    mut lock: ResMut<PlayerControlLock>,
    mut latch: ResMut<PlayerDeathLatch>,
    mut death: ResMut<DeathDelay>,
    mut hud: ResMut<HudState>,
    mut win: ResMut<LevelComplete>,
    mut pw_state: ResMut<PushwallState>,
    mut pw_occ: ResMut<PushwallOcc>,
    mut pw_clock: ResMut<PushwallClock>,
) {
    // Keep lives + score, reset everything else
    let lives = hud.lives;
    let score = hud.score;

    *hud = HudState::default();
    hud.lives = lives;
    hud.score = score;

    // Clear death / restart bookkeeping + win state
    *death = Default::default();
    latch.0 = false;
    lock.0 = false;
    win.0 = false;

    pw_state.active = None;
    pw_occ.clear();
    pw_clock.reset();

    // Consume request so it runs once
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
    mut pw_state: ResMut<PushwallState>,
    mut pw_occ: ResMut<PushwallOcc>,
    mut pw_clock: ResMut<PushwallClock>,
) {
    if !new_game.0 {
        return;
    }

    *hud = HudState::default();

    *death = Default::default();
    latch.0 = false;
    lock.0 = false;
    game_over.0 = false;
    win.0 = false;
    *death_overlay = DeathOverlay::default();

    pw_state.active = None;
    pw_occ.clear();
    pw_clock.reset();
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
    mut pw_state: ResMut<davelib::pushwalls::PushwallState>,
    mut pw_occ: ResMut<davelib::pushwalls::PushwallOcc>,
    mut pw_clock: ResMut<davelib::pushwalls::PushwallClock>,
    mut q_vitals: Query<&mut davelib::player::PlayerVitals, With<davelib::player::Player>>,
    mut q_keys: Query<&mut davelib::player::PlayerKeys, With<davelib::player::Player>>,
) {
    // Preserve Run Stats (Ammo / Score / Lives / Weapons) by NOT Resetting
    // HudState but Keys Do Not Carry Across Levels
    hud.key_gold = false;
    hud.key_silver = false;

    if let Some(mut pkeys) = q_keys.iter_mut().next() {
        pkeys.gold = false;
        pkeys.silver = false;
    }

    // Restore HP From HUD so it Carries Over, setup() Spawns PlayerVitals::default()
    if let Some(mut vitals) = q_vitals.iter_mut().next() {
        vitals.hp = hud.hp.clamp(0, vitals.hp_max);
    }

    // Clear Mission-Success State and Unlock Gameplay
    win.0 = false;
    lock.0 = false;

    // Clear Death Flow Bookkeeping (Safe, Even if Not Dying)
    *death = Default::default();
    latch.0 = false;

    pw_state.active = None;
    pw_occ.clear();
    pw_clock.reset();

    // Consume Request
    advance.0 = false;

    bevy::log::info!("Advance Level: finished (HUD preserved, controls unlocked)");
}
