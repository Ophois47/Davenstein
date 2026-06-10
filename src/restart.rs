/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use std::collections::HashSet;

use davelib::{
    map::DoorTile,
    player::{
        LookAngles,
        Player,
        PlayerControlLock,
        PlayerDeathLatch,
        PlayerKeys,
        PlayerVitals,
    },
    pushwalls::{
        PushwallVisual,
        PushwallState,
        PushwallOcc,
        PushwallClock,
    },
};
use crate::level_complete::{
    ElevatorExitDelay,
    LevelComplete,
    MissionSuccessTally,
    PendingLevelExit,
};
use crate::{
    ui::{
        sync::{
            AdvanceLevelRequested,
            DeathDelay,
            NewGameRequested,
            RestartRequested,
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

#[derive(SystemParam)]
pub struct LoadRequestParams<'w> {
    load: ResMut<'w, crate::save::LoadGameRequested>,
    pending_dead: ResMut<'w, crate::save::PendingDeadRestore>,
    pending_pickups: ResMut<'w, crate::save::PendingPickupRestore>,
    pending_doors: ResMut<'w, crate::save::PendingDoorRestore>,
    pending_pushwalls: ResMut<'w, crate::save::PendingPushwallRestore>,
    restart: ResMut<'w, RestartRequested>,
    new_game: ResMut<'w, NewGameRequested>,
    advance: ResMut<'w, AdvanceLevelRequested>,
}

#[derive(SystemParam)]
pub struct LoadRuntimeParams<'w> {
    lock: ResMut<'w, PlayerControlLock>,
    latch: ResMut<'w, PlayerDeathLatch>,
    death: ResMut<'w, DeathDelay>,
    hud: ResMut<'w, HudState>,
    game_over: ResMut<'w, GameOver>,
    death_overlay: ResMut<'w, DeathOverlay>,
    win: ResMut<'w, LevelComplete>,
    tally: ResMut<'w, MissionSuccessTally>,
    elevator_delay: ResMut<'w, ElevatorExitDelay>,
    pending_exit: ResMut<'w, PendingLevelExit>,
    level_score: ResMut<'w, davelib::level_score::LevelScore>,
    pw_state: ResMut<'w, PushwallState>,
    pw_occ: ResMut<'w, PushwallOcc>,
    pw_clock: ResMut<'w, PushwallClock>,
}

#[derive(SystemParam)]
pub struct LoadPlayerParams<'w, 's> {
    player: Query<
        'w,
        's,
        (
            &'static mut Transform,
            &'static mut PlayerVitals,
            &'static mut PlayerKeys,
            &'static mut LookAngles,
        ),
        With<Player>,
    >,
}

pub fn load_game_finish(
    mut req: LoadRequestParams,
    mut state: LoadRuntimeParams,
    mut q_player: LoadPlayerParams,
) {
    let Some(game) = req.load.0.take() else {
        return;
    };

    crate::save::capture::apply_run_state(&mut *state.hud, &game.run_state);
    crate::save::capture::apply_level_score(&mut *state.level_score, &game.level_score);

    // Stash the dead-enemy set so apply_pending_dead_restore can mark them as
    // corpses once the rebuilt level's enemies exist (a frame or two later).
    req.pending_dead.0 = match &game.world {
        Some(w) => w.dead_enemies.clone(),
        None => Vec::new(),
    };

    // Stash the Present-Pickup Set so apply_pending_pickup_restore Can Despawn
    // Already-Collected Pickups Once the Rebuilt Level's Pickups Exist
    // active Is Set Whenever a World Snapshot Is Present, Because an Empty Set
    // Validly Means "Everything Was Collected"
    match &game.world {
        Some(w) => {
            req.pending_pickups.active = true;
            req.pending_pickups.present_tiles = w.present_pickups.clone();
        }
        None => {
            req.pending_pickups.active = false;
            req.pending_pickups.present_tiles.clear();
        }
    }

    // Stash the Open-Door Set so apply_pending_door_restore Can Re-Open Them
    // Once the Rebuilt Level's Doors Exist
    match &game.world {
        Some(w) => {
            req.pending_doors.active = true;
            req.pending_doors.open_tiles = w.open_doors.clone();
        }
        None => {
            req.pending_doors.active = false;
            req.pending_doors.open_tiles.clear();
        }
    }

    // Stash Completed Pushwalls so apply_pending_pushwall_restore Can Re-Apply
    // Their Grid Effect Once the Rebuilt Level's Grid Exists
    match &game.world {
        Some(w) => {
            req.pending_pushwalls.active = true;
            req.pending_pushwalls.frames_waited = 0;
            req.pending_pushwalls.items = w.pushwalls.clone();
        }
        None => {
            req.pending_pushwalls.active = false;
            req.pending_pushwalls.frames_waited = 0;
            req.pending_pushwalls.items.clear();
        }
    }

    let Some((mut tf, mut vitals, mut keys, mut look)) = q_player.player.iter_mut().next() else {
        error!("Load requested but no player entity exists after level rebuild");

        req.restart.0 = false;
        req.new_game.0 = false;
        req.advance.0 = false;

        return;
    };

    let (yaw, pitch) = crate::save::capture::apply_player(
        &mut *tf,
        &mut *vitals,
        &mut *keys,
        &game.player,
        &game.run_state,
    );

    *look = LookAngles::new(yaw, pitch);

    state.hud.hp = vitals.hp;

    *state.death = DeathDelay::default();
    *state.death_overlay = DeathOverlay::default();
    *state.tally = MissionSuccessTally::default();
    *state.elevator_delay = ElevatorExitDelay::default();
    *state.pending_exit = PendingLevelExit::default();

    state.game_over.0 = false;
    state.win.0 = false;
    state.latch.0 = false;
    state.lock.0 = false;

    state.pw_state.active = None;
    state.pw_occ.clear();
    state.pw_clock.reset();

    req.restart.0 = false;
    req.new_game.0 = false;
    req.advance.0 = false;
}
