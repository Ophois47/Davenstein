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

#[derive(Resource, Debug, Clone)]
pub struct DeathDelay {
    pub active: bool,
    pub timer: Timer,
}

impl Default for DeathDelay {
    fn default() -> Self {
        let mut t = Timer::from_seconds(1.25, TimerMode::Once);
        // Start finished so it does nothing until activated
        t.set_elapsed(t.duration());
        Self { active: false, timer: t }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct RestartRequested(pub bool);

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

pub fn tick_death_delay_and_request_restart(
    time: Res<Time>,
    q_vitals: Query<&davelib::player::PlayerVitals, With<davelib::player::Player>>,
    hud: Res<super::HudState>,
    lock: Res<davelib::player::PlayerControlLock>,
    latch: Res<davelib::player::PlayerDeathLatch>,
    mut death: ResMut<DeathDelay>,
    mut restart: ResMut<RestartRequested>,
) {
    // If we ever become alive again (Step 5 will do this), clear timer/flags.
    let Some(v) = q_vitals.iter().next() else { return; };
    if v.hp > 0 {
        death.active = false;
        let dur = death.timer.duration();
        death.timer.set_elapsed(dur);
        restart.0 = false;
        return;
    }

    // Only run the delay when death has been latched and input is locked.
    if !latch.0 || !lock.0 {
        return;
    }

    // If we already requested a restart (or game over), nothing to do.
    if restart.0 {
        return;
    }

    // Start the timer once.
    if !death.active {
        death.active = true;
        death.timer.reset();
    }

    death.timer.tick(time.delta());
    if !death.timer.is_finished() {
        return;
    }

    death.active = false;

    if hud.lives > 0 {
        restart.0 = true;
        info!("Death delay finished -> restart requested (lives remaining: {})", hud.lives);
    } else {
        info!("Death delay finished -> GAME OVER (no lives remaining)");
        // stay locked; Step 5+ will add a proper game over UI
    }
}
