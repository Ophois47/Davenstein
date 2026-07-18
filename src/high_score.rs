/*
Davenstein - by David Petnick
*/

use crate::app_paths;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Wolfenstein 3-D had 7 High Score Slots
pub const MAX_SCORES: usize = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighScoreEntry {
    pub name: String,      // 3 Letter Initials
    pub score: i32,        // Final Score When Game Ended
    pub episode: u8,       // Which Episode (1 - 6)
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct HighScores {
    pub entries: Vec<HighScoreEntry>,
}

impl Default for HighScores {
    fn default() -> Self {
        // Match Original Wolfenstein 3-D Default High Scores
        Self {
            entries: vec![
                HighScoreEntry { name: "IDS".into(), score: 10000, episode: 1 },
                HighScoreEntry { name: "ADR".into(), score: 10000, episode: 1 },
                HighScoreEntry { name: "JOH".into(), score: 10000, episode: 1 },
                HighScoreEntry { name: "KEV".into(), score: 10000, episode: 1 },
                HighScoreEntry { name: "TOM".into(), score: 10000, episode: 1 },
                HighScoreEntry { name: "JRO".into(), score: 10000, episode: 1 },
                HighScoreEntry { name: "JAY".into(), score: 10000, episode: 1 },
            ],
        }
    }
}

impl HighScores {
    fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
        if !paths.iter().any(|candidate| candidate == &path) {
            paths.push(path);
        }
    }

    fn legacy_highscores_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Ok(executable_dir) = app_paths::executable_dir() {
            Self::push_unique(
                &mut paths,
                executable_dir.join("data").join("highscores.ron"),
            );
            Self::push_unique(&mut paths, executable_dir.join("highscores.ron"));
        }

        if let Ok(current_dir) = std::env::current_dir() {
            Self::push_unique(&mut paths, current_dir.join("highscores.ron"));
        }

        if let Some(config_dir) = dirs::config_dir() {
            Self::push_unique(
                &mut paths,
                config_dir
                    .join(app_paths::APP_DIRECTORY_NAME)
                    .join("highscores.ron"),
            );
        }

        paths
    }

    fn load_candidates() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Ok(path) = app_paths::high_scores_path() {
            Self::push_unique(&mut paths, path);
        }

        for path in Self::legacy_highscores_paths() {
            Self::push_unique(&mut paths, path);
        }

        paths
    }

    fn save_path() -> Option<PathBuf> {
        let path = app_paths::high_scores_path().ok()?;
        let parent = path.parent()?;
        std::fs::create_dir_all(parent).ok()?;
        Some(path)
    }

    fn atomic_write(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
        let tmp = path.with_extension("ron.tmp");
        std::fs::write(&tmp, contents)?;

        #[cfg(windows)]
        {
            let _ = std::fs::remove_file(path);
        }

        std::fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn load() -> Self {
        for path in Self::load_candidates() {
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };

            let Ok(scores) = ron::from_str::<Self>(&contents) else {
                continue;
            };

            return scores;
        }

        Self::default()
    }

    pub fn save(&self) {
        let Some(path) = Self::save_path() else {
            warn!("Unable to resolve the Davenstein high score path");
            return;
        };

        let Ok(contents) = ron::ser::to_string_pretty(self, Default::default()) else {
            warn!("Unable to serialize Davenstein high scores");
            return;
        };

        if let Err(error) = Self::atomic_write(&path, &contents) {
            warn!("Unable to save high scores to {}: {error}", path.display());
        }
    }

    pub fn qualifies(&self, score: i32) -> bool {
        self.entries.len() < MAX_SCORES || self.entries.last().is_some_and(|e| score > e.score)
    }

    pub fn add(&mut self, name: String, score: i32, episode: u8) -> Option<usize> {
        if !self.qualifies(score) {
            return None;
        }

        let entry = HighScoreEntry {
            name: name
                .chars()
                .filter(|c| !c.is_control()) // Filter ALL Control Chars Including \n
                .take(3)
                .collect(),
            score,
            episode,
        };

        let rank = self
            .entries
            .iter()
            .position(|e| score > e.score)
            .unwrap_or(self.entries.len());

        self.entries.insert(rank, entry);
        self.entries.truncate(MAX_SCORES);

        self.save();
        Some(rank)
    }
}

/// Resource to Trigger High Score Check Flow
#[derive(Resource, Debug, Clone)]
pub struct CheckHighScore {
    pub score: i32,
    pub episode: u8,
    pub checked: bool,
}

/// Resource to Manage Name Entry State
#[derive(Resource, Debug, Clone)]
pub struct NameEntryState {
    pub active: bool,
    pub name: String,       // Current Name Being Typed (Max 3 Chars)
    pub cursor_pos: usize,  // 0, 1, or 2
    pub rank: usize,        // Where This Score Will be Inserted (0 - 6)
    pub score: i32,         // Score to be Saved
    pub episode: u8,        // Episode Number
}

impl Default for NameEntryState {
    fn default() -> Self {
        Self {
            active: false,
            name: String::new(),
            cursor_pos: 0,
            rank: 0,
            score: 0,
            episode: 1,
        }
    }
}
