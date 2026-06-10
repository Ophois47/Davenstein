/*
Davenstein - by David Petnick

Save Module Wiring
- model   : Serializable Snapshot Structs (Version + Option<World>)
- storage : The Only Filesystem Seam (Swap For WASM Later)
- capture : Live ECS <-> Save Model Translation

Save Requests Are Consumed Here
Load Restore Reuses the Existing Level-Rebuild Pipeline via LoadGameRequested
restart::load_game_finish Applies Run State After the Rebuilt World Exists
*/

pub mod model;
pub mod storage;
pub mod capture;

use bevy::prelude::*;

use davelib::level::CurrentLevel;
use davelib::level_score::LevelScore;
use davelib::player::{Player, PlayerVitals};
use crate::ui::HudState;

/// Fire-And-Forget Request to Save Into a Slot
/// Set by Menu, Consumed by handle_save_requests
/// None = No Pending Save
#[derive(Resource, Default)]
pub struct SaveGameRequested(pub Option<u32>);

#[derive(Resource, Default)]
pub struct LoadGameRequested(pub Option<model::SaveGame>);

/// Dead Enemies From a Just-Loaded Save Waiting to be Applied as Corpses
/// Rebuilt Level Enemies Spawn via Deferred Commands, so They are Not Queryable
/// Until a Later Frame After load_game_finish
/// apply_pending_dead_restore Consumes This
#[derive(Resource, Default)]
pub struct PendingDeadRestore(pub Vec<model::DeadEnemy>);

// Pickup Tiles That Should Remain (Un-Collected) From a Just-Loaded Save
// apply_pending_pickup_restore Despawns Fresh Pickups Not in This Set
// The bool Tracks Whether a Restore Is Pending (Empty Vec Is Valid: It Means
// Everything Was Collected, so All Fresh Pickups Should Despawn)
#[derive(Resource, Default)]
pub struct PendingPickupRestore {
    pub active: bool,
    pub present_tiles: Vec<[i32; 2]>,
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveGameRequested>()
            .init_resource::<LoadGameRequested>()
            .init_resource::<PendingDeadRestore>()
            .init_resource::<PendingPickupRestore>()
            .add_systems(Update, handle_save_requests)
            .add_systems(Update, apply_pending_dead_restore)
            .add_systems(Update, apply_pending_pickup_restore);
    }
}

/// When SaveGameRequested Holds a Slot, Capture Current State and Write It
fn handle_save_requests(
    mut req: ResMut<SaveGameRequested>,
    hud: Res<HudState>,
    current_level: Res<CurrentLevel>,
    level_score: Res<LevelScore>,
    q_player: Query<(&Transform, &PlayerVitals), With<Player>>,
    q_dead: Query<
        (&davelib::enemies::EnemyKind, &davelib::enemies::SpawnIndex),
        With<davelib::actors::Dead>,
    >,
    q_pickups: Query<&crate::pickups::Pickup>,
) {
    let Some(slot) = req.0 else { return; };

    let Ok((player_tf, vitals)) = q_player.single() else {
        // No Player Yet (Ex: In a Menu) - Clear Request and Do Nothing
        req.0 = None;
        return;
    };

    // Auto-Name From Current Level For Now (Ex: "E1M3")
    // Slice B Replaces This With a Player-Typed Name via Save Name-Entry Screen
    let lr = capture::level_to_ref(current_level.0);
    let name = format!("E{}M{}", lr.episode, lr.floor);

    // Collect Dead Enemies by Kind + Stable Spawn Index
    // Load Restores Them as Corpses Instead of Respawning Them Alive
    let dead: Vec<(davelib::enemies::EnemyKind, davelib::enemies::SpawnIndex)> =
        q_dead.iter().map(|(k, i)| (*k, *i)).collect();
    let dead_enemies = capture::capture_dead_enemies(&dead);

    // Tiles That Still Hold a Pickup Are the Un-Collected Ones (Collecting
    // Despawns the Entity), so on Load We Despawn Any Pickup Not in This Set
    let present_pickups: Vec<[i32; 2]> =
        q_pickups.iter().map(|p| [p.tile.x, p.tile.y]).collect();

    let game = capture::capture_save_game(name, &hud, player_tf, vitals, &current_level, &level_score, dead_enemies, present_pickups);

    match storage::write_slot(slot, &game) {
        Ok(()) => info!("Saved game to slot {slot}"),
        Err(e) => error!("Save to slot {slot} failed: {e:?}"),
    }

    req.0 = None;
}

/// Applies the Dead-Enemy Set From a Just-Loaded Save
/// Matching Enemies are Marked as Corpses After Rebuild Spawns Exist
/// Runs in Update Because Enemy Spawns are Deferred via Commands
/// Clears the Pending Set After One Successful Apply Pass
fn apply_pending_dead_restore(
    mut commands: Commands,
    mut pending: ResMut<PendingDeadRestore>,
    q_enemies: Query<(
        Entity,
        &davelib::enemies::EnemyKind,
        &davelib::enemies::SpawnIndex,
    )>,
) {
    if pending.0.is_empty() {
        return;
    }

    // Wait Until Rebuilt Level Enemies Have Actually Spawned
    // Empty Query Means Deferred Spawns Have Not Applied Yet
    if q_enemies.is_empty() {
        return;
    }

    // Build a Fast Lookup of Which (kind_u8, index) Pairs Should be Dead
    let dead_set: std::collections::HashSet<(u8, u32)> =
        pending.0.iter().map(|d| (d.kind, d.index)).collect();

    let mut applied = 0usize;
    for (e, kind, idx) in q_enemies.iter() {
        let key = (capture::enemy_kind_to_u8(*kind), idx.0);
        if dead_set.contains(&key) {
            make_corpse(&mut commands, e, *kind);
            applied += 1;
        }
    }

    info!(
        "Restored {} dead enemies as corpses on load ({} requested)",
        applied,
        pending.0.len()
    );

    // Consume Pending Set so This Only Runs Once Per Load
    pending.0.clear();
}

/// Put Enemy Entity Into Corpse State
/// Dead Makes AI Systems Ignore It, EnemyAi Removal Prevents Acting
/// Per-Type Corpse Markers Let Existing Added<TypeCorpse> Systems Render Sprites
fn make_corpse(
    commands: &mut Commands,
    e: Entity,
    kind: davelib::enemies::EnemyKind,
) {
    use davelib::enemies::*;

    let mut ec = commands.entity(e);
    ec.insert(davelib::actors::Dead);
    ec.remove::<davelib::ai::EnemyAi>();

    match kind {
        EnemyKind::Guard => { ec.insert(GuardCorpse); }
        EnemyKind::Ss => { ec.insert(SsCorpse); }
        EnemyKind::Officer => { ec.insert(OfficerCorpse); }
        EnemyKind::Mutant => { ec.insert(MutantCorpse); }
        EnemyKind::Dog => { ec.insert(DogCorpse); }
        EnemyKind::Hans => { ec.insert(HansCorpse); }
        EnemyKind::Gretel => { ec.insert(GretelCorpse); }
        EnemyKind::Hitler => { ec.insert(HitlerCorpse); }
        EnemyKind::MechaHitler => { ec.insert(MechaHitlerCorpse); }
        EnemyKind::GhostHitler => { ec.insert(GhostHitlerCorpse); }
        EnemyKind::Schabbs => { ec.insert(SchabbsCorpse); }
        EnemyKind::Otto => { ec.insert(OttoCorpse); }
        EnemyKind::General => { ec.insert(GeneralCorpse); }
    }
}

/// Despawns Pickups That Were Already Collected in a Just-Loaded Save
/// Runs in Update (Not load_game_finish) Because Pickups Spawn via Deferred
/// Commands and Are Not Queryable Until a Later Frame
/// Waits Until Pickups Exist, Applies Once, Then Clears the Pending Flag
fn apply_pending_pickup_restore(
    mut commands: Commands,
    mut pending: ResMut<PendingPickupRestore>,
    q_pickups: Query<(Entity, &crate::pickups::Pickup)>,
) {
    if !pending.active {
        return;
    }

    // Wait Until the Rebuilt Level's Pickups Have Actually Spawned
    // If the Query Is Empty We Are Too Early (Deferred Spawns Not Applied Yet)
    if q_pickups.is_empty() {
        return;
    }

    // Tiles That Should Keep Their Pickup (Un-Collected at Save Time)
    let keep: std::collections::HashSet<(i32, i32)> =
        pending.present_tiles.iter().map(|t| (t[0], t[1])).collect();

    let mut removed = 0usize;
    for (e, pickup) in q_pickups.iter() {
        let tile = (pickup.tile.x, pickup.tile.y);
        if !keep.contains(&tile) {
            commands.entity(e).try_despawn();
            removed += 1;
        }
    }

    info!(
        "Removed {} already-collected pickups on load ({} kept)",
        removed,
        keep.len()
    );

    // Consume So This Only Runs Once Per Load
    pending.active = false;
    pending.present_tiles.clear();
}

/// Helper For Menu / Load Trigger
/// Reads Slot From Disk Into LoadGameRequested and Sets CurrentLevel
/// Returns True if Save Was Found and Queued
/// Caller Should Set the Rebuild in Motion After This Returns True
pub fn begin_load(
    slot: u32,
    load_req: &mut LoadGameRequested,
    current_level: &mut CurrentLevel,
) -> bool {
    match storage::read_slot(slot) {
        Ok(Some(game)) => {
            current_level.0 = capture::level_from_ref(game.level);
            load_req.0 = Some(game);
            true
        }
        Ok(None) => {
            info!("Load slot {slot} is empty");
            false
        }
        Err(e) => {
            error!("Load from slot {slot} failed: {e:?}");
            false
        }
    }
}
