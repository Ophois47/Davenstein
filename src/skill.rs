/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

/// Selected Skill Level (Difficulty)
/// Maps to Wolfenstein 3-D's 4 Difficulty Settings
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillLevel(pub u8);

impl Default for SkillLevel {
    fn default() -> Self {
        // Default to "Don't Hurt Me"
        Self(1)
    }
}

impl SkillLevel {
    /// Can I Play, Daddy? (Easiest)
    pub const DADDY: u8 = 0;
    
    /// Don't Hurt Me (Easy)
    pub const DONT_HURT_ME: u8 = 1;
    
    /// Bring 'Em On! (Medium)
    pub const BRING_EM_ON: u8 = 2;
    
    /// I Am Death Incarnate! (Hardest)
    pub const DEATH_INCARNATE: u8 = 3;

    /// Get Plane1 Spawn Offset for This Difficulty
    /// Wolfenstein 3-D Uses 3 Spawn Density Bands Spaced by +36:
    /// - Easy (Levels 0 - 1): Offset 0
    /// - Medium (Level 2): Offset 36
    /// - Hard (Level 3): Offset 72
    pub fn spawn_offset(&self) -> u16 {
        match self.0 {
            0 | 1 => 0,   // Easy Difficulties use Base Spawn Codes
            2 => 36,      // Medium Uses +36 Offset
            3 => 72,      // Hard Uses +72 Offset
            _ => 0,       // Fallback to Easy
        }
    }

    /// Get Damage Multiplier for Difficulty
    /// Wolfenstein 3-D Reduces Enemy Damage on Easier Difficulties
    pub fn damage_multiplier(&self) -> f32 {
        match self.0 {
            0 => 0.5,  // Can I Play Daddy: 50% damage
            1 => 0.75, // Don't Hurt Me: 75% damage
            2 => 1.0,  // Bring 'Em On: 100% damage
            3 => 1.0,  // Death Incarnate: 100% damage
            _ => 1.0,
        }
    }

    /// Wolfenstein 3-D Uses Faster Enemy AI on Harder Difficulties
    pub fn fast_enemies(&self) -> bool {
        self.0 >= 3
    }

    /// Get Difficulty Name
    pub fn name(&self) -> &'static str {
        match self.0 {
            0 => "Can I play, Daddy?",
            1 => "Don't hurt me",
            2 => "Bring 'em on!",
            3 => "I am Death incarnate!",
            _ => "Don't hurt me",
        }
    }

    /// Create From Menu Selection Index (0 - 3)
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
