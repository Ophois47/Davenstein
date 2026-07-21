/*
Davenstein - by David Petnick

Save Model - Serializable On-Disk Representation
This Module Defines the Serializable Representation of a Saved Game
It Stays Independent of Live Gameplay Types
capture.rs Translates Live ECS State Into This Model on Save and Back on Load
Gameplay Types Avoid serde Dependencies
On-Disk Format Stays Insulated From Gameplay Refactors
Build Order Keeps Bucket 1 Run State First and Bucket 2 World State Later
world Remains Optional so Bucket-1 Saves Stay Valid After Bucket 2 Lands
*/

use serde::{Deserialize, Serialize};

/// Bump This Whenever On-Disk Format Changes Incompatibly
/// load Checks It and Refuses Mismatched Saves Instead of Deserializing Garbage
pub const SAVE_FORMAT_VERSION: u32 = 1;

/// Top-Level Saved Game Snapshot
/// One SaveGame Per Save Slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    /// Format Version
    /// See SAVE_FORMAT_VERSION
    pub version: u32,

    /// Player-Chosen Save Name Shown in the Slot List
    /// Ex: "First level"
    /// serde(default) Keeps Older Saves Loadable With Empty Name
    #[serde(default)]
    pub name: String,

    // ---- Bucket 1: Run State (Implemented Now) ----
    pub run_state: RunState,
    pub player: PlayerSnapshot,
    /// Level Identified by (episode, floor)
    /// Stable Even if LevelId Enum Is Reordered Later
    /// episode = 1 - 6, floor = 1 - 10
    pub level: LevelRef,
    pub level_score: LevelScoreSnapshot,

    // ---- Bucket 2: World State (Added Later) ----
    /// None = Resume at Fresh Level Start
    /// Some = Full Enemy / Door / Pushwall / Pickup Restore
    pub world: Option<WorldSnapshot>,

    /// Cross-Level Episode Tally Mirrored From the EpisodeStats Resource
    /// None = Older Save Without the Field, Load Leaves the Live Tally Untouched
    /// Some = Restore the Accumulated Per-Level Rows so a Mid-Episode Load Keeps
    /// the Episode-End Summary Correct
    #[serde(default)]
    pub episode_stats: Option<EpisodeStatsSnapshot>,
}

/// Mirrors Player-Facing Run State Held in HudState (Keys and Weapons)
/// Also Carries the Selected Skill Level, Which Lives in the SkillLevel
/// Resource Rather Than HudState, so a Load Rebuilds at the Saved Difficulty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub hp: i32,
    pub ammo: i32,
    pub score: i32,
    pub lives: i32,
    pub key_gold: bool,
    pub key_silver: bool,
    /// WeaponSlot Discriminant (0 = Knife, 1 = Pistol, 2 = MachineGun, 3 = Chaingun)
    pub selected_weapon: u8,
    /// Bitmask of Owned Weapons Matching HudState.owned_mask
    pub owned_mask: u8,

    /// Selected Skill Level (0 = Easy, 1 = Medium, 2 = Hard, 3 = Nightmare)
    /// Mirrored From the SkillLevel Resource Rather Than HudState, Which Does Not
    /// Hold Difficulty. Persisted so a Load Rebuilds the Level With the Same
    /// Enemy Spawn Set (Chosen via spawn_offset) and the Same Damage Scaling It
    /// Was Saved Under. Restoring the Wrong Skill Would Also Misalign the
    /// Dead-Enemy Restore, Which Matches on Stable Per-Kind Spawn Index
    /// serde(default) Keeps Older Saves Loadable at Skill 0 (Easy)
    #[serde(default)]
    pub skill: u8,
}

/// One Level's Contribution to the Episode Tally, Mirrored From EpisodeLevelStats
/// Kept as a Plain DTO Because the Engine Type Does Not Derive Serialize
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EpisodeLevelSnapshot {
    pub has: bool,
    pub time_secs: f32,
    pub kill_pct: i32,
    pub secret_pct: i32,
    pub treasure_pct: i32,
}

/// Cross-Level Episode Tally Mirrored From the EpisodeStats Resource. Restored on
/// Load so the Episode-End Summary Matches an Uninterrupted Run. levels Holds One
/// Row per Mission Slot in Floor Order, the Same Layout as EpisodeStats.levels
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EpisodeStatsSnapshot {
    pub episode: u8,
    pub levels: Vec<EpisodeLevelSnapshot>,
}

/// A Single Pickup Persisted With Its Kind so a Load Re-Spawns It Exactly.
/// dropped Marks an Enemy Drop Such as a Boss Key, Which on Restore Gets Back
/// Its DroppedPickup Marker and Small Y Lift That a Map Pickup Lacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupSnapshot {
    pub tile: [i32; 2],
    pub kind: crate::pickups::PickupKind,
    pub dropped: bool,
}

/// A Single Living Enemy at Save Time, Matched Back to Its Rebuilt Twin by
/// (kind, index). An hp_cur of 0 or Less Means the Enemy Was Dying and Comes
/// Back as a Corpse, so a Boss Caught Mid Death Animation Stays Dead. Health,
/// Position, and ai_state Restore the Mid-Fight Scene. Transient AI (the Move
/// Target and Burst Fire) Is Not Stored and Re-Derives Next Tick, so the Save
/// Format Stays Free of the AI Internals Slated for a Rewrite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemySnapshot {
    pub kind: u8,
    pub index: u32,
    pub hp_cur: i32,
    pub pos: [f32; 3],
    pub tile: [i32; 2],
    /// EnemyAiState as u8 (0 = Stand, 1 = Patrol, 2 = Chase), the Alert Level
    pub ai_state: u8,
    pub last_step: [i32; 2],
}

/// Player Position, Facing, and Vitals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    /// World-Space Position (x, y, z)
    pub pos: [f32; 3],
    /// Look Angles in Radians
    pub yaw: f32,
    pub pitch: f32,
    pub hp: i32,
    pub hp_max: i32,
}

/// Stable Level Reference
/// episode = 1 - 6, floor = 1 - 10
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LevelRef {
    pub episode: u8,
    pub floor: u8,
}

/// Mirrors LevelScore Per-Level Progress Counters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelScoreSnapshot {
    pub kills_found: i32,
    pub kills_total: i32,
    pub secrets_found: i32,
    pub secrets_total: i32,
    pub treasure_found: i32,
    pub treasure_total: i32,
    pub time_secs: f32,
}

/// Pushwalls That Fully Completed Their Slide at Save Time
/// Each Records the Final Wall Tile, Direction, Texture, and Distance
/// On Load, the Affected Grid Tiles Are Re-Applied
/// Mid-Slide Pushwalls Are Not Captured and Reset to Un-Pushed
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldSnapshot {
    /// Enemies Dead or Dying at Save Time
    /// Identified by Stable (kind, spawn index)
    /// Load Restores These as Corpses
    /// Survivors Are Not Listed and Rebuild Alive
    pub dead_enemies: Vec<DeadEnemy>,
    /// Tiles That Still Hold an Un-Collected Pickup at Save Time
    /// On Load, Pickups Whose Tile Is Not Listed Here Are Despawned
    /// (They Were Already Collected)
    #[serde(default)]
    pub present_pickups: Vec<[i32; 2]>,
    /// Tiles of Doors That Were Open (or Opening) at Save Time
    /// On Load, These Doors Are Re-Opened (Others Spawn Closed)
    #[serde(default)]
    pub open_doors: Vec<[i32; 2]>,
    /// Pushwalls That Fully Completed Their Slide at Save Time
    /// Each Is the Final Wall Tile, Push Direction, and Wall Texture Id
    /// On Load, the Three Affected Grid Tiles Are Re-Applied
    /// Mid-Slide Pushwalls Are Not Captured and Reset to Un-Pushed
    #[serde(default)]
    pub pushwalls: Vec<PushwallRec>,

    /// Whether This Save Carries Explicit Pushwall Marker and Credit State
    /// (Always True for Saves Written by the Current Build). Older Saves Leave
    /// This False, so Load Falls Back to Rederiving Marker State From the
    /// Completed Pushwall Records
    #[serde(default)]
    pub pushwall_state_saved: bool,

    /// Tiles Marked as Pushable at Save Time. Restored Verbatim so a Reversible
    /// Wall That Migrated to a New Tile Stays Pushable There, and a Consumed Wall
    /// Does Not Reappear as Pushable at Its Original Tile After a Load
    #[serde(default)]
    pub marked_tiles: Vec<[i32; 2]>,

    /// Tiles Whose Secret Was Already Credited at Save Time. Restored so Pushing
    /// a Reversible Wall Again After a Load Cannot Re-Count the Same Secret
    #[serde(default)]
    pub credited_tiles: Vec<[i32; 2]>,

    /// Whether This Save Carries a Full Kinded Pickup List (Always True for Saves
    /// Written by the Current Build). When True the Load Is Authoritative: It
    /// Despawns the Rebuilt Pickups and Re-Spawns pickups_full Verbatim, Which
    /// Also Restores Enemy-Dropped Items (Keys) the Map Rebuild Cannot Recreate.
    /// Older Saves Leave This False and Fall Back to the present_pickups Tile Set
    #[serde(default)]
    pub pickups_authoritative: bool,

    /// Every Live Pickup at Save Time, Map-Placed and Enemy-Dropped Alike, With
    /// Its Kind. Used Only When pickups_authoritative Is True
    #[serde(default)]
    pub pickups_full: Vec<PickupSnapshot>,

    /// Every Living Enemy at Save Time, Matched Back by (kind, index) After the
    /// Rebuild Spawns Them Fresh. Empty for Older Saves, Which Restore Only the
    /// Dead Enemies as Corpses and Leave the Rest at Full Health
    #[serde(default)]
    pub enemies: Vec<EnemySnapshot>,
}

fn default_pushwall_tiles_moved() -> u8 {
    2
}

/// A Completed Pushwall, Enough to Re-Apply Its Grid Effect on Load
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PushwallRec {
    pub dest: [i32; 2],
    pub dir: [i32; 2],
    pub wall_id: u16,
    #[serde(default = "default_pushwall_tiles_moved")]
    pub tiles_moved: u8,
}

/// Dead Enemy Identity For Corpse Restore
/// Kind + Per-Kind Spawn Order Is Enough to Restore on Load
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DeadEnemy {
    pub kind: u8,
    pub index: u32,
}

impl SaveGame {
    /// Construct a Bucket-1 Snapshot Without World State
    pub fn new_bucket1(
        name: String,
        run_state: RunState,
        player: PlayerSnapshot,
        level: LevelRef,
        level_score: LevelScoreSnapshot,
    ) -> Self {
        Self {
            version: SAVE_FORMAT_VERSION,
            name,
            run_state,
            player,
            level,
            level_score,
            world: None,
            episode_stats: None,
        }
    }
}
