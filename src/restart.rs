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
    restart: Res<RestartRequested>,

    q_meshes: Query<Entity, With<Mesh3d>>,
    q_player: Query<Entity, With<Player>>,
    q_doors: Query<Entity, With<DoorTile>>,
    q_pushwalls: Query<Entity, With<PushwallVisual>>,
    q_lights: Query<Entity, With<PointLight>>,

    // NEW: make sure we never keep old camera/listener around
    q_cameras: Query<Entity, With<Camera>>,
    q_listeners: Query<Entity, With<SpatialListener>>,
) {
    if !restart.0 {
        return;
    }

    let mut kill: HashSet<Entity> = HashSet::new();
    kill.extend(q_meshes.iter());
    kill.extend(q_player.iter());
    kill.extend(q_doors.iter());
    kill.extend(q_pushwalls.iter());
    kill.extend(q_lights.iter());
    kill.extend(q_cameras.iter());
    kill.extend(q_listeners.iter());

    for e in kill {
        commands.entity(e).try_despawn();
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
