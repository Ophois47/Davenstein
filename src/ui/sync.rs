/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

use davelib::ai::EnemyFire;
use davelib::player::{
    Player,
    PlayerControlLock,
    PlayerDeathLatch,
    PlayerVitals,
};
use super::HudState;

pub fn sync_player_hp_with_hud(
    mut hud: ResMut<HudState>,
    q_player: Query<&davelib::player::PlayerVitals, With<davelib::player::Player>>,
) {
    let Some(vitals) = q_player.iter().next() else { return; };
    hud.hp = vitals.hp;
}

pub fn apply_enemy_fire_to_player_vitals(
    mut q_player: Query<&mut davelib::player::PlayerVitals, With<davelib::player::Player>>,
    mut enemy_fire: MessageReader<EnemyFire>,
) {
    let Some(mut vitals) = q_player.iter_mut().next() else { return; };

    for ev in enemy_fire.read() {
        // damage == 0 means miss
        if ev.damage <= 0 {
            info!("Enemy missed (damage=0)");
            continue;
        }

        let before = vitals.hp;
        vitals.hp = (vitals.hp - ev.damage).max(0);

        info!(
            "Enemy hit for {} -> hp {} -> {}",
            ev.damage, before, vitals.hp
        );
    }
}

pub fn handle_player_death_once(
    q_vitals: Query<&PlayerVitals, With<Player>>,
    mut hud: ResMut<HudState>,
    mut lock: ResMut<PlayerControlLock>,
    mut latch: ResMut<PlayerDeathLatch>,
) {
    let Some(v) = q_vitals.iter().next() else {
        return;
    };

    // If we're alive, clear the latch + unlock (for later respawn/restart flow).
    if v.hp > 0 {
        latch.0 = false;
        // Do NOT auto-unlock here yet; restart system will control lock state.
        return;
    }

    // HP <= 0: process death exactly once.
    if latch.0 {
        return;
    }
    latch.0 = true;

    if hud.lives > 0 {
        hud.lives -= 1;
    }

    // Freeze player input as the immediate “death” effect.
    lock.0 = true;
}
