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
use crate::ui::DeathOverlay;
use super::{
    HudState,
    GameOver,
};

#[derive(Resource, Debug, Clone)]
pub struct DeathDelay {
    pub active: bool,
    pub timer: Timer,
}

impl Default for DeathDelay {
    fn default() -> Self {
        let mut t = Timer::from_seconds(1.25, TimerMode::Once);
        // Start Finished so it Does Nothing Until Activated
        t.set_elapsed(t.duration());
        Self { active: false, timer: t }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct RestartRequested(pub bool);

/// A Request to Start Fresh Run (Reset Score / Lives / etc)
#[derive(Resource, Debug, Clone, Default)]
pub struct NewGameRequested(pub bool);

/// Request to Advance to Next Level 
/// Preserving Run Stats (Ammo / Weapons / Score / Lives / HP)
#[derive(Resource, Debug, Clone, Default)]
pub struct AdvanceLevelRequested(pub bool);

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
    god: Res<davelib::player::GodMode>,
    mut enemy_fire: MessageReader<EnemyFire>,
) {
    // God Mode: Ignore Damage (But Drain Events)
    if god.0 {
        for _ in enemy_fire.read() {}
        return;
    }

    // If Dead (Latched) or Frozen, Ignore Further Damage
    if lock.0 || latch.0 {
        // Drain Pending Shots so They Don't Apply After Unlock
        for _ in enemy_fire.read() {}
        return;
    }

    let Some(mut vitals) = q_player.iter_mut().next() else { return; };

    for ev in enemy_fire.read() {
        // Damage == 0 Means Miss
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
    let Some(v) = q_vitals.iter().next() else { return; };

    if v.hp > 0 {
        latch.0 = false;
        return;
    }

    if latch.0 { return; }
    latch.0 = true;

    game_over.0 = false;

    death_overlay.active = true;
    death_overlay.timer.reset();

    if hud.lives > 0 { hud.lives -= 1; }
    lock.0 = true;
}

pub fn tick_death_delay_and_request_restart(
    mut commands: Commands,
    time: Res<Time>,
    q_vitals: Query<&PlayerVitals, With<Player>>,
    hud: Res<HudState>,
    current_level: Res<davelib::level::CurrentLevel>,
    lock: Res<PlayerControlLock>,
    latch: Res<PlayerDeathLatch>,
    mut death: ResMut<DeathDelay>,
    mut restart: ResMut<RestartRequested>,
    mut game_over: ResMut<GameOver>,
    mut death_overlay: ResMut<DeathOverlay>,
) {
    let Some(v) = q_vitals.iter().next() else { return; };

    if v.hp > 0 {
        death.active = false;
        let dur = death.timer.duration();
        death.timer.set_elapsed(dur);
        restart.0 = false;
        game_over.0 = false;

        death_overlay.active = false;
        let dur = death_overlay.timer.duration();
        death_overlay.timer.set_elapsed(dur);

        return;
    }

    if !latch.0 || !lock.0 { return; }
    if restart.0 || game_over.0 { return; }

    if !death.active {
        death.active = true;
        death.timer.reset();
    }

    death.timer.tick(time.delta());
    if !death.timer.is_finished() { return; }

    death.active = false;

    if hud.lives > 0 {
        restart.0 = true;
    } else {
        // Check for High Score Before Showing Game Over
        commands.insert_resource(davelib::high_score::CheckHighScore {
            score: hud.score,
            episode: current_level.0.episode(),
            checked: false,
        });
        game_over.0 = true;
    }
}

// In Game Over, Wait for Player Input
// Then Check if Score Qualifies for High Scores
pub fn game_over_input(
    keys: Res<ButtonInput<KeyCode>>,
    game_over: Res<GameOver>,
    hud: Res<HudState>,
    current_level: Res<davelib::level::CurrentLevel>,
    high_scores: Res<davelib::high_score::HighScores>,
    mut new_game: ResMut<NewGameRequested>,
    mut splash_step: ResMut<crate::ui::SplashStep>,
    mut name_entry: ResMut<davelib::high_score::NameEntryState>,
) {
    if !game_over.0 || new_game.0 {
        return;
    }

    // Do Not Allow Enter to Re-Arm Name Entry While in Menu / Scores UI
    if *splash_step != crate::ui::SplashStep::Done {
        return;
    }

    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    // Check if Score Qualifies for High Scores
    if high_scores.qualifies(hud.score) {
        // Find Rank
        let rank = high_scores
            .entries
            .iter()
            .position(|e| hud.score > e.score)
            .unwrap_or(high_scores.entries.len());

        // Activate Name Entry
        name_entry.active = true;
        name_entry.name.clear();
        name_entry.cursor_pos = 0;
        name_entry.rank = rank;
        name_entry.score = hud.score;
        name_entry.episode = current_level.0.episode();

        *splash_step = crate::ui::SplashStep::NameEntry;
    } else {
        // Score Doesn't Qualify, Straight to Menu
        new_game.0 = true;
        *splash_step = crate::ui::SplashStep::Menu;
    }
}
