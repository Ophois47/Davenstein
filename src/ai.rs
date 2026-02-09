/*
Davenstein - by David Petnick

CRITICAL BEVY LIMITATION DOCUMENTED:
====================================
Bevy has a hard limit on the number of parameters a system function can accept.
When this limit is exceeded, you get compiler errors like:
  "the trait `IntoSystem<(), (), _>` is not implemented for fn item"

The original `enemy_ai_tick` system had 17 parameters including a complex ParamSet,
which exceeded Bevy's limits and could not be registered as a system at all.

SOLUTION: Split large systems into multiple smaller systems that each handle
a specific responsibility. This keeps parameter counts manageable while maintaining
the same logic and execution order.

For this AI system, we split into:
1. enemy_ai_activation - Handle state transitions (Stand -> Patrol -> Chase)
2. enemy_ai_combat - Handle shooting, dog bites, burst fire
3. enemy_ai_movement - Handle pathfinding and movement scheduling

These run in sequence via .chain() to maintain the original execution order.
*/
/*
1) Bevy validates ECS borrows at schedule initialization time
   If a single system function has conflicting access to the same component type
   Example Query<&Transform, ...> plus Query<&mut Transform, ...> in one system
   Bevy will panic with error B0001 and the game will crash on startup
   Fix options are
   - Make queries disjoint with filters like Without<T>
   - Merge conflicting queries into a ParamSet and ensure you do not use them simultaneously
   Reference https://bevy.org/learn/errors/b0001

2) Compiler dead_code warnings are a real gameplay signal in ECS projects
   If a system like attach_enemy_ai is never used, it is not registered into any schedule
   That can silently break gameplay because queries stop matching any entities
   In this incident, enemies had EnemyKind but never received EnemyAi
   Result AI systems ran but affected zero enemies so nobody chased or shot

3) Commands are deferred
   Inserting components via Commands does not make them visible to later systems in the same schedule run
   chain() enforces execution order but does not auto-flush deferred Commands
   If same-tick visibility is required, add an apply_deferred boundary between producer and consumer
*/
use bevy::prelude::*;
use std::collections::{HashSet, HashMap};

use crate::actors::{Dead, OccupiesTile};
use crate::ai_patrol::{
    Patrol,
    patrol_dir_from_plane1,
    patrol_step_8way,
    spawn_dir_and_patrol_for_kind,
};
use crate::audio::{PlaySfx, SfxKind};
use crate::decorations::SolidStatics;
use crate::enemies::{
    Dir8,
    EnemyTunings,
    EnemyKind,
};
use crate::map::{
    DoorState,
    DoorTile,
    MapGrid,
    Tile,
};
use crate::player::{
    Player,
    PlayerControlLock,
    PlayerDeathLatch,
};

const AI_TIC_SECS: f32 = 1.0 / 70.0;
const DOOR_OPEN_SECS: f32 = 4.5;
const CLAIM_TILE_EARLY: bool = true;

// Shooting constants
const GUARD_SHOOT_MAX_DIST_TILES: i32 = 7;
const GUARD_SHOOT_PAUSE_SECS: f32 = 0.25;
const MUTANT_SHOOT_PAUSE_SECS: f32 = 0.15;
const LOS_FIRST_SHOT_DELAY_SECS: f32 = 0.02;
const GUARD_SHOOT_COOLDOWN_SECS: f32 = 0.55;
const GUARD_SHOOT_TOTAL_SECS: f32 = GUARD_SHOOT_PAUSE_SECS + GUARD_SHOOT_COOLDOWN_SECS;

#[derive(Resource, Debug, Default)]
pub struct AiTicker {
    pub accum: f32,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct EnemyFire {
    pub kind: EnemyKind,
    pub damage: i32,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct EnemyFireballShot {
    pub origin: Vec3,
    pub dir: Vec3,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct EnemySyringeShot {
    pub origin: Vec3,
    pub dir: Vec3,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct EnemyRocketShot {
    pub origin: Vec3,
    pub dir: Vec3,
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
    Patrol,
    Chase,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyMove {
    pub target: Vec3,
    pub speed_tps: f32,
}

// Temporary component to communicate Dir8 changes from movement system
#[derive(Component, Debug, Clone, Copy)]
struct PendingDir8(Dir8);

#[derive(Clone, Copy, Debug)]
pub struct BurstFire {
    shots_left: u8,
    every_tics: u32,
    next_tics: u32,
}

// Resource to share data between the split AI systems
#[derive(Resource, Default)]
struct AiSharedData {
    occupied: HashSet<IVec2>,
    scheduled_move: HashSet<Entity>,
    dist_map: Vec<i32>,
    player_tile: IVec2,
    player_pos: Vec3,
    player_area: Option<i32>,
}

#[allow(dead_code)]
fn burst_profile(kind: EnemyKind) -> Option<(u8, u32, f32)> {
    match kind {
        EnemyKind::Ss => Some((5, 6, 0.35)),
        EnemyKind::Hans | EnemyKind::Gretel | EnemyKind::Hitler | EnemyKind::MechaHitler => Some((8, 4, 0.45)),
        _ => None,
    }
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
    solid: &SolidStatics,
    occupied: &std::collections::HashSet<IVec2>,
    my_tile: IVec2,
    player_tile: IVec2,
    last_step: IVec2,
) -> ChasePick {
    let dx = player_tile.x - my_tile.x;
    let dz = player_tile.y - my_tile.y;

    let xdir = if dx > 0 { 1 } else if dx < 0 { -1 } else { 0 };
    let zdir = if dz > 0 { 1 } else if dz < 0 { -1 } else { 0 };

    let primary_x = dx.abs() >= dz.abs();

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

    if primary_x {
        candidates[0] = toward_x;
        candidates[1] = toward_z;
    } else {
        candidates[0] = toward_z;
        candidates[1] = toward_x;
    }

    candidates[2] = IVec2::new(0, 1);
    candidates[3] = IVec2::new(0, -1);
    candidates[4] = IVec2::new(1, 0);
    candidates[5] = IVec2::new(-1, 0);

    let reverse = -last_step;

    for step in candidates {
        if step == IVec2::ZERO {
            continue;
        }
        if last_step != IVec2::ZERO && step == reverse {
            continue;
        }

        let dest = my_tile + step;

        if dest == player_tile || occupied.contains(&dest) {
            continue;
        }

        let Some(t) = tile_at(grid, dest) else { continue; };

        if solid.is_solid(dest.x, dest.y) {
            continue;
        }

        match t {
            Tile::Empty | Tile::DoorOpen => return ChasePick::MoveTo(dest),
            Tile::DoorClosed => return ChasePick::OpenDoor(dest),
            _ => {}
        }
    }

    if last_step != IVec2::ZERO {
        let dest = my_tile + reverse;
        if dest != player_tile && !occupied.contains(&dest) && !solid.is_solid(dest.x, dest.y) {
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

fn attach_enemy_ai(
    mut commands: Commands,
    grid: Res<MapGrid>,
    wolf_plane1: Res<crate::level::WolfPlane1>,
    mut q_new: Query<(Entity, &EnemyKind, &OccupiesTile, &mut Dir8), (Added<EnemyKind>, Without<EnemyAi>)>,
) {
    let w = grid.width as i32;

    for (e, kind, occ, mut dir8) in q_new.iter_mut() {
        let mut state = EnemyAiState::Stand;

        let idx = (occ.0.y * w + occ.0.x) as usize;
        if let Some(code) = wolf_plane1.0.get(idx).copied() {
            if let Some((d, is_patrol)) = spawn_dir_and_patrol_for_kind(*kind, code) {
                *dir8 = d;
                if is_patrol {
                    state = EnemyAiState::Patrol;
                    commands.entity(e).insert(Patrol::default());
                }
            }
        }

        commands.entity(e).insert(EnemyAi {
            state,
            last_step: IVec2::ZERO,
        });
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

fn has_line_of_sight(grid: &MapGrid, _solid: &SolidStatics, from: IVec2, to: IVec2) -> bool {
    if from == to {
        return true;
    }

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
    const EPS_T: f32 = 1e-6;

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
        let dist = if t_max_x + EPS_T < t_max_z {
            ix += step_x;
            let d = t_max_x;
            t_max_x += t_delta_x;
            d
        } else if t_max_z + EPS_T < t_max_x {
            iz += step_z;
            let d = t_max_z;
            t_max_z += t_delta_z;
            d
        } else {
            let next_ix = ix + step_x;
            let next_iz = iz + step_z;

            let d = t_max_x;
            t_max_x += t_delta_x;
            t_max_z += t_delta_z;

            for (cx, cz) in [(next_ix, iz), (ix, next_iz)] {
                if cx < 0 || cz < 0 || cx >= grid.width as i32 || cz >= grid.height as i32 {
                    return false;
                }
                if cx == to.x && cz == to.y {
                    continue;
                }
                let t = grid.tile(cx as usize, cz as usize);
                if matches!(t, Tile::Wall | Tile::DoorClosed) {
                    return false;
                }
            }

            ix = next_ix;
            iz = next_iz;
            d
        };

        if dist > max_dist {
            return true;
        }

        if ix < 0 || iz < 0 || ix >= grid.width as i32 || iz >= grid.height as i32 {
            return false;
        }

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
    match (step.x, step.y) {
        (0, 1) => Dir8(0),
        (1, 0) => Dir8(2),
        (0, -1) => Dir8(4),
        (-1, 0) => Dir8(6),
        _ => Dir8(0),
    }
}

fn dir8_towards(from: IVec2, to: IVec2) -> Dir8 {
    let d = to - from;
    if d == IVec2::ZERO {
        return Dir8(0);
    }

    let ang = (d.x as f32).atan2(d.y as f32);
    let step = std::f32::consts::FRAC_PI_4;
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
    ids: Vec<i32>,
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

fn wolf_hitscan_damage(dist_tiles: i32) -> i32 {
    let r = rand::random::<u8>() as i32;
    if dist_tiles <= 1 {
        r / 4
    } else {
        r / 6
    }
}

fn wolf_far_miss_gate(dist_tiles: i32) -> bool {
    if dist_tiles <= 3 {
        true
    } else {
        let r = rand::random::<u8>() as i32;
        (r / 12) >= dist_tiles
    }
}

fn wolf_boss_damage(dist_tiles: i32) -> i32 {
    let effective_dist = (dist_tiles / 3).max(0);
    let r = rand::random::<u8>() as i32;
    
    if effective_dist < 2 {
        r / 4
    } else if effective_dist < 4 {
        r / 8
    } else {
        r / 16
    }
}

// SYSTEM 1: Prepare shared data and handle activation/patrol
fn enemy_ai_prepare_and_activate(
    mut commands: Commands,
    time: Res<Time>,
    mut ticker: ResMut<AiTicker>,
    grid: Res<MapGrid>,
    solid: Res<SolidStatics>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut sfx: MessageWriter<PlaySfx>,
    mut alerted: Local<HashSet<Entity>>,
    wolf_plane1: Res<crate::level::WolfPlane1>,
    tunings: Res<EnemyTunings>,
    mut shared: ResMut<AiSharedData>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &GlobalTransform)>,
    mut q_enemies: Query<
        (
            Entity,
            &EnemyKind,
            &mut EnemyAi,
            &mut OccupiesTile,
            &mut Dir8,
            &Transform,
            Option<&EnemyMove>,
            Option<&Patrol>,
            Option<&crate::enemies::GuardPain>,
            Option<&crate::enemies::MutantPain>,
            Option<&crate::enemies::SsPain>,
            Option<&crate::enemies::OfficerPain>,
            Option<&crate::enemies::DogPain>,
        ),
        (With<EnemyKind>, Without<Player>, Without<Dead>),
    >,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();
    let player_tile = world_to_tile_xz(Vec2::new(player_pos.x, player_pos.z));

    shared.player_tile = player_tile;
    shared.player_pos = player_pos;

    let dt = time.delta_secs();
    ticker.accum += dt;

    while ticker.accum >= AI_TIC_SECS {
        ticker.accum -= AI_TIC_SECS;

        let areas = AreaMap::compute(&grid);
        let player_area = areas.id(player_tile);
        shared.player_area = player_area;

        let w = grid.width as i32;
        let h = grid.height as i32;
        let in_bounds = |t: IVec2| t.x >= 0 && t.y >= 0 && t.x < w && t.y < h;
        let idx = |t: IVec2| (t.y * w + t.x) as usize;

        shared.dist_map = vec![-1i32; grid.width * grid.height];
        if in_bounds(player_tile)
            && !solid.is_solid(player_tile.x, player_tile.y)
            && grid.tile(player_tile.x as usize, player_tile.y as usize) != Tile::Wall
        {
            shared.dist_map[idx(player_tile)] = 0;

            let mut queue: Vec<IVec2> = vec![player_tile];
            let mut qh: usize = 0;

            let dirs = [
                IVec2::new(1, 0),
                IVec2::new(-1, 0),
                IVec2::new(0, 1),
                IVec2::new(0, -1),
            ];

            while qh < queue.len() {
                let cur = queue[qh];
                qh += 1;

                let base = shared.dist_map[idx(cur)];
                let next = base + 1;

                for step in dirs {
                    let n = cur + step;
                    if !in_bounds(n) {
                        continue;
                    }
                    let ni = idx(n);
                    if shared.dist_map[ni] >= 0 {
                        continue;
                    }

                    if solid.is_solid(n.x, n.y) || grid.tile(n.x as usize, n.y as usize) == Tile::Wall {
                        continue;
                    }

                    shared.dist_map[ni] = next;
                    queue.push(n);
                }
            }
        }

        shared.scheduled_move.clear();
        shared.occupied.clear();

        {
            for (
                _e,
                _kind,
                _ai,
                occ,
                _dir8,
                _tf,
                _moving,
                _patrol,
                _guard_pain,
                _mutant_pain,
                _ss_pain,
                _officer_pain,
                _dog_pain,
            ) in q_enemies.iter_mut()
            {
                shared.occupied.insert(occ.0);
            }
        }

        for (
            e,
            kind,
            mut ai,
            mut occ,
            mut dir8,
            tf,
            moving,
            patrol,
            guard_pain,
            mutant_pain,
            ss_pain,
            officer_pain,
            dog_pain,
        ) in q_enemies.iter_mut()
        {
            let t = tunings.for_kind(*kind);
            let my_tile = occ.0;
            let moving_now = moving.is_some() || shared.scheduled_move.contains(&e);

            if matches!(ai.state, EnemyAiState::Stand | EnemyAiState::Patrol) {
                let same_area = player_area.is_some() && areas.id(my_tile) == player_area;
                if same_area && has_line_of_sight(&grid, &solid, my_tile, player_tile) {
                    ai.state = EnemyAiState::Chase;

                    if alerted.insert(e) && !matches!(*kind, EnemyKind::Mutant) {
                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyAlert(*kind),
                            pos: tf.translation,
                        });
                    }
                }
            }

            let in_pain = guard_pain.is_some()
                || ss_pain.is_some()
                || dog_pain.is_some()
                || officer_pain.is_some()
                || mutant_pain.is_some();

            if in_pain {
                *dir8 = dir8_towards(my_tile, player_tile);
                continue;
            }

            if matches!(ai.state, EnemyAiState::Stand) {
                continue;
            }

            if matches!(ai.state, EnemyAiState::Patrol) {
                if moving_now {
                    continue;
                }

                if patrol.is_none() {
                    continue;
                }

                if in_bounds(my_tile) {
                    let code = wolf_plane1.0[idx(my_tile)];
                    if let Some(new_dir) = patrol_dir_from_plane1(code) {
                        *dir8 = new_dir;
                    }
                }

                let step = patrol_step_8way(*dir8);
                let dest = my_tile + step;

                if step == IVec2::ZERO || !in_bounds(dest) || dest == player_tile {
                    dir8.0 = (dir8.0 + 4) & 7;
                    continue;
                }

                let diagonal = step.x != 0 && step.y != 0;
                if diagonal {
                    let a = my_tile + IVec2::new(step.x, 0);
                    let b = my_tile + IVec2::new(0, step.y);

                    if !in_bounds(a) || !in_bounds(b) {
                        dir8.0 = (dir8.0 + 4) & 7;
                        continue;
                    }

                    if shared.occupied.contains(&a) || shared.occupied.contains(&b) {
                        dir8.0 = (dir8.0 + 4) & 7;
                        continue;
                    }

                    if solid.is_solid(a.x, a.y) || solid.is_solid(b.x, b.y) {
                        dir8.0 = (dir8.0 + 4) & 7;
                        continue;
                    }

                    let ta = tile_at(&grid, a).unwrap_or(Tile::Wall);
                    let tb = tile_at(&grid, b).unwrap_or(Tile::Wall);
                    if matches!(ta, Tile::Wall | Tile::DoorClosed) || matches!(tb, Tile::Wall | Tile::DoorClosed) {
                        dir8.0 = (dir8.0 + 4) & 7;
                        continue;
                    }
                }

                if solid.is_solid(dest.x, dest.y) || shared.occupied.contains(&dest) {
                    dir8.0 = (dir8.0 + 4) & 7;
                    continue;
                }

                match tile_at(&grid, dest).unwrap_or(Tile::Wall) {
                    Tile::Wall => {
                        dir8.0 = (dir8.0 + 4) & 7;
                        continue;
                    }
                    Tile::DoorClosed => {
                        if !matches!(*kind, EnemyKind::Dog) {
                            try_open_door_at(dest, &mut q_doors, &mut sfx);
                        }
                        continue;
                    }
                    Tile::DoorOpen | Tile::Empty => {
                        let patrol_speed = t.chase_speed_tps * 0.65;

                        if CLAIM_TILE_EARLY {
                            occ.0 = dest;
                        }

                        let y = tf.translation.y;
                        let target = Vec3::new(dest.x as f32, y, dest.y as f32);

                        commands.entity(e).insert(EnemyMove {
                            target,
                            speed_tps: patrol_speed,
                        });

                        shared.scheduled_move.insert(e);
                        shared.occupied.insert(dest);
                        if CLAIM_TILE_EARLY {
                            shared.occupied.remove(&my_tile);
                        }
                    }
                }
            }
        }
    }
}

// SYSTEM 2: Handle combat (shooting, dog bites, burst fire)
fn enemy_ai_combat(
    mut commands: Commands,
    time: Res<Time>,
    grid: Res<MapGrid>,
    solid: Res<SolidStatics>,
    mut enemy_fire: MessageWriter<EnemyFire>,
    mut enemy_fireball: MessageWriter<EnemyFireballShot>,
    mut enemy_syringe: MessageWriter<EnemySyringeShot>,
    mut enemy_rocket: MessageWriter<EnemyRocketShot>,
    mut sfx: MessageWriter<PlaySfx>,
    mut shoot_cd: Local<HashMap<Entity, f32>>,
    mut bursts: Local<HashMap<Entity, BurstFire>>,
    mut los_hold: Local<HashMap<Entity, f32>>,
    tunings: Res<EnemyTunings>,
    shared: Res<AiSharedData>,
    q_enemies: Query<
        (
            Entity,
            &EnemyKind,
            &EnemyAi,
            &OccupiesTile,
            &Dir8,
            &Transform,
            Option<&EnemyMove>,
            Option<&crate::enemies::DogBite>,
            Option<&crate::enemies::DogBiteCooldown>,
        ),
        (With<EnemyKind>, Without<Dead>),
    >,
) {
    let dt = time.delta_secs();

    shoot_cd.retain(|_, t| {
        *t -= dt;
        *t > 0.0
    });

    bursts.retain(|e, _| q_enemies.get(*e).is_ok());

    let player_tile = shared.player_tile;
    let player_pos = shared.player_pos;

    for (e, kind, ai, occ, _dir8, tf, moving, dog_bite, dog_bite_cd) in q_enemies.iter() {
        if !matches!(ai.state, EnemyAiState::Chase) {
            continue;
        }

        let my_tile = occ.0;
        let moving_now = moving.is_some() || shared.scheduled_move.contains(&e);

        let cd_now = shoot_cd.get(&e).copied().unwrap_or(0.0);

        // Burst continuation must run even while cd is active
        if let Some(b) = bursts.get_mut(&e) {
            let dx = (player_tile.x - my_tile.x).abs();
            let dy = (player_tile.y - my_tile.y).abs();
            let shoot_dist = dx.max(dy);

            let can_see = has_line_of_sight(&grid, &solid, my_tile, player_tile);
            let in_range = shoot_dist <= GUARD_SHOOT_MAX_DIST_TILES;

            if !can_see || !in_range {
                b.shots_left = 0;
            } else {
                if b.next_tics > 0 {
                    b.next_tics -= 1;
                } else {
                    // Ghost Hitler uses projectile volleys, everyone else stays hitscan
                    if matches!(*kind, EnemyKind::GhostHitler) {
                        let origin = Vec3::new(tf.translation.x, 0.55, tf.translation.z);
                        let mut dir = player_pos - origin;
                        dir.y = 0.0;

                        if dir.length_squared() > 1e-6 {
                            enemy_fireball.write(EnemyFireballShot {
                                origin,
                                dir: dir.normalize(),
                            });
                        }
                    // Schabbs syringe throw (single shot, not a volley)
                    } else if matches!(*kind, EnemyKind::Schabbs) {
                        let origin = Vec3::new(tf.translation.x, 0.55, tf.translation.z);
                        let mut dir = player_pos - origin;
                        dir.y = 0.0;

                        if dir.length_squared() > 1e-6 {
                            enemy_syringe.write(EnemySyringeShot {
                                origin,
                                dir: dir.normalize(),
                            });
                        }

                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyShoot(EnemyKind::Schabbs),
                            pos: tf.translation,
                        });
                        shoot_cd.insert(e, GUARD_SHOOT_TOTAL_SECS);

                        commands.entity(e).insert(crate::enemies::SchabbsShoot {
                            t: Timer::from_seconds(crate::enemies::SCHABBS_SHOOT_SECS, TimerMode::Once),
                        });

                        continue;
                    // Otto Rocket Shoot (single shot rocket only)
                    } else if matches!(*kind, EnemyKind::Otto) {
                        let origin = Vec3::new(tf.translation.x, 0.55, tf.translation.z);
                        let mut dir = player_pos - origin;
                        dir.y = 0.0;

                        if dir.length_squared() > 1e-6 {
                            enemy_rocket.write(EnemyRocketShot {
                                origin,
                                dir: dir.normalize(),
                            });
                        }

                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyShoot(EnemyKind::Otto),
                            pos: tf.translation,
                        });
                        shoot_cd.insert(e, GUARD_SHOOT_TOTAL_SECS);

                        commands.entity(e).insert(crate::enemies::OttoShoot {
                            t: Timer::from_seconds(crate::enemies::OTTO_SHOOT_SECS, TimerMode::Once),
                        });

                        continue;
                    // General - Chaingun volley continuation (after initial rocket)
                    } else if matches!(*kind, EnemyKind::General) {
                        // General fires 4 chaingun bullets in rapid succession
                        let hits = wolf_far_miss_gate(shoot_dist);
                        let damage = if hits {
                            wolf_boss_damage(shoot_dist)
                        } else {
                            0
                        };

                        enemy_fire.write(EnemyFire { kind: *kind, damage });

                        // Insert/update the chaingun volley component for view tracking
                        commands.entity(e).insert(crate::enemies::GeneralChaingunVolley {
                            shots_remaining: b.shots_left,
                        });

                        // Remove rocket shoot component so we transition to chaingun sprites
                        commands.entity(e).remove::<crate::enemies::GeneralShoot>();

                        // Play chaingun sound on first shot of the volley
                        if b.shots_left == 4 {
                            sfx.write(PlaySfx {
                                kind: SfxKind::EnemyShoot(EnemyKind::Hans),  // Use Hans chaingun sound
                                pos: tf.translation,
                            });
                        }
                    } else {
                        let hits = wolf_far_miss_gate(shoot_dist);
                        let damage = if hits {
                            match kind {
                                EnemyKind::Hans
                                | EnemyKind::Gretel
                                | EnemyKind::MechaHitler
                                | EnemyKind::Hitler => wolf_boss_damage(shoot_dist),
                                _ => wolf_hitscan_damage(shoot_dist),
                            }
                        } else {
                            0
                        };

                        enemy_fire.write(EnemyFire { kind: *kind, damage });
                    }

                    if b.shots_left > 0 {
                        b.shots_left -= 1;
                    }
                    b.next_tics = b.every_tics;
                }
            }

            if b.shots_left == 0 {
                bursts.remove(&e);
                // Clean up General's chaingun volley component when burst completes
                if matches!(*kind, EnemyKind::General) {
                    commands.entity(e).remove::<crate::enemies::GeneralChaingunVolley>();
                }
            }

            continue;
        }

        // Dog bite state gate
        if matches!(*kind, EnemyKind::Dog) && dog_bite.is_some() {
            continue;
        }

        // Stop to shoot gate
        if cd_now > GUARD_SHOOT_COOLDOWN_SECS {
            continue;
        }

        // Start attacks only when not moving
        if moving_now {
            continue;
        }

        // Dog melee bite
        if matches!(*kind, EnemyKind::Dog) {
            let can_see = has_line_of_sight(&grid, &solid, my_tile, player_tile);
            let dx = (player_tile.x - my_tile.x).abs();
            let dy = (player_tile.y - my_tile.y).abs();
            let dist_tiles = dx.max(dy) as f32;

            if dog_bite_cd.is_none() && can_see && dist_tiles <= tunings.dog.attack_range_tiles {
                commands.entity(e).insert(crate::enemies::DogBite::new());

                sfx.write(PlaySfx {
                    kind: SfxKind::EnemyShoot(EnemyKind::Dog),
                    pos: tf.translation,
                });

                continue;
            }
        }

        // Shoot logic (non-dogs)
        if !matches!(*kind, EnemyKind::Dog) {
            let can_see = has_line_of_sight(&grid, &solid, my_tile, player_tile);

            let dx = (player_tile.x - my_tile.x).abs();
            let dy = (player_tile.y - my_tile.y).abs();
            let shoot_dist = dx.max(dy);
            let in_range = shoot_dist <= GUARD_SHOOT_MAX_DIST_TILES;

            let held = if can_see && in_range {
                let t = los_hold.entry(e).or_insert(0.0);
                *t = (*t + AI_TIC_SECS).min(LOS_FIRST_SHOT_DELAY_SECS);
                *t
            } else {
                los_hold.remove(&e);
                0.0
            };

            let los_ready = held >= LOS_FIRST_SHOT_DELAY_SECS;

            if can_see && in_range {
                if cd_now <= 0.0 && los_ready {
                    // Ghost Hitler projectile volley
                    if matches!(*kind, EnemyKind::GhostHitler) {
                        // Tune these until it visually matches DOS Wolf3D
                        let shots = 14;
                        let every_tics = 2;
                        let post_cd_secs = 0.95;

                        let origin = Vec3::new(tf.translation.x, 0.55, tf.translation.z);
                        let mut dir = player_pos - origin;
                        dir.y = 0.0;

                        if dir.length_squared() > 1e-6 {
                            enemy_fireball.write(EnemyFireballShot {
                                origin,
                                dir: dir.normalize(),
                            });

                            if shots > 1 {
                                bursts.insert(
                                    e,
                                    BurstFire {
                                        shots_left: shots - 1,
                                        every_tics,
                                        next_tics: every_tics,
                                    },
                                );
                            }

                            let burst_secs =
                                ((shots.saturating_sub(1)) as f32) * (every_tics as f32) * AI_TIC_SECS;
                            shoot_cd.insert(e, burst_secs + post_cd_secs);
                        } else {
                            shoot_cd.insert(e, GUARD_SHOOT_TOTAL_SECS);
                        }

                        commands.entity(e).insert(crate::enemies::GhostHitlerShoot {
                            t: Timer::from_seconds(crate::enemies::GHOST_HITLER_SHOOT_SECS, TimerMode::Once),
                        });

                        continue;
                    }

                    // Schabbs syringe throw (single shot, not a volley)
                    if matches!(*kind, EnemyKind::Schabbs) {
                        let origin = Vec3::new(tf.translation.x, 0.55, tf.translation.z);
                        let mut dir = player_pos - origin;
                        dir.y = 0.0;

                        if dir.length_squared() > 1e-6 {
                            enemy_syringe.write(EnemySyringeShot {
                                origin,
                                dir: dir.normalize(),
                            });
                        }

                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyShoot(EnemyKind::Schabbs),
                            pos: tf.translation,
                        });

                        shoot_cd.insert(e, GUARD_SHOOT_TOTAL_SECS);

                        commands.entity(e).insert(crate::enemies::SchabbsShoot {
                            t: Timer::from_seconds(crate::enemies::SCHABBS_SHOOT_SECS, TimerMode::Once),
                        });

                        continue;
                    }

                    // Otto and General Rocket Fire (single shot, not a volley)
                    if matches!(*kind, EnemyKind::Otto) {
                        let origin = Vec3::new(tf.translation.x, 0.55, tf.translation.z);
                        let mut dir = player_pos - origin;
                        dir.y = 0.0;

                        if dir.length_squared() > 1e-6 {
                            enemy_rocket.write(EnemyRocketShot {
                                origin,
                                dir: dir.normalize(),
                            });
                        }

                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyShoot(EnemyKind::Otto),
                            pos: tf.translation,
                        });

                        shoot_cd.insert(e, GUARD_SHOOT_TOTAL_SECS);

                        commands.entity(e).insert(crate::enemies::OttoShoot {
                            t: Timer::from_seconds(crate::enemies::OTTO_SHOOT_SECS, TimerMode::Once),
                        });

                        continue;
                    }

                    // General Fettgesicht - Fires 1 rocket then 4 chaingun bullets per volley
                    if matches!(*kind, EnemyKind::General) {
                        let origin = Vec3::new(tf.translation.x, 0.55, tf.translation.z);
                        let mut dir = player_pos - origin;
                        dir.y = 0.0;

                        // Fire the rocket first
                        if dir.length_squared() > 1e-6 {
                            enemy_rocket.write(EnemyRocketShot {
                                origin,
                                dir: dir.normalize(),
                            });
                        }

                        // Play rocket fire sound
                        sfx.write(PlaySfx {
                            kind: SfxKind::EnemyShoot(EnemyKind::General),
                            pos: tf.translation,
                        });

                        // Set up rocket animation component
                        commands.entity(e).insert(crate::enemies::GeneralShoot {
                            t: Timer::from_seconds(crate::enemies::GENERAL_SHOOT_SECS, TimerMode::Once),
                        });

                        // Queue up 4 chaingun shots in a burst
                        // Tune these values to match Wolf3D feel
                        let shots = 4;
                        let every_tics = 4;  // Spacing between chaingun shots
                        
                        // CRITICAL: Delay first chaingun shot until AFTER rocket animation
                        // Convert rocket animation time to tics
                        let rocket_anim_tics = (crate::enemies::GENERAL_SHOOT_SECS / AI_TIC_SECS).ceil() as u32;

                        bursts.insert(
                            e,
                            BurstFire {
                                shots_left: shots,
                                every_tics,
                                next_tics: rocket_anim_tics,  // Wait for rocket to finish!
                            },
                        );

                        // Total duration = rocket animation + chaingun burst + post cooldown
                        let burst_secs = (shots as f32) * (every_tics as f32) * AI_TIC_SECS;
                        let rocket_secs = crate::enemies::GENERAL_SHOOT_SECS;
                        let post_cd_secs = 0.45;
                        shoot_cd.insert(e, rocket_secs + burst_secs + post_cd_secs);

                        continue;
                    }

                    // Regular hitscan shooting
                    shoot_cd.insert(e, GUARD_SHOOT_TOTAL_SECS);

                    let hits = wolf_far_miss_gate(shoot_dist);
                    let damage = if hits {
                        match kind {
                            EnemyKind::Hans | EnemyKind::Gretel => wolf_boss_damage(shoot_dist),
                            _ => wolf_hitscan_damage(shoot_dist),
                        }
                    } else {
                        0
                    };

                    enemy_fire.write(EnemyFire { kind: *kind, damage });

                    match kind {
                        EnemyKind::Guard => {
                            commands.entity(e).insert(crate::enemies::GuardShoot {
                                timer: Timer::from_seconds(GUARD_SHOOT_PAUSE_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Mutant => {
                            commands.entity(e).insert(crate::enemies::MutantShoot {
                                timer: Timer::from_seconds(MUTANT_SHOOT_PAUSE_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Ss => {
                            commands.entity(e).insert(crate::enemies::SsShoot {
                                t: Timer::from_seconds(crate::enemies::SS_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Officer => {
                            commands.entity(e).insert(crate::enemies::OfficerShoot {
                                t: Timer::from_seconds(crate::enemies::OFFICER_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Hans => {
                            commands.entity(e).insert(crate::enemies::HansShoot {
                                t: Timer::from_seconds(crate::enemies::HANS_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Gretel => {
                            commands.entity(e).insert(crate::enemies::GretelShoot {
                                t: Timer::from_seconds(crate::enemies::GRETEL_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Hitler => {
                            commands.entity(e).insert(crate::enemies::HitlerShoot {
                                t: Timer::from_seconds(crate::enemies::HITLER_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::MechaHitler => {
                            commands.entity(e).insert(crate::enemies::MechaHitlerShoot {
                                t: Timer::from_seconds(crate::enemies::MECHA_HITLER_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::GhostHitler => {
                            commands.entity(e).insert(crate::enemies::GhostHitlerShoot {
                                t: Timer::from_seconds(crate::enemies::GHOST_HITLER_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Schabbs => {
                            commands.entity(e).insert(crate::enemies::SchabbsShoot {
                                t: Timer::from_seconds(crate::enemies::SCHABBS_THROW_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Otto => {
                            commands.entity(e).insert(crate::enemies::OttoShoot {
                                t: Timer::from_seconds(crate::enemies::OTTO_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::General => {
                            commands.entity(e).insert(crate::enemies::GeneralShoot {
                                t: Timer::from_seconds(crate::enemies::GENERAL_SHOOT_SECS, TimerMode::Once),
                            });
                        }
                        EnemyKind::Dog => {}
                    }

                    sfx.write(PlaySfx {
                        kind: SfxKind::EnemyShoot(*kind),
                        pos: tf.translation,
                    });

                    continue;
                }
            }
        }
    }
}

// SYSTEM 3: Handle movement (pathfinding and door opening)
fn enemy_ai_movement(
    mut commands: Commands,
    grid: Res<MapGrid>,
    solid: Res<SolidStatics>,
    mut sfx: MessageWriter<PlaySfx>,
    tunings: Res<EnemyTunings>,
    mut shared: ResMut<AiSharedData>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &GlobalTransform)>,
    mut q_enemies: Query<
        (
            Entity,
            &EnemyKind,
            &mut EnemyAi,
            &mut OccupiesTile,
            &Transform,  // Changed from &mut Dir8 to just &Transform
            Option<&EnemyMove>,
        ),
        (With<EnemyKind>, Without<Dead>),
    >,
) {
    let player_tile = shared.player_tile;
    
    let w = grid.width as i32;
    let h = grid.height as i32;
    let in_bounds = |t: IVec2| t.x >= 0 && t.y >= 0 && t.x < w && t.y < h;
    let idx = |t: IVec2| (t.y * w + t.x) as usize;

    let dirs = [
        IVec2::new(1, 0),
        IVec2::new(-1, 0),
        IVec2::new(0, 1),
        IVec2::new(0, -1),
    ];

    for (e, kind, mut ai, mut occ, tf, moving) in q_enemies.iter_mut() {
        if !matches!(ai.state, EnemyAiState::Chase) {
            continue;
        }

        let t = tunings.for_kind(*kind);
        let speed = t.chase_speed_tps;
        let my_tile = occ.0;
        let moving_now = moving.is_some() || shared.scheduled_move.contains(&e);

        if moving_now {
            continue;
        }

        let mut moved_or_acted = false;

        if in_bounds(my_tile) {
            let my_d = shared.dist_map[idx(my_tile)];
            if my_d >= 0 {
                let mut best: Option<(i32, IVec2, Tile)> = None;

                for step in dirs {
                    let dest = my_tile + step;
                    if dest == player_tile || !in_bounds(dest) {
                        continue;
                    }

                    let tile = grid.tile(dest.x as usize, dest.y as usize);

                    if tile == Tile::Wall || solid.is_solid(dest.x, dest.y) {
                        continue;
                    }

                    if matches!(*kind, EnemyKind::Dog) && tile == Tile::DoorClosed {
                        continue;
                    }

                    let d = shared.dist_map[idx(dest)];
                    if d < 0 || d >= my_d {
                        continue;
                    }

                    if shared.occupied.contains(&dest) {
                        continue;
                    }

                    let mut score = d * 10;
                    if step == -ai.last_step {
                        score += 5;
                    }
                    if tile == Tile::DoorClosed {
                        score += 1;
                    }

                    if best.map(|(bs, _, _)| score < bs).unwrap_or(true) {
                        best = Some((score, dest, tile));
                    }
                }

                if let Some((_score, dest, tile)) = best {
                    if tile == Tile::DoorClosed {
                        if !matches!(*kind, EnemyKind::Dog) {
                            try_open_door_at(dest, &mut q_doors, &mut sfx);
                            ai.last_step = IVec2::ZERO;
                            moved_or_acted = true;
                        }
                    } else {
                        let step = dest - my_tile;
                        let new_dir = dir8_from_step(step);
                        commands.entity(e).insert(PendingDir8(new_dir));
                        ai.last_step = step;

                        if CLAIM_TILE_EARLY {
                            occ.0 = dest;
                        }

                        let y = tf.translation.y;
                        let target = Vec3::new(dest.x as f32, y, dest.y as f32);

                        commands.entity(e).insert(EnemyMove {
                            target,
                            speed_tps: speed,
                        });

                        shared.scheduled_move.insert(e);
                        shared.occupied.insert(dest);
                        if CLAIM_TILE_EARLY {
                            shared.occupied.remove(&my_tile);
                        }

                        moved_or_acted = true;
                    }
                }
            }
        }

        // Fallback pathfinding
        if !moved_or_acted {
            match pick_chase_step(&grid, &solid, &shared.occupied, my_tile, player_tile, ai.last_step) {
                ChasePick::MoveTo(dest) => {
                    if dest != player_tile && !shared.occupied.contains(&dest) {
                        let step = dest - my_tile;
                        let new_dir = dir8_from_step(step);
                        commands.entity(e).insert(PendingDir8(new_dir));  // Insert pending instead of mutating
                        ai.last_step = step;

                        if CLAIM_TILE_EARLY {
                            occ.0 = dest;
                        }

                        let y = tf.translation.y;
                        let target = Vec3::new(dest.x as f32, y, dest.y as f32);

                        commands.entity(e).insert(EnemyMove {
                            target,
                            speed_tps: speed,
                        });

                        shared.scheduled_move.insert(e);
                        shared.occupied.insert(dest);
                        if CLAIM_TILE_EARLY {
                            shared.occupied.remove(&my_tile);
                        }
                    }
                }
                ChasePick::OpenDoor(door_tile) => {
                    if !matches!(*kind, EnemyKind::Dog) {
                        try_open_door_at(door_tile, &mut q_doors, &mut sfx);
                        ai.last_step = IVec2::ZERO;
                    }
                }
                ChasePick::None => {}
            }
        }
    }
}

// SYSTEM 4: Apply pending Dir8 changes
fn apply_pending_dir8(
    mut commands: Commands,
    mut q: Query<(Entity, &PendingDir8, &mut Dir8)>,
) {
    for (e, pending, mut dir8) in q.iter_mut() {
        *dir8 = pending.0;
        commands.entity(e).remove::<PendingDir8>();
    }
}

fn enemy_ai_move(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<
        (
            Entity,
            &EnemyMove,
            &mut Transform,
            Option<&crate::enemies::GuardPain>,
            Option<&crate::enemies::SsPain>,
            Option<&crate::enemies::DogPain>,
            Option<&crate::enemies::OfficerPain>,
            Option<&crate::enemies::MutantPain>,
        ),
        Without<Dead>,
    >,
) {
    let dt = time.delta_secs();

    for (e, mv, mut tf, guard_pain, ss_pain, dog_pain, officer_pain, mutant_pain) in q.iter_mut() {
        if guard_pain.is_some()
            || ss_pain.is_some()
            || dog_pain.is_some()
            || officer_pain.is_some()
            || mutant_pain.is_some()
        {
            continue;
        }

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

fn player_can_be_targeted(
    lock: Res<PlayerControlLock>,
    latch: Res<PlayerDeathLatch>,
    map: Option<Res<MapGrid>>,
    plane1: Option<Res<crate::level::WolfPlane1>>,
) -> bool {
    !lock.0 && !latch.0 && map.is_some() && plane1.is_some()
}

fn world_ready(
    map: Option<Res<MapGrid>>,
    plane1: Option<Res<crate::level::WolfPlane1>>,
) -> bool {
    map.is_some() && plane1.is_some()
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum AiFixedSet {
    Prepare,
    Combat,
    Movement,
    ApplyDir8,
    Move,
}

pub struct EnemyAiPlugin;

impl Plugin for EnemyAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AiTicker>()
            .init_resource::<AiSharedData>()
            .insert_resource(EnemyTunings::baseline())
            .add_message::<EnemyFire>()
            .add_message::<EnemyFireballShot>()
            .add_message::<EnemySyringeShot>()
            .add_message::<EnemyRocketShot>()
            .configure_sets(
                FixedUpdate,
                (
                    AiFixedSet::Prepare,
                    AiFixedSet::Combat,
                    AiFixedSet::Movement,
                    AiFixedSet::ApplyDir8,
                    AiFixedSet::Move,
                )
                    .chain(),
            )
            .add_systems(Update, attach_enemy_ai.run_if(world_ready))
            .add_systems(
                FixedUpdate,
                enemy_ai_prepare_and_activate
                    .in_set(AiFixedSet::Prepare)
                    .run_if(player_can_be_targeted),
            )
            .add_systems(
                FixedUpdate,
                enemy_ai_combat
                    .in_set(AiFixedSet::Combat)
                    .run_if(player_can_be_targeted),
            )
            .add_systems(
                FixedUpdate,
                enemy_ai_movement
                    .in_set(AiFixedSet::Movement)
                    .run_if(player_can_be_targeted),
            )
            .add_systems(
                FixedUpdate,
                apply_pending_dir8
                    .in_set(AiFixedSet::ApplyDir8)
                    .run_if(player_can_be_targeted),
            )
            .add_systems(
                FixedUpdate,
                enemy_ai_move
                    .in_set(AiFixedSet::Move)
                    .run_if(player_can_be_targeted),
            );
    }
}
