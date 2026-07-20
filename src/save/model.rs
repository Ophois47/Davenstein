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
}

/// Mirrors Player-Facing Run State Held in HudState
/// Includes Keys and Weapons
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
        }
    }
}
