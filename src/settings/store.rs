/*
Davenstein - by David Petnick

Storage Seam for Player Settings - the Only File That Touches the Filesystem for
Options. Mirrors save/storage.rs: RON on Disk, Path Resolved via
'davelib::app_paths' (so It Follows Installed vs Portable Automatically), With a
Version Guard.

Load Returns Ok(None) for Any Situation Where the Player Should Simply Keep Their
Live Defaults - No File Yet (First Run) or a File Whose Version This Build Does
Not Understand. Genuine I/O and Parse Failures Return Err so the Caller Can Log
Them, but Never Crash the Game
*/

use crate::settings::model::{SettingsFile, SETTINGS_FORMAT_VERSION};

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    Serialize(String),
    Deserialize(String),
}

impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        SettingsError::Io(e)
    }
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsError::Io(e) => write!(f, "I/O error: {e}"),
            SettingsError::Serialize(s) => write!(f, "serialize error: {s}"),
            SettingsError::Deserialize(s) => write!(f, "deserialize error: {s}"),
        }
    }
}

/// Read and Parse settings.ron. Ok(None) Means "No Usable File, Keep Defaults"
/// (Absent File or a Version This Build Does Not Recognize). Err Means the File
/// Existed but Could Not Be Read or Parsed
pub fn load() -> Result<Option<SettingsFile>, SettingsError> {
    let path = davelib::app_paths::settings_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(&path)?;
    let text = String::from_utf8_lossy(&bytes);

    let file: SettingsFile =
        ron::from_str(&text).map_err(|e| SettingsError::Deserialize(e.to_string()))?;

    // Unknown Version: Do Not Risk Applying a Format We Do Not Understand. Fall
    // Back to Defaults Rather Than Failing, so a Future Downgrade Stays Playable
    if file.version != SETTINGS_FORMAT_VERSION {
        return Ok(None);
    }

    Ok(Some(file))
}

/// Serialize and Write settings.ron, Creating the Data Directory if Needed
pub fn save(file: &SettingsFile) -> Result<(), SettingsError> {
    let path = davelib::app_paths::settings_path()?;

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let ron_str = ron::ser::to_string_pretty(file, ron::ser::PrettyConfig::default())
        .map_err(|e| SettingsError::Serialize(e.to_string()))?;

    std::fs::write(&path, ron_str.as_bytes())?;
    Ok(())
}
