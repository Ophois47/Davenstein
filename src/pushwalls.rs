/*
Davenstein - by David Petnick
*/

// Wolf3D '92-style pushwalls (secret walls)
//
// Minimal, Wolf-accurate behavior:
// - Only ONE pushwall can move at a time.
// - Trigger: player "use" (Space) on a pushwall-marked wall.
// - Moves 2 tiles total.
// - Uses Wolf's 70 Hz tic clock and 128 tics per tile => 256 tics total.
// - Collision/hitscan treat BOTH the current pushwall base tile and the
//   tile in front as blocked (matches Wolf's tilemap=64 / actorat=BLOCKTILE trick).
// - Tile-boundary updates: the tile the wall leaves becomes empty on 128-tic boundaries.
// - Pushwalls are one-shot: marker is consumed on activation.

use bevy::prelude::*;

use crate::actors::{Dead, OccupiesTile};
use crate::audio::{PlaySfx, SfxKind};
use crate::decorations::SolidStatics;
use crate::enemies::EnemyKind;
use crate::map::{MapGrid, Tile};
use crate::player::{Player, PlayerControlLock};
use crate::world::{RebuildWalls, WallRenderCache};

const WOLF_TIC_HZ: f32 = 70.0;
const WOLF_TIC_SECS: f32 = 1.0 / WOLF_TIC_HZ;

// Wolf uses 128 tics per tile for pushwalls (and stops at 256 for 2 tiles).
const PUSHWALL_TICS_PER_TILE: u32 = 128;
const PUSHWALL_TOTAL_TICS: u32 = PUSHWALL_TICS_PER_TILE * 2;

// Plane1 "pushwall marker" code in Wolf maps (the tile in plane0 is a normal wall).
const PUSHWALL_MARKER_CODE: u16 = 98;

#[derive(Resource, Debug, Clone)]
pub struct PushwallMarkers {
    width: usize,
    height: usize,
    marked: Vec<bool>,
}

impl PushwallMarkers {
    pub fn empty(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            marked: vec![false; width * height],
        }
    }

    pub fn from_wolf_plane1(width: usize, height: usize, plane1: &[u16]) -> Self {
        let mut out = Self::empty(width, height);
        for z in 0..height {
            for x in 0..width {
                let i = z * width + x;
                if i < plane1.len() && plane1[i] == PUSHWALL_MARKER_CODE {
                    out.marked[i] = true;
                }
            }
        }
        out
    }

    #[inline]
    fn idx(&self, x: i32, z: i32) -> Option<usize> {
        if x < 0 || z < 0 {
            return None;
        }
        let (xu, zu) = (x as usize, z as usize);
        if xu >= self.width || zu >= self.height {
            return None;
        }
        Some(zu * self.width + xu)
    }

    pub fn is_marked(&self, x: i32, z: i32) -> bool {
        self.idx(x, z).map(|i| self.marked[i]).unwrap_or(false)
    }

    pub fn consume(&mut self, x: i32, z: i32) {
        if let Some(i) = self.idx(x, z) {
            self.marked[i] = false;
        }
    }
}

/// Tiles blocked by the moving pushwall (current base + tile ahead).
#[derive(Resource, Default, Debug, Clone)]
pub struct PushwallOcc {
    pub a: Option<IVec2>,
    pub b: Option<IVec2>,
}

impl PushwallOcc {
    pub fn clear(&mut self) {
        self.a = None;
        self.b = None;
    }

    pub fn set(&mut self, a: IVec2, b: IVec2) {
        self.a = Some(a);
        self.b = Some(b);
    }

    pub fn blocks(&self, t: IVec2) -> bool {
        self.a == Some(t) || self.b == Some(t)
    }

    pub fn blocks_tile(&self, x: i32, z: i32) -> bool {
        self.blocks(IVec2::new(x, z))
    }
}

#[derive(Component)]
pub struct PushwallVisual;

#[derive(Resource, Default)]
pub struct PushwallClock {
    accum: f32,
}

impl PushwallClock {
    pub fn reset(&mut self) {
        self.accum = 0.0;
    }
}

#[derive(Debug)]
pub struct ActivePushwall {
    /// Wall texture id from plane0 (1..63 for Wolf walls).
    pub wall_id: u16,
    /// Base tile coordinate (Wolf's pwallx/pwally) – advances at 128-tic boundaries.
    pub base: IVec2,
    /// Cardinal direction of movement.
    pub dir: IVec2,
    /// "pwallstate" counter (0..=256).
    pub state: u32,
    /// Visual entity for the moving wall (a small 4-face "block").
    pub entity: Entity,
}

#[derive(Resource, Default)]
pub struct PushwallState {
    pub active: Option<ActivePushwall>,
}

fn despawn_tree(commands: &mut Commands, q_children: &Query<&Children>, e: Entity) {
    if let Ok(children) = q_children.get(e) {
        // In this Bevy version, Children::iter() already yields Entity (copied).
        let kids: Vec<Entity> = children.iter().collect();
        for child in kids {
            despawn_tree(commands, q_children, child);
        }
    }
    commands.entity(e).despawn();
}

fn cardinal_from_fwd(fwd: Vec3) -> IVec2 {
    // Determine dominant axis (Wolf is 4-way).
    if fwd.x.abs() > fwd.z.abs() {
        if fwd.x >= 0.0 {
            IVec2::new(1, 0)
        } else {
            IVec2::new(-1, 0)
        }
    } else {
        if fwd.z >= 0.0 {
            IVec2::new(0, 1)
        } else {
            IVec2::new(0, -1)
        }
    }
}

fn in_bounds(grid: &MapGrid, t: IVec2) -> bool {
    t.x >= 0
        && t.y >= 0
        && (t.x as usize) < grid.width
        && (t.y as usize) < grid.height
}

fn is_blocked_for_push(
    grid: &MapGrid,
    solid: &SolidStatics,
    q_enemies: &Query<&OccupiesTile, (With<EnemyKind>, Without<Dead>)>,
    t: IVec2,
) -> bool {
    if !in_bounds(grid, t) {
        return true;
    }
    // Walls and closed doors are hard blockers.
    match grid.tile(t.x as usize, t.y as usize) {
        Tile::Wall | Tile::DoorClosed => return true,
        _ => {}
    }
    // Blocking statics
    if solid.is_solid(t.x, t.y) {
        return true;
    }
    // Living actors
    for ot in q_enemies.iter() {
        if ot.0 == t {
            return true;
        }
    }
    false
}

fn spawn_pushwall_visual(
    commands: &mut Commands,
    cache: &WallRenderCache,
    wall_id: u16,
    start_center: Vec3,
) -> Entity {
    // Recreate the same wall chunk mapping used in world.rs
    let wall_type = (wall_id.saturating_sub(1)) as usize;
    let pair_base = wall_type * 2;
    let light_idx = pair_base;
    let dark_idx = pair_base + 1;

    let light_panel = cache
        .atlas_panels
        .get(light_idx)
        .cloned()
        .unwrap_or_else(|| cache.atlas_panels[0].clone());
    let dark_panel = cache
        .atlas_panels
        .get(dark_idx)
        .cloned()
        .unwrap_or_else(|| cache.atlas_panels[0].clone());

    // A "block" is 4 vertical planes around the tile center (like your static walls).
    let parent = commands
        .spawn((
            PushwallVisual,
            Transform::from_translation(start_center),
            GlobalTransform::default(),
            Visibility::Visible,
        ))
        .with_children(|p| {
            // North (-Z) light
            p.spawn((
                Mesh3d(light_panel.clone()),
                MeshMaterial3d(cache.wall_mat.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.0, -0.5),
                    rotation: Quat::from_rotation_y(0.0) * cache.wall_base,
                    ..default()
                },
                Visibility::Visible,
            ));
            // South (+Z) light
            p.spawn((
                Mesh3d(light_panel.clone()),
                MeshMaterial3d(cache.wall_mat.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.0, 0.5),
                    rotation: Quat::from_rotation_y(std::f32::consts::PI) * cache.wall_base,
                    ..default()
                },
                Visibility::Visible,
            ));
            // East (+X) dark
            p.spawn((
                Mesh3d(dark_panel.clone()),
                MeshMaterial3d(cache.wall_mat_dark.clone()),
                Transform {
                    translation: Vec3::new(0.5, 0.0, 0.0),
                    rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2) * cache.wall_base,
                    ..default()
                },
                Visibility::Visible,
            ));
            // West (-X) dark
            p.spawn((
                Mesh3d(dark_panel.clone()),
                MeshMaterial3d(cache.wall_mat_dark.clone()),
                Transform {
                    translation: Vec3::new(-0.5, 0.0, 0.0),
                    rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2) * cache.wall_base,
                    ..default()
                },
                Visibility::Visible,
            ));
        })
        .id();

    parent
}

/// Player "use" handler: attempts to start a pushwall. Plays "no way" when blocked.
pub fn use_pushwalls(
    keys: Res<ButtonInput<KeyCode>>,
    lock: Res<PlayerControlLock>,
    grid: Option<Res<MapGrid>>,
    solid: Option<Res<SolidStatics>>,
    mut markers: ResMut<PushwallMarkers>,
    cache: Res<WallRenderCache>,
    q_player: Query<&Transform, With<Player>>,
    q_enemies: Query<&OccupiesTile, (With<EnemyKind>, Without<Dead>)>,
    mut pw_state: ResMut<PushwallState>,
    mut pw_occ: ResMut<PushwallOcc>,
    mut sfx: MessageWriter<PlaySfx>,
    mut rebuild: MessageWriter<RebuildWalls>,
    mut commands: Commands,
) {
    let (Some(grid), Some(solid)) = (grid, solid) else {
        return;
    };

     // Prevents Use of Pushwalls While Dead / Game Over
    if lock.0 {
        return;
    }

    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    let Some(player_tf) = q_player.iter().next() else {
        return;
    };

    // IMPORTANT: Tiles Centered on Integers
    // Boundaries are at n ± 0.5
    let world_to_tile = |p: Vec3| -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.z + 0.5).floor() as i32)
    };

    // If Pushwall Already Active Don't Start Another
    if pw_state.active.is_some() {
        let player_tile = world_to_tile(player_tf.translation);

        let mut fwd = player_tf.rotation * Vec3::NEG_Z;
        fwd.y = 0.0;
        let dir = cardinal_from_fwd(fwd.normalize_or_zero());
        let front = player_tile + dir;

        if in_bounds(&grid, front)
            && matches!(grid.tile(front.x as usize, front.y as usize), Tile::Wall)
        {
            sfx.write(PlaySfx {
                kind: SfxKind::NoWay,
                pos: player_tf.translation,
            });
        }
        return;
    }

    let player_tile = world_to_tile(player_tf.translation);

    let mut fwd = player_tf.rotation * Vec3::NEG_Z;
    fwd.y = 0.0;
    let dir = cardinal_from_fwd(fwd.normalize_or_zero());
    let front = player_tile + dir;

    if !in_bounds(&grid, front) {
        return;
    }

    // Only Attempt if Tile in Front is Wall
    if !matches!(grid.tile(front.x as usize, front.y as usize), Tile::Wall) {
        return;
    }

    // Must be Marked as Pushwall in plane1 Markers
    if !markers.is_marked(front.x, front.y) {
        sfx.write(PlaySfx {
            kind: SfxKind::NoWay,
            pos: player_tf.translation,
        });
        return;
    }

    let wall_id = grid.plane0_code(front.x as usize, front.y as usize);
    if wall_id == 0 {
        sfx.write(PlaySfx {
            kind: SfxKind::NoWay,
            pos: player_tf.translation,
        });
        return;
    }

    // 2 Tiles Ahead Must be Clear
    let t1 = front + dir;
    let t2 = front + dir * 2;

    if is_blocked_for_push(&grid, &solid, &q_enemies, t1)
        || is_blocked_for_push(&grid, &solid, &q_enemies, t2)
    {
        sfx.write(PlaySfx {
            kind: SfxKind::NoWay,
            pos: player_tf.translation,
        });
        return;
    }

    // Consume Marker so Can't be Pushed Again
    markers.consume(front.x, front.y);

    // Spawn Visual Wall Centered on Pushwall Tile (Y is Half Wall Height = 0.5)
    let start_center = Vec3::new(front.x as f32, 0.5, front.y as f32);
    let ent = spawn_pushwall_visual(&mut commands, &cache, wall_id, start_center);

    // Initialize State. Wolfenstein 3D Base Starts at Wall Tile Itself
    let active = ActivePushwall {
        wall_id,
        base: front,
        dir,
        state: 1,
        entity: ent,
    };

    pw_state.active = Some(active);

    // Block Base + Ahead Tile
    pw_occ.set(front, front + dir);

    // Rebuild Wall Faces, Skipping Pushwall Base Tile (Moving Wall Renders It)
    rebuild.write(RebuildWalls { skip: Some(front) });

    // Play Pushwall Sound
    sfx.write(PlaySfx {
        kind: SfxKind::Pushwall,
        pos: player_tf.translation,
    });
}

/// Fixed-Tick Pushwall Movement at Wolf's 70 Hz Tic Rate
/// Run in FixedUpdate (60Hz) but Internally Step at 70Hz Using an Accumulator
pub fn tick_pushwalls(
    time: Res<Time>,
    mut clock: ResMut<PushwallClock>,
    mut pws: ResMut<PushwallState>,
    mut occ: ResMut<PushwallOcc>,
    mut grid: ResMut<MapGrid>,
    mut q_vis: Query<&mut Transform, With<PushwallVisual>>,
    q_children: Query<&Children>,
    mut commands: Commands,
    mut rebuild: MessageWriter<RebuildWalls>,
) {
    let Some(active) = pws.active.as_mut() else {
        return;
    };

    clock.accum += time.delta_secs();

    // Process Multiple 70Hz Tics if FixedUpdate is Slow
    while clock.accum >= WOLF_TIC_SECS {
        clock.accum -= WOLF_TIC_SECS;

        let old_block = active.state / PUSHWALL_TICS_PER_TILE;
        active.state += 1;
        let new_block = active.state / PUSHWALL_TICS_PER_TILE;

        // Boundary Crossing (Every 128 Tics)
        if new_block != old_block {
            // Tile Behind Becomes Empty
            if in_bounds(&grid, active.base) {
                grid.set_tile(active.base.x as usize, active.base.y as usize, Tile::Empty);
                grid.set_plane0_code(active.base.x as usize, active.base.y as usize, 0);
            }

            // Stop After Exactly 2 Tiles
            if active.state >= PUSHWALL_TOTAL_TICS {
                let dest = active.base + active.dir;
                if in_bounds(&grid, dest) {
                    grid.set_tile(dest.x as usize, dest.y as usize, Tile::Wall);
                    grid.set_plane0_code(dest.x as usize, dest.y as usize, active.wall_id);
                }

                // Remove Visual Entity + Children
                despawn_tree(&mut commands, &q_children, active.entity);

                // Clear State + Occupancy
                pws.active = None;
                occ.clear();

                // Rebuild Walls Normally (No Skip)
                rebuild.write(RebuildWalls { skip: None });
                return;
            }

            // Continue: Advance Base by 1 Tile
            active.base += active.dir;

            // Block Base + Ahead Tile
            occ.set(active.base, active.base + active.dir);

            // Rebuild Walls Skipping New Base Tile (Moving Wall Renders It)
            rebuild.write(RebuildWalls {
                skip: Some(active.base),
            });
        }
    }

    // Visual Interpolation Inside Current Tile Segment
    let pwallpos = ((active.state / 2) & 63) as f32 / 64.0;
    let base_center = Vec3::new(active.base.x as f32, 0.5, active.base.y as f32);
    let offset = Vec3::new(active.dir.x as f32, 0.0, active.dir.y as f32) * pwallpos;
    let pos = base_center + offset;

    if let Ok(mut tf) = q_vis.get_mut(active.entity) {
        tf.translation = pos;
    }
}
