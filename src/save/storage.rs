/*
Davenstein - by David Petnick

Storage Module - Native Filesystem Seam
This Is the Only File That Touches the Filesystem
Everything Above This Deals in SaveGame Structs and RON Strings
When WASM Target Is Added, This File Changes to Browser Storage
Save Model, Capture, and Menu Code Stay Identical
Native Implementation Stores Saves Under Platform Data Dir as Slot Files
*/

use std::path::PathBuf;

use crate::save::model::{
    SaveGame,
    SAVE_FORMAT_VERSION,
};

/// How Many Save Slots the UI Exposes
/// Wolf3D-Style Numbered Slots
pub const SLOT_COUNT: u32 = 10;

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Serialize(String),
    Deserialize(String),
    /// On-Disk Version Did Not Match What This Build Understands
    VersionMismatch { found: u32, expected: u32 },
}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "I/O error: {e}"),
            SaveError::Serialize(s) => write!(f, "serialize error: {s}"),
            SaveError::Deserialize(s) => write!(f, "deserialize error: {s}"),
            SaveError::VersionMismatch { found, expected } =>
                write!(f, "save version mismatch: found {found}, expected {expected}"),
        }
    }
}

/// Directory Where Save Files Live
fn save_dir() -> Result<PathBuf, SaveError> {
    Ok(davelib::app_paths::save_dir()?)
}

fn slot_path(slot: u32) -> Result<PathBuf, SaveError> {
    Ok(save_dir()?.join(format!("savegam{slot}.ron")))
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|candidate| candidate == &path) {
        paths.push(path);
    }
}

fn load_paths(slot: u32) -> Result<Vec<PathBuf>, SaveError> {
    let mut paths = vec![slot_path(slot)?];
    let filename = format!("savegam{slot}.ron");

    if let Ok(executable_dir) = davelib::app_paths::executable_dir() {
        push_unique(&mut paths, executable_dir.join("saves").join(&filename));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique(&mut paths, current_dir.join("saves").join(&filename));
    }

    Ok(paths)
}

/// Serialize and Write a SaveGame to the Given Slot
pub fn write_slot(slot: u32, game: &SaveGame) -> Result<(), SaveError> {
    let dir = save_dir()?;
    std::fs::create_dir_all(&dir)?;

    let ron_str = ron::ser::to_string_pretty(game, ron::ser::PrettyConfig::default())
        .map_err(|e| SaveError::Serialize(e.to_string()))?;

    std::fs::write(slot_path(slot)?, ron_str.as_bytes())?;
    Ok(())
}

/// Read and Deserialize a SaveGame From the Given Slot
/// Returns Ok(None) if the Slot Is Empty
pub fn read_slot(slot: u32) -> Result<Option<SaveGame>, SaveError> {
    for path in load_paths(slot)? {
        if !path.exists() {
            continue;
        }

        let bytes = std::fs::read(&path)?;
        let text = String::from_utf8_lossy(&bytes);

        let game: SaveGame =
            ron::from_str(&text).map_err(|e| SaveError::Deserialize(e.to_string()))?;

        if game.version != SAVE_FORMAT_VERSION {
            return Err(SaveError::VersionMismatch {
                found: game.version,
                expected: SAVE_FORMAT_VERSION,
            });
        }

        return Ok(Some(game));
    }

    Ok(None)
}

/// Whether a Slot Currently Holds a Save
/// Used by the Load Menu to Show or Grey Out Slots
pub fn slot_occupied(slot: u32) -> bool {
    match load_paths(slot) {
        Ok(paths) => paths.into_iter().any(|path| path.exists()),
        Err(_) => false,
    }
}

/// Lightweight Per-Slot Summary For the Load / Save Slot List UI
/// None = Empty Slot
/// Some = Stored Save Name and Level Metadata
/// Name May Be Empty For Very Old Saves Written Before the Name Field Existed
#[derive(Debug, Clone)]
pub struct SlotMeta {
    pub name: String,
    pub episode: u8,
    pub floor: u8,
}

/// Read Summaries For All SLOT_COUNT Slots
/// Index = Slot Number
/// Empty or Unreadable Slots Return None and Show as "- empty -" in the UI
/// Full Deserialization Is Cheap For 10 Tiny RON Files Read Once on Menu Open
pub fn read_all_slot_meta() -> Vec<Option<SlotMeta>> {
    (0..SLOT_COUNT)
        .map(|slot| match read_slot(slot) {
            Ok(Some(game)) => Some(SlotMeta {
                name: game.name,
                episode: game.level.episode,
                floor: game.level.floor,
            }),
            _ => None,
        })
        .collect()
}
