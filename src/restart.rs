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

// Commands are Deferred. Resources / Entities Inserted via Commands During Level
// Setup or Restart are Not Available to Later Systems Until After apply_deferred
// Systems that Read Level Resources Must Either:
// (1) Run Only After apply_deferred Boundary, and / or
// (2) use Option<Res<T>> and Early Return, and / or
// (3) be Gated by WorldReady Run Condition
// Missing-resource Panics Treated as Regressions. Add Gating Before Adding New Res Dependencies

// Despawn What Should NOT Persist Across Life Restart
// Leave UI / Resources Alone, Rebuild Entire 3D World + Actors
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
            // Children::iter() Yields Entity in Bevy
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
    // Keep Lives + Score, Reset Everything Else
    let lives = hud.lives;
    let score = hud.score;

    *hud = HudState::default();
    hud.lives = lives;
    hud.score = score;

    // Clear Death / Restart Nookkeeping + Win State
    *death = Default::default();
    latch.0 = false;
    lock.0 = false;
    win.0 = false;

    pw_state.active = None;
    pw_occ.clear();
    pw_clock.reset();

    // Consume Request so it Runs Once
    restart.0 = false;
}

pub fn new_game_finish(
    mut new_game: ResMut<NewGameRequested>,
    mut lock: ResMut<PlayerControlLock>,
    mut latch: ResMut<PlayerDeathLatch>,
    mut death: ResMut<DeathDelay>,
    mut hud: ResMut<HudState>,
    mut episode_stats: ResMut<davelib::level_score::EpisodeStats>,
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
    *episode_stats = davelib::level_score::EpisodeStats::default();

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
}
