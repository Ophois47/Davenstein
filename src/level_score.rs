/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

/// Per-Floor Tallies for Intermission ("MISSION SUCCESS") Screen
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
