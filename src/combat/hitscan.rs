use bevy::prelude::*;
use davelib::map::{MapGrid, Tile};

#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    #[allow(dead_code)]
    pub tile: Tile,
    #[allow(dead_code)]
    pub tile_coord: IVec2,
    #[allow(dead_code)]
    pub pos: Vec3,
    #[allow(dead_code)]
    pub normal: Vec3,
    #[allow(dead_code)]
    pub dist: f32,
}

pub fn raycast_grid(grid: &MapGrid, origin: Vec3, dir3: Vec3, max_dist: f32) -> Option<RayHit> {
    // Keep in sync with world.rs
    const FLOOR_Y: f32 = 0.0;
    const WALL_H: f32 = 1.0;

    const EPS_DIR: f32 = 1e-8;
    const EPS_Y: f32 = 1e-4;

    let dir3 = dir3.normalize_or_zero();
    if dir3 == Vec3::ZERO {
        return None;
    }

    let dx = dir3.x;
    let dy = dir3.y;
    let dz = dir3.z;

    let floor_hit = |t: f32| RayHit {
        tile: Tile::Empty,              // floor sentinel
        tile_coord: IVec2::new(-1, -1), // floor sentinel
        pos: origin + dir3 * t,
        normal: Vec3::Y,
        dist: t,
    };

    // Floor intersection (no ceiling per design)
    let t_floor = if dy < -EPS_DIR {
        let t = (FLOOR_Y - origin.y) / dy;
        (t >= 0.0).then_some(t)
    } else {
        None
    };

    // If basically vertical (no XZ movement), only floor can be hit
    if dx.abs() < EPS_DIR && dz.abs() < EPS_DIR {
        if let Some(t) = t_floor {
            if t <= max_dist {
                return Some(floor_hit(t));
            }
        }
        return None;
    }

    // DDA in XZ (tile boundaries at N+0.5, matches your collision scheme)
    let px = origin.x + 0.5;
    let pz = origin.z + 0.5;

    let mut ix = px.floor() as i32;
    let mut iz = pz.floor() as i32;

    let step_x = if dx > 0.0 { 1 } else { -1 };
    let step_z = if dz > 0.0 { 1 } else { -1 };

    let t_delta_x = if dx.abs() < EPS_DIR { f32::INFINITY } else { 1.0 / dx.abs() };
    let t_delta_z = if dz.abs() < EPS_DIR { f32::INFINITY } else { 1.0 / dz.abs() };

    let next_x = if dx > 0.0 { ix as f32 + 1.0 } else { ix as f32 };
    let next_z = if dz > 0.0 { iz as f32 + 1.0 } else { iz as f32 };

    let mut t_max_x = if dx.abs() < EPS_DIR { f32::INFINITY } else { (next_x - px) / dx };
    let mut t_max_z = if dz.abs() < EPS_DIR { f32::INFINITY } else { (next_z - pz) / dz };

    if t_max_x < 0.0 { t_max_x = 0.0; }
    if t_max_z < 0.0 { t_max_z = 0.0; }

    let max_steps = (grid.width.max(grid.height) as i32) * 4;

    for _ in 0..max_steps {
        let t_next = t_max_x.min(t_max_z);

        // Floor can be hit before we even reach the next grid boundary
        if let Some(t) = t_floor {
            if t <= t_next && t <= max_dist {
                return Some(floor_hit(t));
            }
        }

        // Step to next cell boundary; compute the normal for THIS step locally
        let (dist, step_normal) = if t_max_x < t_max_z {
            ix += step_x;
            let dist = t_max_x;
            t_max_x += t_delta_x;
            (dist, Vec3::new(-(step_x as f32), 0.0, 0.0))
        } else {
            iz += step_z;
            let dist = t_max_z;
            t_max_z += t_delta_z;
            (dist, Vec3::new(0.0, 0.0, -(step_z as f32)))
        };

        if dist > max_dist {
            return None;
        }

        // Bounds
        if ix < 0 || iz < 0 || ix >= grid.width as i32 || iz >= grid.height as i32 {
            return None;
        }

        let tile = grid.tile(ix as usize, iz as usize);

        // Stops on walls + closed doors (open doors are pass-through)
        if matches!(tile, Tile::Wall | Tile::DoorClosed) {
            let y_at = origin.y + dy * dist;
            if y_at >= FLOOR_Y - EPS_Y && y_at <= WALL_H + EPS_Y {
                return Some(RayHit {
                    tile,
                    tile_coord: IVec2::new(ix, iz),
                    pos: origin + dir3 * dist,
                    normal: step_normal,
                    dist,
                });
            }
        }
    }

    None
}
