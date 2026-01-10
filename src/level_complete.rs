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
    Par,
    Bonus,
}

#[derive(Component, Clone, Copy)]
pub struct MissionStatText {
    pub kind: MissionStatKind,
}



#[derive(Component, Clone, Copy)]
pub struct MissionStatRightAlign {
    pub right_edge_native: f32,
    pub overlay_scale: f32,
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
    pub pause_steps: u8,

    pub shown_bonus: i32,
    pub target_bonus: i32,

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

            pause_steps: 0,

            shown_bonus: 0,
            target_bonus: 0,

            tick: Timer::from_seconds(1.0 / 120.0, TimerMode::Repeating),
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

    // Hard Cut to End Level Music
    music_mode.0 = davelib::audio::MusicModeKind::LevelEnd;
}

fn par_seconds_ep1(floor: i32) -> Option<u32> {
    // Wolf3D Episode 1 PAR times (floors 1..=8)
    // Boss/secret floors have no par
    // 1:30, 2:00, 2:00, 3:30, 3:00, 3:00, 2:30, 2:30  (E1)
    match floor {
        1 => Some(1 * 60 + 30),
        2 => Some(2 * 60 + 0),
        3 => Some(2 * 60 + 0),
        4 => Some(3 * 60 + 30),
        5 => Some(3 * 60 + 0),
        6 => Some(3 * 60 + 0),
        7 => Some(2 * 60 + 30),
        8 => Some(2 * 60 + 30),
        _ => None, // 9/10 (boss/secret) => no PAR
    }
}

fn mm_ss_from_seconds(total: u32) -> (u32, u32) {
    (total / 60, total % 60)
}


const PAR_AMOUNT: i32 = 500;
const PERCENT100AMT: i32 = 10_000;

fn compute_target_bonus(score: &davelib::level_score::LevelScore, floor: i32) -> i32 {
    let time_secs = score.time_secs.max(0.0).floor() as i32;

    // Time-under-par bonus
    let mut bonus = 0;
    if let Some(par) = par_seconds_ep1(floor) {
        let under = (par as i32 - time_secs).max(0);
        bonus += under * PAR_AMOUNT;
    }

    // +10,000 for each category exactly 100%
    if score.kills_pct() == 100 {
        bonus += PERCENT100AMT;
    }
    if score.secrets_pct() == 100 {
        bonus += PERCENT100AMT;
    }
    if score.treasure_pct() == 100 {
        bonus += PERCENT100AMT;
    }

    bonus.max(0)
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
        Option<&MissionStatRightAlign>,
        &mut Node,
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

    fn text_w_native_px(s: &str) -> f32 {
        let mut w = 0.0;
        for ch in s.chars() {
            w += match ch {
                ' ' => 16.0,
                ':' => 9.0,
                '%' => 16.0,
                '!' => 7.0,
                '\'' => 8.0,
                _ => 16.0,
            };
        }
        w
    }

    let floor = current_level.0.floor_number();

    let (kill_pct, secret_pct, treasure_pct, (mm, ss), bonus_val) = if let Some(t) = tally.as_deref() {
        if t.active {
            (
                t.shown_kill,
                t.shown_secret,
                t.shown_treasure,
                secs_to_mm_ss(t.shown_time_secs),
                t.shown_bonus,
            )
        } else {
            (
                score.kills_pct(),
                score.secrets_pct(),
                score.treasure_pct(),
                score.time_mm_ss(),
                compute_target_bonus(&score, floor),
            )
        }
    } else {
        (
            score.kills_pct(),
            score.secrets_pct(),
            score.treasure_pct(),
            score.time_mm_ss(),
            compute_target_bonus(&score, floor),
        )
    };

    for (tag, align, mut node, text, bt) in q.iter_mut() {
        let s = match tag.kind {
            MissionStatKind::Title => format!("{floor}"),
            MissionStatKind::Time => format!("{}:{:02}", mm, ss),
            MissionStatKind::KillRatio => format!("{kill_pct}%"),
            MissionStatKind::SecretRatio => format!("{secret_pct}%"),
            MissionStatKind::TreasureRatio => format!("{treasure_pct}%"),
            MissionStatKind::Par => match par_seconds_ep1(floor) {
                Some(par_sec) => {
                    let (pm, ps) = mm_ss_from_seconds(par_sec);
                    format!("{}:{:02}", pm, ps)
                }
                None => "--:--".to_string(),
            },
            MissionStatKind::Bonus => format!("{bonus_val}"),
        };

        if let Some(align) = align {
            let left_native = (align.right_edge_native - text_w_native_px(&s)).max(0.0);
            node.left = Val::Px(left_native * align.overlay_scale);
        }

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

        tally.shown_bonus = tally.target_bonus;
        tally.pause_steps = 0;

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
    current_level: Res<davelib::level::CurrentLevel>,
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

    tally.pause_steps = 0;

    let floor = current_level.0.floor_number();
    tally.target_bonus = compute_target_bonus(&score, floor);

    // Checkpoint behavior: show correct bonus immediately (counting comes next step)
    tally.shown_bonus = tally.target_bonus;

    tally.tick.reset();
}

pub fn tick_mission_success_tally(
    time: Res<Time>,
    win: Res<LevelComplete>,
    mut tally: ResMut<MissionSuccessTally>,
    mut sfx: MessageWriter<PlaySfx>,

    // Local-only state (so we don't have to change MissionSuccessTally's struct again)
    mut pause_steps_local: Local<i32>,
    mut pending_sound_local: Local<Option<SfxKind>>,
    mut pending_next_phase_local: Local<Option<MissionSuccessPhase>>,
    mut pending_post_pause_local: Local<i32>,

    // NEW: one-time “screen just came up” pause gate
    mut did_initial_pause_local: Local<bool>,
) {
    // If we're not tallying, reset local state
    if !win.0 || !tally.active {
        *pause_steps_local = 0;
        *pending_sound_local = None;
        *pending_next_phase_local = None;
        *pending_post_pause_local = 0;
        *did_initial_pause_local = false;
        return;
    }

    // --- Step timing derived from YOUR timer ---
    // (This is the key fix: no more hard-coded "45Hz mental model")
    let step_dt = tally.tick.duration().as_secs_f32();
    if step_dt <= 0.0 {
        return;
    }

    fn secs_to_steps(secs: f32, step_dt: f32) -> i32 {
        ((secs / step_dt).round() as i32).max(1)
    }

    // --- Cadence tuneables (seconds) ---
    // Brief pause when the score screen first appears
    const INITIAL_SCREEN_PAUSE_SECS: f32 = 0.30;

    // Tiny gap so the last tick SFX doesn't stomp the stinger/confirm.
    const PRE_END_SOUND_GAP_SECS: f32 = 0.06;

    // This is the big one you asked for:
    // pause AFTER the end sound before starting the next stat.
    // (Set to 1.0 or 2.0 to taste.)
    const BETWEEN_STATS_PAUSE_SECS: f32 = 1.25;

    // If you want the pause to wait for the sound to *finish*, you need an estimate.
    // Keep these as “tune knobs”. If your assets differ, adjust.
    const SFX_PERCENT100_EST_SECS: f32 = 1.22;
    const SFX_NO_BONUS_EST_SECS: f32 = 0.76;
    // Confirm is typically short; we just pause the “between stats” time after it.
    const SFX_CONFIRM_EST_SECS: f32 = 0.10;

    let initial_pause_steps = secs_to_steps(INITIAL_SCREEN_PAUSE_SECS, step_dt);
    let pre_end_gap_steps = secs_to_steps(PRE_END_SOUND_GAP_SECS, step_dt);

    let post_percent100_steps =
        secs_to_steps(SFX_PERCENT100_EST_SECS + BETWEEN_STATS_PAUSE_SECS, step_dt);
    let post_no_bonus_steps =
        secs_to_steps(SFX_NO_BONUS_EST_SECS + BETWEEN_STATS_PAUSE_SECS, step_dt);
    let post_confirm_steps =
        secs_to_steps(SFX_CONFIRM_EST_SECS + BETWEEN_STATS_PAUSE_SECS, step_dt);

    // DOS-feel: keep your existing 2% step size.
    const PCT_STEP: i32 = 2;

    fn crossed_multiple(prev: i32, new: i32, n: i32) -> bool {
        if n <= 0 || new <= prev {
            return false;
        }
        (prev / n) < (new / n)
    }

    let mut emitted_this_call = false;
    let emit_once = |kind: SfxKind, sfx: &mut MessageWriter<PlaySfx>, emitted: &mut bool| {
        if *emitted {
            return;
        }
        sfx.write(PlaySfx {
            kind,
            pos: Vec3::ZERO, // UI sounds configured non-spatial
        });
        *emitted = true;
    };

    // Deref Locals ONCE so we don't fight &mut Local<T> typing inside helpers
    let pause_steps: &mut i32 = &mut *pause_steps_local;
    let pending_sound: &mut Option<SfxKind> = &mut *pending_sound_local;
    let pending_next_phase: &mut Option<MissionSuccessPhase> = &mut *pending_next_phase_local;
    let pending_post_pause: &mut i32 = &mut *pending_post_pause_local;

    // One-time initial pause when tally starts (screen appears)
    if !*did_initial_pause_local && *pause_steps == 0 {
        *pause_steps = initial_pause_steps;
        *did_initial_pause_local = true;
    }

    // End-of-phase scheduler:
    // - wait a tiny gap
    // - play the completion sound
    // - wait long enough for the sound + between-stats pause
    // - advance phase
    let schedule_end = |ratio: i32,
                        next: MissionSuccessPhase,
                        pause_steps: &mut i32,
                        pending_sound: &mut Option<SfxKind>,
                        pending_post_pause: &mut i32,
                        pending_next_phase: &mut Option<MissionSuccessPhase>| {
        let (sound, post_steps) = if ratio == 100 {
            (SfxKind::IntermissionPercent100, post_percent100_steps)
        } else if ratio == 0 {
            (SfxKind::IntermissionNoBonus, post_no_bonus_steps)
        } else {
            (SfxKind::IntermissionConfirm, post_confirm_steps)
        };

        *pending_next_phase = Some(next);

        // Always do a tiny gap so the last tick doesn't trample the stinger/confirm.
        *pause_steps = pre_end_gap_steps;
        *pending_sound = Some(sound);
        *pending_post_pause = post_steps;
    };

    // Drive from the existing timer (your chosen Hz)
    tally.tick.tick(time.delta());
    let mut steps = tally.tick.times_finished_this_tick();
    if steps == 0 {
        return;
    }

    // Avoid huge dt spikes (alt-tab) causing lots of roll in one frame
    steps = steps.min(6);

    for _ in 0..(steps as i32) {
        // Pause gate: during pause, nothing advances.
        if *pause_steps > 0 {
            *pause_steps -= 1;

            if *pause_steps == 0 {
                // If an end sound is queued after pre-gap, play it now then start post-pause.
                if let Some(k) = pending_sound.take() {
                    emit_once(k, &mut sfx, &mut emitted_this_call);

                    if *pending_post_pause > 0 {
                        *pause_steps = *pending_post_pause;
                        *pending_post_pause = 0;
                    }
                } else {
                    // This was the post-sound pause; now advance to next phase.
                    if let Some(next) = pending_next_phase.take() {
                        tally.phase = next;
                        if tally.phase == MissionSuccessPhase::Done {
                            tally.active = false;
                        }
                    }
                }
            }

            continue;
        }

        // IMPORTANT: time/par should already be displayed, not “counted” here.
        // (So: no time roll logic in this system.)

        match tally.phase {
            MissionSuccessPhase::Kill => {
                if tally.shown_kill < tally.target_kill {
                    let prev = tally.shown_kill;
                    tally.shown_kill = (tally.shown_kill + PCT_STEP).min(tally.target_kill);

                    // Keep your current tick rule (10% boundaries)
                    if crossed_multiple(prev, tally.shown_kill, 10) {
                        emit_once(SfxKind::IntermissionTick, &mut sfx, &mut emitted_this_call);
                    }
                } else {
                    schedule_end(
                        tally.target_kill,
                        MissionSuccessPhase::Secret,
                        pause_steps,
                        pending_sound,
                        pending_post_pause,
                        pending_next_phase,
                    );
                }
            }

            MissionSuccessPhase::Secret => {
                if tally.shown_secret < tally.target_secret {
                    let prev = tally.shown_secret;
                    tally.shown_secret = (tally.shown_secret + PCT_STEP).min(tally.target_secret);

                    if crossed_multiple(prev, tally.shown_secret, 10) {
                        emit_once(SfxKind::IntermissionTick, &mut sfx, &mut emitted_this_call);
                    }
                } else {
                    schedule_end(
                        tally.target_secret,
                        MissionSuccessPhase::Treasure,
                        pause_steps,
                        pending_sound,
                        pending_post_pause,
                        pending_next_phase,
                    );
                }
            }

            MissionSuccessPhase::Treasure => {
                if tally.shown_treasure < tally.target_treasure {
                    let prev = tally.shown_treasure;
                    tally.shown_treasure =
                        (tally.shown_treasure + PCT_STEP).min(tally.target_treasure);

                    if crossed_multiple(prev, tally.shown_treasure, 10) {
                        emit_once(SfxKind::IntermissionTick, &mut sfx, &mut emitted_this_call);
                    }
                } else {
                    schedule_end(
                        tally.target_treasure,
                        MissionSuccessPhase::Done,
                        pause_steps,
                        pending_sound,
                        pending_post_pause,
                        pending_next_phase,
                    );
                }
            }

            MissionSuccessPhase::Done => {
                tally.active = false;
                break;
            }
        }
    }
}
