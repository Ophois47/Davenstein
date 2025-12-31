/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

use davelib::audio::{PlaySfx, SfxKind};
use davelib::map::{MapGrid, Tile};
use davelib::player::{Player, PlayerControlLock};
use davelib::world::RebuildWalls;

/// Latched "win" state (like GameOver), driven by using the elevator switch.
#[derive(Resource, Debug, Clone, Default)]
pub struct LevelComplete(pub bool);

/// Marker for the full-screen "MISSION SUCCESS" UI overlay.
#[derive(Component)]
pub struct MissionSuccessOverlay;

/// Wall IDs for the elevator switch textures (Wolf wall IDs, not atlas chunk indices).
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

    // 4-way facing (same rule as doors).
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

    // Only the "down" switch can be activated.
    let wall_id = grid.plane0_code(tx, tz);
    if wall_id != ELEV_SWITCH_DOWN_WALL_ID {
        return;
    }

    // Flip switch texture.
    grid.set_plane0_code(tx, tz, ELEV_SWITCH_UP_WALL_ID);

    // Rebuild wall faces so the flipped wall ID is visible immediately.
    rebuild.write(RebuildWalls { skip: None });

    // Play elevator switch sound (add the asset + mapping in audio.rs).
    sfx.write(PlaySfx {
        kind: SfxKind::ElevatorSwitch,
        pos: Vec3::new(target.x as f32, 0.6, target.y as f32),
    });

    // Latch win state and freeze gameplay.
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

pub fn mission_success_input(
    keys: Res<ButtonInput<KeyCode>>,
    win: Res<LevelComplete>,
    mut new_game: ResMut<crate::ui::sync::NewGameRequested>,
    mut current_level: ResMut<davelib::level::CurrentLevel>,
) {
    // Only while mission success is active, and only once.
    if !win.0 || new_game.0 {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) {
        use davelib::level::LevelId;

        // Temporary progression table until more maps exist.
        current_level.0 = match current_level.0 {
            LevelId::E1M1 => LevelId::E1M2,
            LevelId::E1M2 => LevelId::E1M1,
        };

        new_game.0 = true;
        info!("Mission Success: advancing to {:?} -> new game requested", current_level.0);
    }
}
