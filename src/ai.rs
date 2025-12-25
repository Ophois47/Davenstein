/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use std::collections::{HashSet, HashMap};

use crate::actors::{Dead, OccupiesTile};
use crate::audio::{PlaySfx, SfxKind};
use crate::enemies::{Dir8, EnemyKind, Guard};
use crate::map::{DoorState, DoorTile, MapGrid, Tile};
use crate::player::Player;

const AI_TIC_SECS: f32 = 1.0 / 70.0;
const DOOR_OPEN_SECS: f32 = 4.5;
const GUARD_CHASE_SPEED_TPS: f32 = 1.6;
const CLAIM_TILE_EARLY: bool = true;

#[derive(Resource, Debug, Default)]
pub struct AiTicker {
    accum: f32,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct EnemyFire {
    pub kind: EnemyKind,
    pub damage: i32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyAi {
    pub state: EnemyAiState,
    pub last_step: IVec2,
}

impl Default for EnemyAi {
    fn default() -> Self {
        Self {
            state: EnemyAiState::Stand,
            last_step: IVec2::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyAiState {
    Stand,
    Chase,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyMove {
    pub target: Vec3,
    pub speed_tps: f32,
}

#[allow(dead_code)]
enum ChasePick {
    MoveTo(IVec2),
    OpenDoor(IVec2),
    None,
}

#[allow(dead_code)]
fn pick_chase_step(
    grid: &MapGrid,
    occupied: &std::collections::HashSet<IVec2>,
    my_tile: IVec2,
    player_tile: IVec2,
    last_step: IVec2,
) -> ChasePick {
    let dx = player_tile.x - my_tile.x;
    let dz = player_tile.y - my_tile.y;

    // Desired directions toward player (4-way)
    let xdir = if dx > 0 { 1 } else if dx < 0 { -1 } else { 0 };
    let zdir = if dz > 0 { 1 } else if dz < 0 { -1 } else { 0 };

    let primary_x = dx.abs() >= dz.abs();

    // Candidate steps in Wolf-ish priority order
    let mut candidates: [IVec2; 6] = [
        IVec2::ZERO,
        IVec2::ZERO,
        IVec2::ZERO,
        IVec2::ZERO,
        IVec2::ZERO,
        IVec2::ZERO,
    ];

    let toward_x = IVec2::new(xdir, 0);
    let toward_z = IVec2::new(0, zdir);

    // Two “toward player” axes first
    if primary_x {
        candidates[0] = toward_x;
        candidates[1] = toward_z;
    } else {
        candidates[0] = toward_z;
        candidates[1] = toward_x;
    }

    // Then perpendicular fallbacks (try to go around)
    candidates[2] = IVec2::new(0, 1);
    candidates[3] = IVec2::new(0, -1);
    candidates[4] = IVec2::new(1, 0);
    candidates[5] = IVec2::new(-1, 0);

    let reverse = -last_step;

    for step in candidates {
        if step == IVec2::ZERO {
            continue;
        }
        // Avoid Immediate Reversing Unless Forced
        if last_step != IVec2::ZERO && step == reverse {
            continue;
        }

        let dest = my_tile + step;

        // Don't Step Into Occupied Tiles or Player Tile
        if dest == player_tile || occupied.contains(&dest) {
            continue;
        }

        let Some(t) = tile_at(grid, dest) else { continue; };

        match t {
            Tile::Empty | Tile::DoorOpen => return ChasePick::MoveTo(dest),
            Tile::DoorClosed => return ChasePick::OpenDoor(dest),
            _ => {}
        }
    }

    // If nothing worked, allow reverse as last resort
    if last_step != IVec2::ZERO {
        let dest = my_tile + reverse;
        if dest != player_tile && !occupied.contains(&dest) {
            if let Some(t) = tile_at(grid, dest) {
                match t {
                    Tile::Empty | Tile::DoorOpen => return ChasePick::MoveTo(dest),
                    Tile::DoorClosed => return ChasePick::OpenDoor(dest),
                    _ => {}
                }
            }
        }
    }

    ChasePick::None
}

fn attach_guard_ai(mut commands: Commands, q_new: Query<Entity, (Added<Guard>, Without<EnemyAi>)>) {
    for e in q_new.iter() {
        commands.entity(e).insert(EnemyAi::default());
    }
}

fn world_to_tile_xz(xz: Vec2) -> IVec2 {
    IVec2::new((xz.x + 0.5).floor() as i32, (xz.y + 0.5).floor() as i32)
}

fn tile_at(grid: &MapGrid, t: IVec2) -> Option<Tile> {
    if t.x < 0 || t.y < 0 {
        return None;
    }
    let x = t.x as usize;
    let z = t.y as usize;
    if x >= grid.width || z >= grid.height {
        return None;
    }
    Some(grid.tile(x, z))
}

fn has_line_of_sight(grid: &MapGrid, from: IVec2, to: IVec2) -> bool {
    if from == to {
        return true;
    }

    // Ray from tile center to tile center, using the same N+0.5 boundary scheme as hitscan/collision.
    let origin = Vec2::new(from.x as f32, from.y as f32);
    let target = Vec2::new(to.x as f32, to.y as f32);

    let mut dir = target - origin;
    let max_dist = dir.length();
    if max_dist < 1e-6 {
        return true;
    }
    dir /= max_dist;

    let dx = dir.x;
    let dz = dir.y;

    const EPS: f32 = 1e-8;

    // Tile boundaries at N+0.5
    let px = origin.x + 0.5;
    let pz = origin.y + 0.5;

    let mut ix = px.floor() as i32;
    let mut iz = pz.floor() as i32;

    let step_x = if dx > 0.0 { 1 } else { -1 };
    let step_z = if dz > 0.0 { 1 } else { -1 };

    let t_delta_x = if dx.abs() < EPS { f32::INFINITY } else { 1.0 / dx.abs() };
    let t_delta_z = if dz.abs() < EPS { f32::INFINITY } else { 1.0 / dz.abs() };

    let mut t_max_x = if dx.abs() < EPS {
        f32::INFINITY
    } else if dx > 0.0 {
        ((ix as f32 + 1.0) - px) / dx
    } else {
        (px - ix as f32) / (-dx)
    };

    let mut t_max_z = if dz.abs() < EPS {
        f32::INFINITY
    } else if dz > 0.0 {
        ((iz as f32 + 1.0) - pz) / dz
    } else {
        (pz - iz as f32) / (-dz)
    };

    loop {
        let dist = if t_max_x < t_max_z {
            ix += step_x;
            let d = t_max_x;
            t_max_x += t_delta_x;
            d
        } else {
            iz += step_z;
            let d = t_max_z;
            t_max_z += t_delta_z;
            d
        };

        if dist > max_dist {
            return true;
        }

        // Bounds -> treat as blocked LOS
        if ix < 0 || iz < 0 || ix >= grid.width as i32 || iz >= grid.height as i32 {
            return false;
        }

        // If we’ve entered the target tile, LOS is clear.
        if ix == to.x && iz == to.y {
            return true;
        }

        let tile = grid.tile(ix as usize, iz as usize);
        if matches!(tile, Tile::Wall | Tile::DoorClosed) {
            return false;
        }
    }
}

fn dir8_from_step(step: IVec2) -> Dir8 {
    // Match Enemies.rs::quantize_view8 Convention:
    // Dir8(0)=+Z, Dir8(2)=+X, Dir8(4)=-Z, Dir8(6)=-X
    match (step.x, step.y) {
        (0, 1) => Dir8(0),  // +Z
        (1, 0) => Dir8(2),  // +X
        (0, -1) => Dir8(4), // -Z
        (-1, 0) => Dir8(6), // -X
        _ => Dir8(0),
    }
}

fn dir8_towards(from: IVec2, to: IVec2) -> Dir8 {
    let d = to - from;
    if d == IVec2::ZERO {
        return Dir8(0);
    }

    // 0 rad = +Z (positive "y" in grid coords), matches your yaw usage elsewhere
    let ang = (d.x as f32).atan2(d.y as f32);

    // Quantize into 8 octants (0..7), with 0 = +Z, 2 = +X, 4 = -Z, 6 = -X
    let step = std::f32::consts::FRAC_PI_4; // 45°
    let mut oct = ((ang + step * 0.5) / step).floor() as i32;
    oct = ((oct % 8) + 8) % 8;

    Dir8(oct as u8)
}

fn try_open_door_at(
    door_tile: IVec2,
    q_doors: &mut Query<(&DoorTile, &mut DoorState, &GlobalTransform)>,
    sfx: &mut MessageWriter<PlaySfx>,
) {
    for (dt, mut ds, gt) in q_doors.iter_mut() {
        if dt.0 != door_tile {
            continue;
        }

        ds.open_timer = DOOR_OPEN_SECS;

        if !ds.want_open {
            ds.want_open = true;
            sfx.write(PlaySfx {
                kind: SfxKind::DoorOpen,
                pos: gt.translation(),
            });
        }

        break;
    }
}

#[derive(Debug)]
struct AreaMap {
    w: usize,
    h: usize,
    ids: Vec<i32>, // -1 = solid/unassigned
}

impl AreaMap {
    fn compute(grid: &MapGrid) -> Self {
        let w = grid.width;
        let h = grid.height;

        let mut ids = vec![-1; w * h];
        let mut next_id: i32 = 0;

        let passable = |t: Tile| matches!(t, Tile::Empty | Tile::DoorOpen);

        for z in 0..h {
            for x in 0..w {
                let idx = z * w + x;
                if ids[idx] != -1 {
                    continue;
                }

                let t = grid.tile(x, z);
                if !passable(t) {
                    continue;
                }

                // flood fill
                let mut stack = vec![IVec2::new(x as i32, z as i32)];
                ids[idx] = next_id;

                while let Some(p) = stack.pop() {
                    let n4 = [
                        IVec2::new(p.x + 1, p.y),
                        IVec2::new(p.x - 1, p.y),
                        IVec2::new(p.x, p.y + 1),
                        IVec2::new(p.x, p.y - 1),
                    ];

                    for n in n4 {
                        if n.x < 0 || n.y < 0 || n.x as usize >= w || n.y as usize >= h {
                            continue;
                        }
                        let ni = n.y as usize * w + n.x as usize;
                        if ids[ni] != -1 {
                            continue;
                        }

                        let nt = grid.tile(n.x as usize, n.y as usize);
                        if !passable(nt) {
                            continue;
                        }

                        ids[ni] = next_id;
                        stack.push(n);
                    }
                }

                next_id += 1;
            }
        }

        Self { w, h, ids }
    }

    fn id(&self, t: IVec2) -> Option<i32> {
        if t.x < 0 || t.y < 0 || t.x as usize >= self.w || t.y as usize >= self.h {
            return None;
        }
        let id = self.ids[t.y as usize * self.w + t.x as usize];
        if id < 0 { None } else { Some(id) }
    }
}

pub fn enemy_ai_tick(
    mut commands: Commands,
    time: Res<Time>,
    mut ticker: ResMut<AiTicker>,
    grid: Res<MapGrid>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &GlobalTransform)>,
    mut sfx: MessageWriter<PlaySfx>,
    mut enemy_fire: MessageWriter<EnemyFire>,
    mut shoot_cd: Local<HashMap<Entity, f32>>,
    mut alerted: Local<HashSet<Entity>>,
    mut q: ParamSet<(
        Query<&OccupiesTile, (With<EnemyKind>, Without<Dead>)>,
        Query<
            (
                Entity,
                &EnemyKind,
                &mut EnemyAi,
                &mut OccupiesTile,
                &mut Dir8,
                &Transform,
                Option<&EnemyMove>,
            ),
            (With<EnemyKind>, Without<Player>, Without<Dead>),
        >,
    )>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();
    let player_tile = world_to_tile_xz(Vec2::new(player_pos.x, player_pos.z));

    // Snapshot occupied tiles (alive enemies only).
    let mut occupied: HashSet<IVec2> = HashSet::new();
    for ot in q.p0().iter() {
        occupied.insert(ot.0);
    }

    // Cooldowns tick down every frame
    let dt = time.delta_secs();
    shoot_cd.retain(|_, t| {
        *t -= dt;
        *t > 0.0
    });

    ticker.accum += dt;

    while ticker.accum >= AI_TIC_SECS {
        ticker.accum -= AI_TIC_SECS;

        let areas = AreaMap::compute(&grid);
        let player_area = areas.id(player_tile);

        for (e, kind, mut ai, mut occ, mut dir8, tf, moving) in q.p1().iter_mut() {
            let speed = match kind {
                EnemyKind::Guard => GUARD_CHASE_SPEED_TPS,
            };

            let my_tile = occ.0;

            // Acquire -> Chase (same "area" + LOS)
            if ai.state == EnemyAiState::Stand {
                let same_area = player_area.is_some() && areas.id(my_tile) == player_area;
                if same_area && has_line_of_sight(&grid, my_tile, player_tile) {
                    ai.state = EnemyAiState::Chase;

                    // one-time alert per enemy (without adding fields to EnemyAi)
                    if alerted.insert(e) {
                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyAlert(*kind),
                            pos: tf.translation,
                        });
                    }
                }
            }

            if ai.state != EnemyAiState::Chase {
                continue;
            }

            let current_dist =
                (player_tile.x - my_tile.x).abs() + (player_tile.y - my_tile.y).abs();

            // =========================
            // SHOOT LOGIC
            // =========================
            let same_area = player_area.is_some() && areas.id(my_tile) == player_area;
            let can_see = same_area && has_line_of_sight(&grid, my_tile, player_tile);

            // NOTE: no CHASE_MAX_SHOOT_DIST constant in your code; keep it simple for now.
            // Adjust this number later once you’re happy with behavior.
            let in_range = current_dist <= 6;

            if can_see && in_range {
                // Face the player for correct view selection / shooting visuals.
                *dir8 = dir8_towards(my_tile, player_tile);

                let cd = shoot_cd.get(&e).copied().unwrap_or(0.0);
                if cd <= 0.0 {
                    shoot_cd.insert(e, 0.8);

                    let dist = current_dist as f32;
                    let max_dist = 6.0;
                    let hit_chance = (1.0 - (dist / max_dist)).clamp(0.15, 0.75);

                    let damage = if rand::random::<f32>() < hit_chance { 10 } else { 0 };

                    enemy_fire.write(EnemyFire {
                        kind: *kind,
                        damage,
                    });

                    // Drive shooting animation via GuardShoot.timer (the real struct field)
                    commands.entity(e).insert(crate::enemies::GuardShoot {
                        timer: Timer::from_seconds(0.25, TimerMode::Once),
                    });

                    sfx.write(PlaySfx {
                        kind: SfxKind::EnemyShoot(*kind),
                        pos: tf.translation,
                    });

                    info!(
                        "Enemy {:?} fired: kind={:?} dist={} chance={:.2} dmg={}",
                        e, kind, current_dist, hit_chance, damage
                    );
                }
            }

            // If already moving, don’t pick a new chase step this tic.
            if moving.is_some() {
                continue;
            }

            // =========================
            // MOVE LOGIC
            // =========================
            let dirs = [
                IVec2::new(1, 0),
                IVec2::new(-1, 0),
                IVec2::new(0, 1),
                IVec2::new(0, -1),
            ];

            let mut best_move: Option<(IVec2, i32)> = None;
            let mut best_door: Option<(IVec2, i32)> = None;

            for step in dirs {
                let dest = my_tile + step;

                if dest == player_tile {
                    continue;
                }

                if occupied.contains(&dest) {
                    continue;
                }

                let Some(t) = tile_at(&grid, dest) else { continue; };

                let score =
                    (player_tile.x - dest.x).abs() + (player_tile.y - dest.y).abs();

                match t {
                    Tile::Empty | Tile::DoorOpen => {
                        if best_move.map(|(_, s)| score < s).unwrap_or(true) {
                            best_move = Some((dest, score));
                        }
                    }
                    Tile::DoorClosed => {
                        if best_door.map(|(_, s)| score < s).unwrap_or(true) {
                            best_door = Some((dest, score));
                        }
                    }
                    _ => {}
                }
            }

            if let Some((dest, score)) = best_move {
                if score <= current_dist {
                    let step = dest - my_tile;
                    *dir8 = dir8_from_step(step);

                    if CLAIM_TILE_EARLY {
                        occ.0 = dest;
                    }

                    let y = tf.translation.y;
                    let target = Vec3::new(dest.x as f32, y, dest.y as f32);

                    commands.entity(e).insert(EnemyMove {
                        target,
                        speed_tps: speed,
                    });

                    occupied.insert(dest);
                    if CLAIM_TILE_EARLY {
                        occupied.remove(&my_tile);
                    }
                }
            } else if let Some((door_tile, score)) = best_door {
                if score <= current_dist {
                    try_open_door_at(door_tile, &mut q_doors, &mut sfx);
                }
            }
        }
    }
}

fn enemy_ai_move(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &EnemyMove, &mut Transform), Without<Dead>>,
) {
    let dt = time.delta_secs();

    for (e, mv, mut tf) in q.iter_mut() {
        let mut to = mv.target - tf.translation;
        to.y = 0.0;

        let dist = to.length();
        if dist < 0.0001 {
            tf.translation.x = mv.target.x;
            tf.translation.z = mv.target.z;
            commands.entity(e).remove::<EnemyMove>();
            continue;
        }

        let step = mv.speed_tps * dt;
        if dist <= step {
            tf.translation.x = mv.target.x;
            tf.translation.z = mv.target.z;
            commands.entity(e).remove::<EnemyMove>();
        } else {
            let dir = to / dist;
            tf.translation += Vec3::new(dir.x * step, 0.0, dir.z * step);
        }
    }
}

pub struct EnemyAiPlugin;

impl Plugin for EnemyAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AiTicker>()
            .add_message::<EnemyFire>()
            .add_systems(Update, attach_guard_ai)
            .add_systems(FixedUpdate, (enemy_ai_tick, enemy_ai_move).chain());
    }
}
