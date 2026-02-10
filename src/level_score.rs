/*
Davenstein - by David Petnick
*/
use crate::level::LevelId;

use bevy::prelude::*;

/// Per Floor Tallies for Intermission ("MISSION SUCCESS") Screen
#[derive(Resource, Debug, Clone)]
pub struct LevelScore {
    pub kills_found: i32,
    pub kills_total: i32,

    pub secrets_found: i32,
    pub secrets_total: i32,

    pub treasure_found: i32,
    pub treasure_total: i32,

    pub time_secs: f32,
}

impl Default for LevelScore {
    fn default() -> Self {
        Self {
            kills_found: 0,
            kills_total: 0,
            secrets_found: 0,
            secrets_total: 0,
            treasure_found: 0,
            treasure_total: 0,
            time_secs: 0.0,
        }
    }
}

impl LevelScore {
    pub fn reset_for_level(
        &mut self,
        kills_total: usize,
        secrets_total: usize,
        treasure_total: usize,
    ) {
        self.kills_found = 0;
        self.secrets_found = 0;
        self.treasure_found = 0;

        self.kills_total = kills_total as i32;
        self.secrets_total = secrets_total as i32;
        self.treasure_total = treasure_total as i32;

        self.time_secs = 0.0;
    }

    #[inline]
    fn ratio_percent(found: i32, total: i32) -> i32 {
        if total <= 0 {
            return 0;
        }
        let pct = (found as f32) / (total as f32) * 100.0;
        pct.round().clamp(0.0, 100.0) as i32
    }

    pub fn kills_pct(&self) -> i32 {
        Self::ratio_percent(self.kills_found, self.kills_total)
    }
    pub fn secrets_pct(&self) -> i32 {
        Self::ratio_percent(self.secrets_found, self.secrets_total)
    }
    pub fn treasure_pct(&self) -> i32 {
        Self::ratio_percent(self.treasure_found, self.treasure_total)
    }

    pub fn time_mm_ss(&self) -> (i32, i32) {
        let total = self.time_secs.max(0.0).floor() as i32;
        (total / 60, total % 60)
    }
}

/// Tick Only While Gameplay is Running (We Already Gate FixedUpdate with PlayerControlLock)
pub fn tick_level_time(time: Res<Time>, mut score: ResMut<LevelScore>) {
    score.time_secs += time.delta_secs();
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EpisodeLevelStats {
    pub has: bool,
    pub time_secs: f32,
    pub kill_pct: i32,
    pub secret_pct: i32,
    pub treasure_pct: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EpisodeSummary {
    pub total_time_secs: f32,
    pub avg_kill_pct: i32,
    pub avg_secret_pct: i32,
    pub avg_treasure_pct: i32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct EpisodeStats {
    pub episode: u8,
    pub levels: [EpisodeLevelStats; 11],
}

impl EpisodeStats {
    pub fn clear(&mut self) {
        self.episode = 0;
        self.levels = [EpisodeLevelStats::default(); 11];
    }

    pub fn record_level(&mut self, level: LevelId, score: &LevelScore) {
        let ep = level.episode();

        if self.episode == 0 {
            self.episode = ep;
        } else if self.episode != ep {
            self.clear();
            self.episode = ep;
        }

        let floor = level.floor_number();
        if floor <= 0 {
            return;
        }

        let idx = floor as usize;
        if idx >= self.levels.len() {
            return;
        }

        self.levels[idx] = EpisodeLevelStats {
            has: true,
            time_secs: score.time_secs,
            kill_pct: score.kills_pct(),
            secret_pct: score.secrets_pct(),
            treasure_pct: score.treasure_pct(),
        };
    }

    pub fn summary_for_episode(&self, episode: u8) -> EpisodeSummary {
        if self.episode != episode {
            return EpisodeSummary::default();
        }

        let mut sum_time = 0.0f32;
        let mut sum_kill = 0i32;
        let mut sum_secret = 0i32;
        let mut sum_treasure = 0i32;

        for mission in 1..=8 {
            let s = self.levels[mission];
            if s.has {
                sum_time += s.time_secs;
                sum_kill += s.kill_pct;
                sum_secret += s.secret_pct;
                sum_treasure += s.treasure_pct;
            }
        }

        EpisodeSummary {
            total_time_secs: sum_time,
            avg_kill_pct: sum_kill / 8,
            avg_secret_pct: sum_secret / 8,
            avg_treasure_pct: sum_treasure / 8,
        }
    }
}
