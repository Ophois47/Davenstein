/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Original Wolf3D had 7 high score slots
pub const MAX_SCORES: usize = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighScoreEntry {
    pub name: String,      // 3-letter initials (Wolf3D style)
    pub score: i32,        // Final score when game ended
    pub episode: u8,       // Which episode (1-6)
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct HighScores {
    pub entries: Vec<HighScoreEntry>,
}

impl Default for HighScores {
    fn default() -> Self {
        // Match original Wolf3D default high scores
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
    fn config_path() -> Option<PathBuf> {
        #[cfg(debug_assertions)]
        {
            // Debug builds: save in project directory
            let mut p = std::env::current_dir().ok()?;
            p.push("highscores.ron");
            Some(p)
        }
        #[cfg(not(debug_assertions))]
        {
            // Release builds: save in AppData
            dirs::config_dir().and_then(|mut p| {
                p.push("Davenstein");
                std::fs::create_dir_all(&p).ok()?;
                p.push("highscores.ron");
                Some(p)
            })
        }
    }

    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|contents| ron::from_str(&contents).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Ok(contents) = ron::ser::to_string_pretty(self, Default::default()) {
                let _ = std::fs::write(path, contents);
            }
        }
    }

    /// Check if a score qualifies for the high score list
    pub fn qualifies(&self, score: i32) -> bool {
        self.entries.len() < MAX_SCORES || score > self.entries.last().unwrap().score
    }

    /// Add a new high score entry
    /// Returns the rank (0-6) if it qualified, None otherwise
    pub fn add(&mut self, name: String, score: i32, episode: u8) -> Option<usize> {
    if !self.qualifies(score) {
        return None;
    }

    let entry = HighScoreEntry { 
        name: name.chars()
            .filter(|c| !c.is_control())  // Filter out ALL control chars including \n
            .take(3)
            .collect(),
        score, 
        episode 
    };
        
        // Find insertion point
        let rank = self.entries.iter()
            .position(|e| score > e.score)
            .unwrap_or(self.entries.len());
        
        self.entries.insert(rank, entry);
        self.entries.truncate(MAX_SCORES);
        
        self.save();
        Some(rank)
    }
}

/// Resource to trigger high score check flow
#[derive(Resource, Debug, Clone)]
pub struct CheckHighScore {
    pub score: i32,
    pub episode: u8,
    pub checked: bool,
}

/// Resource to manage name entry state
#[derive(Resource, Debug, Clone)]
pub struct NameEntryState {
    pub active: bool,
    pub name: String,       // Current name being typed (max 3 chars)
    pub cursor_pos: usize,  // 0, 1, or 2
    pub rank: usize,        // Where this score will be inserted (0-6)
    pub score: i32,         // Score to be saved
    pub episode: u8,        // Episode number
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
