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
    pub bonus_applied: bool,

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
            bonus_applied: false,

            tick: Timer::from_seconds(1.0 / 120.0, TimerMode::Repeating),
        }
    }
}

#[derive(Resource)]
pub struct ElevatorExitDelay {
    pub active: bool,
    pub timer: Timer,
}

impl Default for ElevatorExitDelay {
    fn default() -> Self {
        Self {
            active: false,
            // Tune: start small. This is just “long enough to see the flip”.
            timer: Timer::from_seconds(0.35, TimerMode::Once),
        }
    }
}

/// When the player hits the elevator switch:
/// - we lock immediately
/// - we flip + rebuild immediately
/// - we delay showing the intermission overlay (win.0) until this timer completes
pub fn tick_elevator_exit_delay(
    time: Res<Time>,
    mut delay: ResMut<ElevatorExitDelay>,
    mut win: ResMut<LevelComplete>,
    mut lock: ResMut<PlayerControlLock>,
) {
    // If we’re not waiting to show the win screen, nothing to do.
    if !delay.active {
        return;
    }

    // If win is already up, clear pending state (safety).
    if win.0 {
        delay.active = false;
        return;
    }

    // Keep gameplay frozen during the delay.
    lock.0 = true;

    delay.timer.tick(time.delta());

    if delay.timer.is_finished() && delay.timer.just_finished() {
        // NOW show the intermission overlay.
        win.0 = true;
        delay.active = false;
    }
}

pub fn use_elevator_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut lock: ResMut<PlayerControlLock>,
    win: ResMut<LevelComplete>,
    mut grid: ResMut<MapGrid>,
    q_player: Query<&Transform, With<Player>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut rebuild: MessageWriter<RebuildWalls>,
    mut music_mode: ResMut<davelib::audio::MusicMode>,
    mut exit_delay: ResMut<ElevatorExitDelay>,
) {
    // If gameplay is already locked, or win screen already up, or we're already delaying, do nothing.
    if lock.0 || win.0 || exit_delay.active {
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

    // Rebuild Wall Faces so Flipped Wall ID is Visible
    rebuild.write(RebuildWalls { skip: None });

    // Play Elevator Switch Sound
    sfx.write(PlaySfx {
        kind: SfxKind::ElevatorSwitch,
        pos: Vec3::new(target.x as f32, 0.6, target.y as f32),
    });

    // Freeze gameplay immediately
    lock.0 = true;

    // Hard cut to end level music immediately
    music_mode.0 = davelib::audio::MusicModeKind::LevelEnd;

    // IMPORTANT: Delay showing the score screen so the flip can render
    exit_delay.active = true;
    exit_delay.timer.reset();

    // NOTE: We DO NOT set win.0 here anymore
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
    _buttons: Res<ButtonInput<MouseButton>>,
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
        || keys.just_pressed(KeyCode::Space);

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

pub fn apply_mission_success_bonus_to_player_score_once(
    win: Res<LevelComplete>,
    mut tally: ResMut<MissionSuccessTally>,
    mut hud: ResMut<crate::ui::HudState>,
) {
    // Only during the intermission screen
    if !win.0 {
        return;
    }

    // Only once
    if tally.bonus_applied {
        return;
    }

    // Only after tallying is complete (either naturally, or via skip-to-end)
    if tally.phase != MissionSuccessPhase::Done {
        return;
    }

    let add = tally.target_bonus.max(0);
    hud.score = hud.score.saturating_add(add);
    tally.bonus_applied = true;

    info!("Mission Success: applied bonus {} (new score: {})", add, hud.score);
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

    tally.bonus_applied = false;
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

    let mut base_bonus = tally.target_bonus;
    if tally.target_kill == 100 {
        base_bonus = base_bonus.saturating_sub(PERCENT100AMT);
    }
    if tally.target_secret == 100 {
        base_bonus = base_bonus.saturating_sub(PERCENT100AMT);
    }
    if tally.target_treasure == 100 {
        base_bonus = base_bonus.saturating_sub(PERCENT100AMT);
    }
    tally.shown_bonus = base_bonus.max(0);

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

    // NEW: bonus delta to apply right after the queued stinger plays
    mut pending_bonus_local: Local<i32>,

    // One-time init so TIME is immediately up + intro pause happens once
    mut did_init_local: Local<bool>,
) {
    // If we're not tallying, reset local state
    if !win.0 || !tally.active {
        *pause_steps_local = 0;
        *pending_sound_local = None;
        *pending_next_phase_local = None;
        *pending_post_pause_local = 0;
        *pending_bonus_local = 0;
        *did_init_local = false;
        return;
    }

    // --- Tick-rate-aware pause tuning ---
    let step_dt = tally.tick.duration().as_secs_f32().max(0.000_1);
    let secs_to_steps = |secs: f32| -> i32 {
        if secs <= 0.0 {
            0
        } else {
            (secs / step_dt).ceil() as i32
        }
    };

    // --- Tuneables (in seconds) ---
    const INTRO_PAUSE_SECS: f32 = 0.85;
    const PRE_STINGER_GAP_SECS: f32 = 0.08;
    const BETWEEN_PHASE_PAUSE_SECS: f32 = 1.10;
    const POST_PERCENT100_PAD_SECS: f32 = 0.20;
    const POST_NO_BONUS_PAD_SECS: f32 = 0.15;

    let intro_steps = secs_to_steps(INTRO_PAUSE_SECS);
    let pre_stinger_steps = secs_to_steps(PRE_STINGER_GAP_SECS);
    let between_phase_steps = secs_to_steps(BETWEEN_PHASE_PAUSE_SECS);
    let post_percent100_steps = secs_to_steps(BETWEEN_PHASE_PAUSE_SECS + POST_PERCENT100_PAD_SECS);
    let post_no_bonus_steps = secs_to_steps(BETWEEN_PHASE_PAUSE_SECS + POST_NO_BONUS_PAD_SECS);

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
            pos: Vec3::ZERO,
        });
        *emitted = true;
    };

    // Deref Locals ONCE
    let pause_steps: &mut i32 = &mut *pause_steps_local;
    let pending_sound: &mut Option<SfxKind> = &mut *pending_sound_local;
    let pending_next_phase: &mut Option<MissionSuccessPhase> = &mut *pending_next_phase_local;
    let pending_post_pause: &mut i32 = &mut *pending_post_pause_local;
    let pending_bonus: &mut i32 = &mut *pending_bonus_local;

    // One-time init:
    // - TIME is immediately up (no counting)
    // - Intro pause before first stat starts
    if !*did_init_local {
        tally.shown_time_secs = tally.target_time_secs;
        tally.time_step_accum = 0;

        if intro_steps > 0 {
            *pause_steps = intro_steps;
        }

        *did_init_local = true;
    }

    let schedule_end = |ratio: i32,
                        next: MissionSuccessPhase,
                        pause_steps: &mut i32,
                        pending_sound: &mut Option<SfxKind>,
                        pending_post_pause: &mut i32,
                        pending_next_phase: &mut Option<MissionSuccessPhase>,
                        pending_bonus: &mut i32,
                        sfx: &mut MessageWriter<PlaySfx>,
                        emitted: &mut bool| {
        // Reset any previously queued bonus delta
        *pending_bonus = 0;

        // Choose completion sound based on final ratio
        let (sound, pre_steps, post_steps) = if ratio == 100 {
            // Queue +10,000 to be applied right after the 100% stinger plays
            *pending_bonus = PERCENT100AMT;

            (SfxKind::IntermissionPercent100, pre_stinger_steps, post_percent100_steps)
        } else if ratio == 0 {
            (SfxKind::IntermissionNoBonus, pre_stinger_steps, post_no_bonus_steps)
        } else {
            (SfxKind::IntermissionConfirm, 0, between_phase_steps)
        };

        *pending_next_phase = Some(next);

        if pre_steps > 0 {
            *pause_steps = pre_steps;
            *pending_sound = Some(sound);
            *pending_post_pause = post_steps.max(1);
        } else {
            emit_once(sound, sfx, emitted);
            *pause_steps = post_steps.max(1);
            *pending_sound = None;
            *pending_post_pause = 0;
        }
    };

    // Drive from the existing timer
    tally.tick.tick(time.delta());
    let mut steps = tally.tick.times_finished_this_tick();
    if steps == 0 {
        return;
    }

    steps = steps.min(6);

    for _ in 0..(steps as i32) {
        // Pause gate
        if *pause_steps > 0 {
            *pause_steps -= 1;

            if *pause_steps == 0 {
                // If a stinger is queued after a pre-pause, play it now then start post-pause.
                if let Some(k) = pending_sound.take() {
                    emit_once(k, &mut sfx, &mut emitted_this_call);

                    // NEW: after playing the 100% stinger, add its bonus chunk now
                    if *pending_bonus > 0 {
                        tally.shown_bonus = tally
                            .shown_bonus
                            .saturating_add(*pending_bonus)
                            .min(tally.target_bonus);
                        *pending_bonus = 0;
                    }

                    if *pending_post_pause > 0 {
                        *pause_steps = (*pending_post_pause).max(1);
                        *pending_post_pause = 0;
                    }
                } else {
                    // Post-sound pause ended; advance to next phase.
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

        match tally.phase {
            MissionSuccessPhase::Kill => {
                if tally.shown_kill < tally.target_kill {
                    let prev = tally.shown_kill;
                    tally.shown_kill = (tally.shown_kill + PCT_STEP).min(tally.target_kill);

                    if tally.shown_kill < tally.target_kill && crossed_multiple(prev, tally.shown_kill, 10) {
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
                        pending_bonus,
                        &mut sfx,
                        &mut emitted_this_call,
                    );
                }
            }

            MissionSuccessPhase::Secret => {
                if tally.shown_secret < tally.target_secret {
                    let prev = tally.shown_secret;
                    tally.shown_secret = (tally.shown_secret + PCT_STEP).min(tally.target_secret);

                    if tally.shown_secret < tally.target_secret && crossed_multiple(prev, tally.shown_secret, 10) {
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
                        pending_bonus,
                        &mut sfx,
                        &mut emitted_this_call,
                    );
                }
            }

            MissionSuccessPhase::Treasure => {
                if tally.shown_treasure < tally.target_treasure {
                    let prev = tally.shown_treasure;
                    tally.shown_treasure = (tally.shown_treasure + PCT_STEP).min(tally.target_treasure);

                    if tally.shown_treasure < tally.target_treasure && crossed_multiple(prev, tally.shown_treasure, 10) {
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
                        pending_bonus,
                        &mut sfx,
                        &mut emitted_this_call,
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
