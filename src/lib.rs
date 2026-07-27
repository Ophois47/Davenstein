/*
Davenstein - by David Petnick
*/

pub mod actors;
pub mod app_paths;
pub mod ai;
pub mod ai_areas;
pub mod ai_patrol;
pub mod audio;
pub mod decorations;
pub mod enemies;
#[path = "episode_end_markers.rs"]
pub mod episode_end;
pub mod high_score;
pub mod input;
pub mod level;
pub mod level_score;
pub mod map;
pub mod options;
pub mod perf_overlay;
pub mod player;
pub mod pushwalls;
pub mod skill;
pub mod world;

// Application Modules Originally Compiled in the Desktop Binary Refer to the
// Shared Library as 'davelib'. Alias This Crate to Itself so Those Paths Stay
// Unchanged While Desktop and Mobile Entrypoints Share One Application Host
extern crate self as davelib;

// Keep Application-Owned Modules at the Crate Root so Existing 'crate::' Paths,
// System Wiring, and the Shared Episode-End Marker Boundary Remain Unchanged
mod combat;
#[path = "episode_end.rs"]
mod episode_end_app;
mod level_complete;
mod pak_assets;
mod pickups;
mod restart;
mod save;
mod settings;
mod ui;

mod app;

pub(crate) use app::world_ready;
pub use app::run;
