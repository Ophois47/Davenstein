/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

use davelib::audio::{PlaySfx, SfxKind};
use davelib::map::{MapGrid, Tile};
use davelib::player::{Player, PlayerControlLock};
use davelib::world::RebuildWalls;

/// Latched "Win" State, Driven by Elevator Switch
#[derive(Resource, Debug, Clone, Default)]
pub struct LevelComplete(pub bool);

/// Marker for Full Screen "MISSION SUCCESS" UI Overlay
#[derive(Component)]
pub struct MissionSuccessOverlay;

#[derive(Component, Clone, Copy)]
pub enum MissionStatKind {
    Title,
    KillRatio,
    SecretRatio,
    TreasureRatio,
    Time,
}

#[derive(Component, Clone, Copy)]
pub struct MissionStatText {
    pub kind: MissionStatKind,
}

/// Wall IDs for the Elevator Switch Textures 
// (Wolfenstein Wall IDs, NOT Atlas Chunk Indices)
const ELEV_SWITCH_DOWN_WALL_ID: u16 = 21;
const ELEV_SWITCH_UP_WALL_ID: u16 = 22;

pub fn use_elevator_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut lock: ResMut<PlayerControlLock>,
    mut win: ResMut<LevelComplete>,
    mut grid: ResMut<MapGrid>,
    q_player: Query<&Transform, With<Player>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut rebuild: MessageWriter<RebuildWalls>,
) {
    if lock.0 || win.0 {
        return;
    }
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    let Some(player_tf) = q_player.iter().next() else { return; };

    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    let player_tile = world_to_tile(Vec2::new(player_tf.translation.x, player_tf.translation.z));

    // 4-way Facing (Same as Doors)
    let mut fwd = player_tf.rotation * Vec3::NEG_Z;
    fwd.y = 0.0;
    if fwd.length_squared() < 1e-6 {
        return;
    }
    let fwd = fwd.normalize();

    let (dx, dz) = if fwd.x.abs() > fwd.z.abs() {
        (fwd.x.signum() as i32, 0)
    } else {
        (0, fwd.z.signum() as i32)
    };

    let target = IVec2::new(player_tile.x + dx, player_tile.y + dz);
    if target.x < 0
        || target.y < 0
        || target.x >= grid.width as i32
        || target.y >= grid.height as i32
    {
        return;
    }

    let (tx, tz) = (target.x as usize, target.y as usize);
    if grid.tile(tx, tz) != Tile::Wall {
        return;
    }

    // Only Down Switch Can be Activated
    let wall_id = grid.plane0_code(tx, tz);
    if wall_id != ELEV_SWITCH_DOWN_WALL_ID {
        return;
    }

    // Flip Switch Texture
    grid.set_plane0_code(tx, tz, ELEV_SWITCH_UP_WALL_ID);

    // Rebuild Wall Faces so Flipped Wall ID is Visible Immediately
    rebuild.write(RebuildWalls { skip: None });

    // Play Elevator Switch Sound
    sfx.write(PlaySfx {
        kind: SfxKind::ElevatorSwitch,
        pos: Vec3::new(target.x as f32, 0.6, target.y as f32),
    });

    // Latch Win State and Freeze Gameplay
    win.0 = true;
    lock.0 = true;
}

pub fn sync_mission_success_overlay_visibility(
    win: Res<LevelComplete>,
    mut q: Query<&mut Visibility, With<MissionSuccessOverlay>>,
) {
    let Some(mut vis) = q.iter_mut().next() else { return; };

    *vis = if win.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

pub fn sync_mission_success_stats_text(
    win: Res<LevelComplete>,
    score: Res<davelib::level_score::LevelScore>,
    current_level: Res<davelib::level::CurrentLevel>,
    mut q: Query<(
        &MissionStatText,
        Option<&mut Text>,
        Option<&mut crate::ui::level_end_font::LevelEndBitmapText>,
    )>,
) {
    if !win.0 {
        return;
    }

    let floor = current_level.0.floor_number();
    let (mm, ss) = score.time_mm_ss();

    for (tag, text, bt) in q.iter_mut() {
        let s = match tag.kind {
            MissionStatKind::Title => format!("FLOOR {} COMPLETED", floor),
            MissionStatKind::KillRatio => format!("KILL RATIO     {}%", score.kills_pct()),
            MissionStatKind::SecretRatio => format!("SECRET RATIO   {}%", score.secrets_pct()),
            MissionStatKind::TreasureRatio => format!("TREASURE RATIO {}%", score.treasure_pct()),
            MissionStatKind::Time => format!("TIME          {}:{:02}", mm, ss),
        };

        if let Some(mut text) = text {
            text.0 = s;
        } else if let Some(mut bt) = bt {
            bt.text = s;
        }
    }
}

pub fn mission_success_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut win: ResMut<LevelComplete>,
    mut advance: ResMut<crate::ui::sync::AdvanceLevelRequested>,
    mut current_level: ResMut<davelib::level::CurrentLevel>,
) {
    // Only While Mission Success Active, Only Once
    if !win.0 || advance.0 {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) {
        win.0 = false; // Hide mission success immediately on continue

        current_level.0 = current_level.0.next_e1_normal();

        advance.0 = true;
        info!(
            "Mission Success: advancing to {:?} -> advance level requested",
            current_level.0
        );
    }
}
