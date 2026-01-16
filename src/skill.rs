/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

/// Selected Skill Level (Difficulty)
/// Maps to Wolfenstein 3D's 4 difficulty settings
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLevel(pub u8);

impl Default for SkillLevel {
    fn default() -> Self {
        // Default to "Don't hurt me" (level 1)
        Self(1)
    }
}

impl SkillLevel {
    /// Can I play, Daddy? (Easiest)
    pub const DADDY: u8 = 0;
    
    /// Don't hurt me (Easy)
    pub const DONT_HURT_ME: u8 = 1;
    
    /// Bring 'em on! (Medium)
    pub const BRING_EM_ON: u8 = 2;
    
    /// I am Death incarnate! (Hardest)
    pub const DEATH_INCARNATE: u8 = 3;

    /// Get the plane1 spawn offset for this difficulty
    /// Wolf3D uses 3 spawn density bands spaced by +36:
    /// - Easy (levels 0-1): offset 0
    /// - Medium (level 2): offset 36
    /// - Hard (level 3): offset 72
    pub fn spawn_offset(&self) -> u16 {
        match self.0 {
            0 | 1 => 0,   // Easy difficulties use base spawn codes
            2 => 36,      // Medium uses +36 offset
            3 => 72,      // Hard uses +72 offset
            _ => 0,       // Fallback to easy
        }
    }

    /// Get damage multiplier for this difficulty
    /// Wolf3D reduces enemy damage on easier difficulties
    pub fn damage_multiplier(&self) -> f32 {
        match self.0 {
            0 => 0.5,  // Daddy: 50% damage
            1 => 0.75, // Don't hurt me: 75% damage
            2 => 1.0,  // Bring 'em on: 100% damage
            3 => 1.0,  // Death incarnate: 100% damage
            _ => 1.0,
        }
    }

    /// Should enemies have faster reaction times?
    /// Wolf3D uses faster enemy AI on harder difficulties
    pub fn fast_enemies(&self) -> bool {
        self.0 >= 3
    }

    /// Get the difficulty name
    pub fn name(&self) -> &'static str {
        match self.0 {
            0 => "Can I play, Daddy?",
            1 => "Don't hurt me",
            2 => "Bring 'em on!",
            3 => "I am Death incarnate!",
            _ => "Don't hurt me",
        }
    }

    /// Create from menu selection index (0-3)
    pub fn from_selection(selection: usize) -> Self {
        Self(selection.min(3) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_offsets() {
        assert_eq!(SkillLevel(0).spawn_offset(), 0);
        assert_eq!(SkillLevel(1).spawn_offset(), 0);
        assert_eq!(SkillLevel(2).spawn_offset(), 36);
        assert_eq!(SkillLevel(3).spawn_offset(), 72);
    }

    #[test]
    fn test_damage_multipliers() {
        assert_eq!(SkillLevel(0).damage_multiplier(), 0.5);
        assert_eq!(SkillLevel(1).damage_multiplier(), 0.75);
        assert_eq!(SkillLevel(2).damage_multiplier(), 1.0);
        assert_eq!(SkillLevel(3).damage_multiplier(), 1.0);
    }
}
