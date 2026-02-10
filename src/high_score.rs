/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Wolfenstein 3D had 7 High Score Slots
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
        // Match Original Wolfenstein 3D Default High Scores
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
            // Debug Builds: Save in Project Directory
            let mut p = std::env::current_dir().ok()?;
            p.push("highscores.ron");
            Some(p)
        }
        #[cfg(not(debug_assertions))]
        {
            // Release Builds: Save in AppData
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

    /// Check if Score Qualifies for High Score List
    pub fn qualifies(&self, score: i32) -> bool {
        self.entries.len() < MAX_SCORES || score > self.entries.last().unwrap().score
    }

    /// Add New High Score Entry
    /// Returns Rank (0 - 6) if Qualified, None Otherwise
    pub fn add(&mut self, name: String, score: i32, episode: u8) -> Option<usize> {
    if !self.qualifies(score) {
        return None;
    }

    let entry = HighScoreEntry { 
        name: name.chars()
            .filter(|c| !c.is_control()) // Filter ALL Control Chars Including \n
            .take(3)
            .collect(),
        score, 
        episode 
    };
        
        // Find Insertion Point
        let rank = self.entries.iter()
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
