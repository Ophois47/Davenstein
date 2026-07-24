/*
Davenstein - by David Petnick

Application Data Path Policy
Installed Builds Store Player Data Under the Platform Data Directory
Portable Builds Store Player Data Beside the Executable Under data/
Portable Mode Is Enabled by a portable.flag File Beside the Executable
*/

use std::{
    io,
    path::{Path, PathBuf},
};

pub const APP_DIRECTORY_NAME: &str = "Davenstein";
pub const PORTABLE_MARKER_FILE: &str = "portable.flag";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    Installed,
    Portable,
}

pub fn executable_dir() -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;

    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::NotFound,
            "executable has no parent directory",
        ))
}

pub fn storage_mode() -> io::Result<StorageMode> {
    Ok(storage_mode_for(&executable_dir()?))
}

pub fn data_root() -> io::Result<PathBuf> {
    let executable_dir = executable_dir()?;
    let platform_data_dir = dirs::data_local_dir();
    let mode = storage_mode_for(&executable_dir);
    data_root_for(&executable_dir, platform_data_dir.as_deref(), mode)
}

pub fn save_dir() -> io::Result<PathBuf> {
    Ok(data_root()?.join("saves"))
}

pub fn high_scores_path() -> io::Result<PathBuf> {
    Ok(data_root()?.join("highscores.ron"))
}

pub fn settings_path() -> io::Result<PathBuf> {
    Ok(data_root()?.join("settings.ron"))
}

fn storage_mode_for(executable_dir: &Path) -> StorageMode {
    if executable_dir.join(PORTABLE_MARKER_FILE).is_file() {
        StorageMode::Portable
    } else {
        StorageMode::Installed
    }
}

fn data_root_for(
    executable_dir: &Path,
    platform_data_dir: Option<&Path>,
    mode: StorageMode,
) -> io::Result<PathBuf> {
    match mode {
        StorageMode::Portable => Ok(executable_dir.join("data")),
        StorageMode::Installed => platform_data_dir
            .map(|path| path.join(APP_DIRECTORY_NAME))
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "platform data directory is unavailable",
            )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_data_root_stays_beside_executable() {
        let executable_dir = Path::new("release");
        let platform_data_dir = Path::new("platform-data");

        let result = data_root_for(
            executable_dir,
            Some(platform_data_dir),
            StorageMode::Portable,
        )
        .unwrap();

        assert_eq!(result, PathBuf::from("release").join("data"));
    }

    #[test]
    fn installed_data_root_uses_platform_directory() {
        let executable_dir = Path::new("release");
        let platform_data_dir = Path::new("platform-data");

        let result = data_root_for(
            executable_dir,
            Some(platform_data_dir),
            StorageMode::Installed,
        )
        .unwrap();

        assert_eq!(
            result,
            PathBuf::from("platform-data").join(APP_DIRECTORY_NAME),
        );
    }

    #[test]
    fn installed_data_root_requires_platform_directory() {
        let result = data_root_for(
            Path::new("release"),
            None,
            StorageMode::Installed,
        );

        assert!(result.is_err());
    }
}
