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

use crate::{
    ui::{
        sync::{
            DeathDelay,
            RestartRequested
        }, 
    HudState},
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

// Runs after the respawn chain finishes: unlock + reset “stuff”.
pub fn restart_finish(
    mut restart: ResMut<RestartRequested>,
    mut lock: ResMut<PlayerControlLock>,
    mut latch: ResMut<PlayerDeathLatch>,
    mut death: ResMut<DeathDelay>,
    mut hud: ResMut<HudState>,
) {
    // Keep lives + score; reset everything else to “fresh life”.
    let lives = hud.lives;
    let score = hud.score;

    *hud = HudState::default();
    hud.lives = lives;
    hud.score = score;

    // Clear death/restart bookkeeping.
    *death = Default::default();
    latch.0 = false;
    lock.0 = false;

    // Consume the request so it runs once.
    restart.0 = false;

    bevy::log::info!("Restart: finished (controls unlocked, HUD reset)");
}
