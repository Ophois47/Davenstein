/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

use davelib::audio::{PlaySfx, SfxKind};
use davelib::map::{MapGrid, Tile};
use davelib::player::{Player, PlayerControlLock};
use davelib::world::RebuildWalls;

/// Wall IDs for the Elevator Switch Textures 
// (Wolfenstein Wall IDs, NOT Atlas Chunk Indices)
const ELEV_SWITCH_DOWN_WALL_ID: u16 = 21;
const ELEV_SWITCH_UP_WALL_ID: u16 = 22;

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

#[derive(Resource, Debug, Clone)]
pub struct MissionSuccessTally {
    pub active: bool,
    pub phase: MissionSuccessPhase,

    pub shown_kill: i32,
    pub shown_secret: i32,
    pub shown_treasure: i32,

    pub target_kill: i32,
    pub target_secret: i32,
    pub target_treasure: i32,

    pub shown_time_secs: i32,
    pub target_time_secs: i32,
    pub time_step_accum: i32,

    pub tick: Timer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionSuccessPhase {
    Kill,
    Secret,
    Treasure,
    Done,
}

impl Default for MissionSuccessTally {
    fn default() -> Self {
        Self {
            active: false,
            phase: MissionSuccessPhase::Done,
            shown_kill: 0,
            shown_secret: 0,
            shown_treasure: 0,
            target_kill: 0,
            target_secret: 0,
            target_treasure: 0,

            shown_time_secs: 0,
            target_time_secs: 0,
            time_step_accum: 0,

            tick: Timer::from_seconds(1.0 / 45.0, TimerMode::Repeating),
        }
    }
}

pub fn use_elevator_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut lock: ResMut<PlayerControlLock>,
    mut win: ResMut<LevelComplete>,
    mut grid: ResMut<MapGrid>,
    q_player: Query<&Transform, With<Player>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut rebuild: MessageWriter<RebuildWalls>,
    mut music_mode: ResMut<davelib::audio::MusicMode>,
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

    // Hard cut to end level music
    music_mode.0 = davelib::audio::MusicModeKind::LevelEnd;
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
    tally: Option<Res<MissionSuccessTally>>,
    mut q: Query<(
        &MissionStatText,
        Option<&mut Text>,
        Option<&mut crate::ui::level_end_font::LevelEndBitmapText>,
    )>,
) {
    if !win.0 {
        return;
    }

    fn secs_to_mm_ss(secs: i32) -> (i32, i32) {
        let s = secs.max(0);
        (s / 60, s % 60)
    }

    let floor = current_level.0.floor_number();

    let (kill_pct, secret_pct, treasure_pct, (mm, ss)) = if let Some(t) = tally.as_deref() {
        if t.active {
            (
                t.shown_kill,
                t.shown_secret,
                t.shown_treasure,
                secs_to_mm_ss(t.shown_time_secs),
            )
        } else {
            (score.kills_pct(), score.secrets_pct(), score.treasure_pct(), score.time_mm_ss())
        }
    } else {
        (score.kills_pct(), score.secrets_pct(), score.treasure_pct(), score.time_mm_ss())
    };

    for (tag, text, bt) in q.iter_mut() {
        let s = match tag.kind {
            MissionStatKind::Title => format!("{floor}"),
            MissionStatKind::Time => format!("{}:{:02}", mm, ss),
            MissionStatKind::KillRatio => format!("{kill_pct}%"),
            MissionStatKind::SecretRatio => format!("{secret_pct}%"),
            MissionStatKind::TreasureRatio => format!("{treasure_pct}%"),
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
    buttons: Res<ButtonInput<MouseButton>>,
    win: Res<LevelComplete>,
    mut tally: ResMut<MissionSuccessTally>,
    mut advance: ResMut<crate::ui::sync::AdvanceLevelRequested>,
    mut current_level: ResMut<davelib::level::CurrentLevel>,
    mut music_mode: ResMut<davelib::audio::MusicMode>,
) {
    if !win.0 || advance.0 {
        return;
    }

    let go = keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Space)
        || buttons.just_pressed(MouseButton::Left);

    if !go {
        return;
    }

    if tally.active {
        tally.shown_kill = tally.target_kill;
        tally.shown_secret = tally.target_secret;
        tally.shown_treasure = tally.target_treasure;

        tally.shown_time_secs = tally.target_time_secs;
        tally.time_step_accum = 0;

        tally.active = false;
        tally.phase = MissionSuccessPhase::Done;
        return;
    }

    music_mode.0 = davelib::audio::MusicModeKind::Gameplay;
    current_level.0 = current_level.0.next_e1_normal();
    advance.0 = true;

    info!(
        "Mission Success: advancing to {:?} -> advance level requested",
        current_level.0
    );
}

pub fn start_mission_success_tally_on_win(
    win: Res<LevelComplete>,
    score: Res<davelib::level_score::LevelScore>,
    mut tally: ResMut<MissionSuccessTally>,
    mut prev_win: Local<bool>,
) {
    if !win.0 {
        tally.active = false;
        tally.phase = MissionSuccessPhase::Done;
        *prev_win = false;
        return;
    }

    if *prev_win {
        return;
    }
    *prev_win = true;

    tally.active = true;
    tally.phase = MissionSuccessPhase::Kill;

    tally.shown_kill = 0;
    tally.shown_secret = 0;
    tally.shown_treasure = 0;

    tally.target_kill = score.kills_pct().clamp(0, 100);
    tally.target_secret = score.secrets_pct().clamp(0, 100);
    tally.target_treasure = score.treasure_pct().clamp(0, 100);

    tally.shown_time_secs = 0;
    tally.target_time_secs = score.time_secs.max(0.0).floor() as i32;
    tally.time_step_accum = 0;

    tally.tick.reset();
}

pub fn tick_mission_success_tally(
    time: Res<Time>,
    win: Res<LevelComplete>,
    mut tally: ResMut<MissionSuccessTally>,
) {
    if !win.0 || !tally.active {
        return;
    }

    tally.tick.tick(time.delta());
    let steps = tally.tick.times_finished_this_tick();
    if steps == 0 {
        return;
    }

    for _ in 0..steps {
        // NEW: time advances slower than percent roll:
        // 1 second per 3 tally steps (tweakable)
        if tally.shown_time_secs < tally.target_time_secs {
            tally.time_step_accum += 1;
            if tally.time_step_accum >= 3 {
                tally.time_step_accum = 0;
                tally.shown_time_secs += 1;
            }
        }

        match tally.phase {
            MissionSuccessPhase::Kill => {
                if tally.shown_kill < tally.target_kill {
                    tally.shown_kill += 1;
                } else {
                    tally.phase = MissionSuccessPhase::Secret;
                }
            }
            MissionSuccessPhase::Secret => {
                if tally.shown_secret < tally.target_secret {
                    tally.shown_secret += 1;
                } else {
                    tally.phase = MissionSuccessPhase::Treasure;
                }
            }
            MissionSuccessPhase::Treasure => {
                if tally.shown_treasure < tally.target_treasure {
                    tally.shown_treasure += 1;
                } else {
                    tally.phase = MissionSuccessPhase::Done;
                    tally.active = false;
                    break;
                }
            }
            MissionSuccessPhase::Done => {
                tally.active = false;
                break;
            }
        }
    }
}
