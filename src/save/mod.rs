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

    // Frame Counter so We Wait for the Rebuild's Deferred Pickup Spawns Before
    // Touching Them, Mirroring the Pushwall Restore
    pub frames_waited: u32,

    // When authoritative Is True the Load Despawns the Rebuilt Pickups and
    // Re-Spawns full Verbatim (Map and Enemy Drops Alike). Older Saves Leave It
    // False and Fall Back to the present_tiles Keep Set
    pub authoritative: bool,
    pub full: Vec<model::PickupSnapshot>,
}

// Door Tiles That Should Be Open From a Just-Loaded Save
// apply_pending_door_restore Re-Opens These Once the Rebuilt Doors Exist
// active Distinguishes "No Load" From "Load With No Open Doors"
#[derive(Resource, Default)]
pub struct PendingDoorRestore {
    pub active: bool,
    pub open_tiles: Vec<[i32; 2]>,
}

// Completed Pushwalls From a Just-Loaded Save, Waiting to Re-Apply Their Grid
// Effect Once the Rebuilt Level Exists. active Distinguishes "No Load" From
// "Load With No Completed Pushwalls"
#[derive(Resource, Default)]
pub struct PendingPushwallRestore {
    pub active: bool,
    pub items: Vec<model::PushwallRec>,
    pub frames_waited: u32,

    // Explicit Marker and Credit State From the Loaded Save. When state_saved Is
    // True These Are Applied Verbatim via PushwallMarkers::restore_state. When
    // False (Older Saves) Load Falls Back to Consuming the Derived Origin Tile
    pub state_saved: bool,
    pub marked_tiles: Vec<[i32; 2]>,
    pub credited_tiles: Vec<[i32; 2]>,
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveGameRequested>()
            .init_resource::<LoadGameRequested>()
            .init_resource::<PendingDeadRestore>()
            .init_resource::<PendingPickupRestore>()
            .init_resource::<PendingDoorRestore>()
            .init_resource::<PendingPushwallRestore>()
            .add_systems(Update, handle_save_requests)
            .add_systems(Update, apply_pending_dead_restore)
            .add_systems(Update, apply_pending_pickup_restore)
            .add_systems(Update, apply_pending_door_restore)
            .add_systems(Update, apply_pending_pushwall_restore);
    }
}

/// When SaveGameRequested Holds a Slot, Capture Current State and Write It
fn handle_save_requests(
    mut req: ResMut<SaveGameRequested>,
    hud: Res<HudState>,
    current_level: Res<CurrentLevel>,
    level_score: Res<LevelScore>,
    skill: Res<davelib::skill::SkillLevel>,
    episode_stats: Res<davelib::level_score::EpisodeStats>,
    q_player: Query<(&Transform, &PlayerVitals), With<Player>>,
    q_dead: Query<
        (&davelib::enemies::EnemyKind, &davelib::enemies::SpawnIndex),
        With<davelib::actors::Dead>,
    >,
    q_pickups: Query<(&crate::pickups::Pickup, Option<&crate::pickups::DroppedPickup>)>,
    q_doors: Query<(&davelib::map::DoorTile, &davelib::map::DoorState)>,
    completed_pushwalls: Res<davelib::pushwalls::CompletedPushwalls>,
    // Optional Because PushwallMarkers Is Inserted by setup and Does Not Exist
    // Before the First Level Loads (Ex: Saving From a Menu). A Strict Res Would
    // Fail System-Param Validation and Panic in That Window
    markers: Option<Res<davelib::pushwalls::PushwallMarkers>>,
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
        q_pickups.iter().map(|(p, _)| [p.tile.x, p.tile.y]).collect();

    // Every Live Pickup With Its Kind, Map-Placed and Enemy-Dropped Alike. The
    // Load Re-Spawns This List Verbatim, Which Is the Only Way an Enemy-Dropped
    // Item (a Boss Key) Survives, Since the Map Rebuild Cannot Recreate It
    let pickups_full: Vec<model::PickupSnapshot> = q_pickups
        .iter()
        .map(|(p, dropped)| model::PickupSnapshot {
            tile: [p.tile.x, p.tile.y],
            kind: p.kind,
            dropped: dropped.is_some(),
        })
        .collect();

    // Doors Whose want_open Is True Were Open (or Opening) at Save Time
    // Load Re-Opens These, Letting the Normal Door Tick / Auto-Close Take Over
    let open_doors: Vec<[i32; 2]> = q_doors
        .iter()
        .filter(|(_, state)| state.want_open)
        .map(|(door, _)| [door.0.x, door.0.y])
        .collect();

    // Completed Pushwalls, Converted From the Engine Record to the Save Model
    let pushwalls: Vec<model::PushwallRec> = completed_pushwalls
        .items
        .iter()
        .map(|c| model::PushwallRec {
            dest: [c.dest.x, c.dest.y],
            dir: [c.dir.x, c.dir.y],
            wall_id: c.wall_id,
            tiles_moved: c.tiles_moved,
        })
        .collect();

    // Persist the Live Marker and Credit Grids so a Load Restores Them Exactly
    // Rather Than Rederiving From the Completed Records. A Missing PushwallMarkers
    // Resource Means There Is No Pushwall State to Save (No Level Loaded Yet)
    let (marked_tiles, credited_tiles, pushwall_state_saved) = match &markers {
        Some(m) => (
            m.marked_tiles().iter().map(|t| [t.x, t.y]).collect(),
            m.credited_tiles().iter().map(|t| [t.x, t.y]).collect(),
            true,
        ),
        None => (Vec::new(), Vec::new(), false),
    };

    let game = capture::capture_save_game(
        name,
        &hud,
        player_tf,
        vitals,
        &current_level,
        &level_score,
        skill.0,
        &episode_stats,
        dead_enemies,
        present_pickups,
        open_doors,
        pushwalls,
        marked_tiles,
        credited_tiles,
        pushwall_state_saved,
        pickups_full,
        true,
    );

    match storage::write_slot(slot, &game) {
        Ok(()) => info!("Saved Game to Slot {slot}"),
        Err(e) => error!("Save to Slot {slot} Failed: {e:?}"),
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

    for (e, kind, idx) in q_enemies.iter() {
        let key = (capture::enemy_kind_to_u8(*kind), idx.0);
        if dead_set.contains(&key) {
            make_corpse(&mut commands, e, *kind);
        }
    }

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
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !pending.active {
        return;
    }

    // Wait a Few Frames for the Rebuild's Deferred Pickup Spawns to Apply Before
    // We Touch Them. A Frame Count Is Used Instead of "Pickups Exist" so the
    // Authoritative Path Still Runs on a Level That Has No Map Pickups at All
    if pending.frames_waited < 3 {
        pending.frames_waited += 1;
        return;
    }

    if pending.authoritative {
        // The Save Is Authoritative: Despawn Every Rebuilt Pickup, Then Re-Spawn
        // Exactly What Was Saved. This Restores Enemy-Dropped Items (Keys) the
        // Map Rebuild Cannot Recreate, Matching How the Original Game Restored
        // Its Whole Static-Object List
        for (e, _) in q_pickups.iter() {
            commands.entity(e).try_despawn();
        }
        for snap in pending.full.iter() {
            let tile = IVec2::new(snap.tile[0], snap.tile[1]);
            let name = if snap.dropped { "Pickup_Drop" } else { "Pickup" };
            crate::pickups::spawn_pickup_entity(
                &mut commands,
                &mut meshes,
                &mut materials,
                &asset_server,
                tile,
                snap.kind,
                snap.dropped,
                name,
            );
        }
    } else {
        // Older Save Without a Kinded List: Keep the Un-Collected Map Pickups and
        // Despawn the Rest. This Cannot Restore Enemy Drops, It Only Removes
        // Pickups That Were Already Collected at Save Time
        let keep: std::collections::HashSet<(i32, i32)> =
            pending.present_tiles.iter().map(|t| (t[0], t[1])).collect();

        for (e, pickup) in q_pickups.iter() {
            let tile = (pickup.tile.x, pickup.tile.y);
            if !keep.contains(&tile) {
                commands.entity(e).try_despawn();
            }
        }
    }

    // Consume So This Only Runs Once Per Load
    pending.active = false;
    pending.frames_waited = 0;
    pending.present_tiles.clear();
    pending.authoritative = false;
    pending.full.clear();
}

/// Re-Opens Doors That Were Open in a Just-Loaded Save
/// Runs in Update (Not load_game_finish) Because Doors Spawn via Deferred
/// Commands and Are Not Queryable Until a Later Frame
/// Sets want_open + a Fresh Open Timer, so the Normal Door Tick Animates It Open
/// and the Auto-Close System Behaves as if the Player Just Opened It
fn apply_pending_door_restore(
    mut pending: ResMut<PendingDoorRestore>,
    mut q_doors: Query<(&davelib::map::DoorTile, &mut davelib::map::DoorState, &mut davelib::map::DoorAnim)>,
) {
    if !pending.active {
        return;
    }

    // Wait Until the Rebuilt Level's Doors Have Actually Spawned
    if q_doors.is_empty() {
        return;
    }

    // Door Open Duration, Matching the Player Door Logic (DOOR_OPEN_SECS = 4.5)
    const DOOR_OPEN_SECS: f32 = 4.5;

    let open: std::collections::HashSet<(i32, i32)> =
        pending.open_tiles.iter().map(|t| (t[0], t[1])).collect();

    for (door, mut state, mut anim) in q_doors.iter_mut() {
        let tile = (door.0.x, door.0.y);
        if open.contains(&tile) {
            state.want_open = true;
            state.open_timer = DOOR_OPEN_SECS;
            anim.progress = 1.0;
        }
    }

    // Consume So This Only Runs Once Per Load
    pending.active = false;
    pending.open_tiles.clear();
}

/// Re-Applies Completed Pushwalls From a Just-Loaded Save by Stamping Their
/// Grid Effect Back In, Then Rebuilding Wall Geometry
/// Runs in Update and Waits a Few Frames so the Rebuild's Deferred MapGrid /
/// PushwallMarkers Inserts Have Landed Before We Mutate Them
/// Also Repopulates CompletedPushwalls so a Later Save Re-Captures These
fn apply_pending_pushwall_restore(
    mut pending: ResMut<PendingPushwallRestore>,
    grid: Option<ResMut<davelib::map::MapGrid>>,
    markers: Option<ResMut<davelib::pushwalls::PushwallMarkers>>,
    completed: Option<ResMut<davelib::pushwalls::CompletedPushwalls>>,
    mut rebuild: MessageWriter<davelib::world::RebuildWalls>,
) {
    if !pending.active {
        return;
    }

    // These World Resources Are Inserted by setup During a Level Rebuild and Do
    // Not Exist Before the First Level Loads. If Any Is Missing We Are Too Early
    let (Some(mut grid), Some(mut markers), Some(mut completed)) = (grid, markers, completed)
    else {
        return;
    };

    // Wait a Few Frames so setup's Deferred Grid / Marker Inserts Are Applied
    // (MapGrid and PushwallMarkers Are Inserted via Commands During the Rebuild)
    if pending.frames_waited < 3 {
        pending.frames_waited += 1;
        return;
    }

    use davelib::map::Tile;

    let mut applied = 0usize;
    for rec in pending.items.iter() {
        let dest = IVec2::new(rec.dest[0], rec.dest[1]);
        let dir = IVec2::new(rec.dir[0], rec.dir[1]);

        let tiles_moved = rec.tiles_moved.clamp(1, 2);
        let orig = dest - dir * i32::from(tiles_moved);

        // Empty Every Tile the Pushwall Moved Out Of
        for step in 0..tiles_moved {
            let empty_tile = orig + dir * i32::from(step);

            if in_bounds_grid(&grid, empty_tile) {
                grid.set_tile(
                    empty_tile.x as usize,
                    empty_tile.y as usize,
                    Tile::Empty,
                );
                grid.set_plane0_code(
                    empty_tile.x as usize,
                    empty_tile.y as usize,
                    0,
                );
            }
        }

        if in_bounds_grid(&grid, dest) {
            grid.set_tile(dest.x as usize, dest.y as usize, Tile::Wall);
            grid.set_plane0_code(dest.x as usize, dest.y as usize, rec.wall_id);
        }

        // Older Saves Without Explicit Marker State Consume the Derived Origin.
        // That Stops the Wall Being Pushed Again. Newer Saves Skip This Fallback.
        // They Restore the Real Marker and Credit Grids After the Loop Through
        // restore_state, Which Is Exact Even When a Wall Was Pushed Twice
        if !pending.state_saved {
            markers.consume(orig.x, orig.y);
        }

        // Repopulate the Completed Record so a Subsequent Save Re-Captures It
        completed.items.push(davelib::pushwalls::CompletedPushwall {
            dest,
            dir,
            wall_id: rec.wall_id,
            tiles_moved,
        });

        applied += 1;
    }

    if applied > 0 {
        // One Rebuild Brings Wall Geometry in Line With the Modified Grid
        rebuild.write(davelib::world::RebuildWalls { skip: None });
    }

    // Newer Saves Carry the Exact Marker and Credit Grids and Apply Them Verbatim.
    // Running After the Grid Stamping Loop, This Path Replaces the Per-Record
    // consume Guess Used by Older Saves and Restores a Reversible Wall Pushed
    // More Than Once Correctly, so the Same Secret Cannot Be Counted Twice
    if pending.state_saved {
        let marked: Vec<IVec2> = pending
            .marked_tiles
            .iter()
            .map(|t| IVec2::new(t[0], t[1]))
            .collect();
        let credited: Vec<IVec2> = pending
            .credited_tiles
            .iter()
            .map(|t| IVec2::new(t[0], t[1]))
            .collect();
        markers.restore_state(&marked, &credited);
    }

    pending.active = false;
    pending.frames_waited = 0;
    pending.items.clear();
    pending.state_saved = false;
    pending.marked_tiles.clear();
    pending.credited_tiles.clear();
}

/// Bounds Check Mirroring the Pushwall Module's in_bounds, Local to Avoid a Dep
fn in_bounds_grid(grid: &davelib::map::MapGrid, t: IVec2) -> bool {
    t.x >= 0 && t.y >= 0 && (t.x as usize) < grid.width && (t.y as usize) < grid.height
}

/// Helper For Menu / Load Trigger
/// Reads Slot From Disk Into LoadGameRequested and Sets CurrentLevel
/// Returns True if Save Was Found and Queued
/// Caller Should Set the Rebuild in Motion After This Returns True
pub fn begin_load(
    slot: u32,
    load_req: &mut LoadGameRequested,
    current_level: &mut CurrentLevel,
    skill_level: &mut davelib::skill::SkillLevel,
) -> bool {
    match storage::read_slot(slot) {
        Ok(Some(game)) => {
            current_level.0 = capture::level_from_ref(game.level);

            // Restore Skill Before the Rebuild Runs. setup() Reads the SkillLevel
            // Resource and Its spawn_offset to Choose the Enemy Spawn Set. Skill
            // Must Be Correct Now, Not Later in load_game_finish, Which Runs After
            // Enemies Spawn. Setting It Here Beside CurrentLevel Keeps the Restored
            // Dead-Enemy Indices Aligned, Since They Match on a Stable Spawn Index
            skill_level.0 = game.run_state.skill;

            load_req.0 = Some(game);
            true
        }
        Ok(None) => {
            info!("Load Slot {slot} Empty");
            false
        }
        Err(e) => {
            error!("Load From Slot {slot} Failed: {e:?}");
            false
        }
    }
}
