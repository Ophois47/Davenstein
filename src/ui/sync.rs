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
use super::{HudState, DeathOverlay, GameOver};

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

/// A request to start a fresh run (reset score/lives/etc.).
#[derive(Resource, Debug, Clone, Default)]
pub struct NewGameRequested(pub bool);

pub fn sync_player_hp_with_hud(
    mut hud: ResMut<HudState>,
    q_player: Query<&davelib::player::PlayerVitals, With<davelib::player::Player>>,
) {
    let Some(vitals) = q_player.iter().next() else { return; };
    hud.hp = vitals.hp;
}

pub fn apply_enemy_fire_to_player_vitals(
    mut q_player: Query<&mut davelib::player::PlayerVitals, With<davelib::player::Player>>,
    lock: Res<PlayerControlLock>,
    latch: Res<PlayerDeathLatch>,
    mut enemy_fire: MessageReader<EnemyFire>,
) {
    // If we're dead (latched) or frozen, ignore further damage.
    if lock.0 || latch.0 {
        // Drain pending shots so they don't apply after we unlock.
        for _ in enemy_fire.read() {}
        return;
    }

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
    mut death_overlay: ResMut<DeathOverlay>,
    mut game_over: ResMut<GameOver>,
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

    // Clear any prior game-over state (if we were resurrected mid-flow).
    game_over.0 = false;
    // Start the death overlay immediately.
    death_overlay.trigger();

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
    mut game_over: ResMut<GameOver>,
    mut death_overlay: ResMut<DeathOverlay>,
) {
    // If we ever become alive again (Step 5 will do this), clear timer/flags.
    let Some(v) = q_vitals.iter().next() else { return; };
    if v.hp > 0 {
        death.active = false;
        let dur = death.timer.duration();
        death.timer.set_elapsed(dur);
        restart.0 = false;
        game_over.0 = false;
        death_overlay.clear();
        return;
    }

    // Only run the delay when death has been latched and input is locked.
    if !latch.0 || !lock.0 {
        return;
    }

    // If we already requested a restart (or game over), nothing to do.
    if restart.0 || game_over.0 {
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
        game_over.0 = true;
        info!("Death delay finished -> GAME OVER (no lives remaining)");
        // stay locked; game_over_input handles Enter->NewGameRequested
    }
}

/// While in Game Over, wait for player input to start a new run.
pub fn game_over_input(
    keys: Res<ButtonInput<KeyCode>>,
    game_over: Res<GameOver>,
    mut new_game: ResMut<NewGameRequested>,
) {
    if !game_over.0 || new_game.0 {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) {
        new_game.0 = true;
        info!("Game Over: Enter pressed -> new game requested");
    }
}
