/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use davelib::map::{MapGrid, Tile};
use davelib::decorations::SolidStatics;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    pub tile: Tile,
    pub tile_coord: IVec2,
    pub pos: Vec3,
    pub normal: Vec3,
    pub dist: f32,
}

pub fn raycast_grid(
    grid: &MapGrid,
    _solid: &SolidStatics,
    origin: Vec3, 
    dir3: Vec3, 
    max_dist: f32
) -> Option<RayHit> {
    const FLOOR_Y: f32 = 0.0;
    const WALL_H: f32 = 1.0;

    const EPS_DIR: f32 = 1e-8;
    const EPS_Y: f32 = 1e-4;
    // Tie-Break For Corner Hits
    const EPS_T: f32 = 1e-6;

    let dir3 = dir3.normalize_or_zero();
    if dir3 == Vec3::ZERO {
        return None;
    }

    let dx = dir3.x;
    let dy = dir3.y;
    let dz = dir3.z;

    // Helper Hit Constructors
    let floor_hit = |t: f32| RayHit {
        tile: Tile::Empty,
        tile_coord: IVec2::new(-1, -1),
        pos: origin + dir3 * t,
        normal: Vec3::Y,
        dist: t,
    };

    // Floor Intersection
    let t_floor = if dy < -EPS_DIR {
        let t = (FLOOR_Y - origin.y) / dy;
        (t >= 0.0).then_some(t)
    } else {
        None
    };

    // DDA in XZ (Tile Boundaries at N+0.5, Matches Collision Scheme)
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
    let mut step_normal;

    for _ in 0..max_steps {
        let t_next = t_max_x.min(t_max_z);

        // Floor Can be Hit Before Reaching Next Grid Boundary
        if let Some(t) = t_floor {
            if t <= t_next && t <= max_dist {
                return Some(floor_hit(t));
            }
        }

        // Step to Next Cell Boundary
        let dist = if t_max_x + EPS_T < t_max_z {
            ix += step_x;
            let dist = t_max_x;
            t_max_x += t_delta_x;
            step_normal = Vec3::new(-(step_x as f32), 0.0, 0.0);
            dist
        } else if t_max_z + EPS_T < t_max_x {
            iz += step_z;
            let dist = t_max_z;
            t_max_z += t_delta_z;
            step_normal = Vec3::new(0.0, 0.0, -(step_z as f32));
            dist
        } else {
            // Corner Cross: Test Both Adjacent Cells
            // Prevents Corner Peeking Through Walls
            let cand_x = (ix + step_x, iz);
            let cand_z = (ix, iz + step_z);
            let dist = t_max_x; // ~= t_max_z

            for (cx, cz, normal) in [
                (cand_x.0, cand_x.1, Vec3::new(-(step_x as f32), 0.0, 0.0)),
                (cand_z.0, cand_z.1, Vec3::new(0.0, 0.0, -(step_z as f32))),
            ] {
                if cx < 0 || cz < 0 || cx >= grid.width as i32 || cz >= grid.height as i32 {
                    return None;
                }

                // Intentionally Do NOT Stop on SolidStatics Here
                // Statics Block Movement, Hitscan Should Pass Through Decorations

                let tile = grid.tile(cx as usize, cz as usize);
                if matches!(tile, Tile::Wall | Tile::DoorClosed) {
                    return Some(RayHit {
                        tile,
                        tile_coord: IVec2::new(cx, cz),
                        pos: origin + dir3 * dist,
                        normal,
                        dist,
                    });
                }
            }

            // Advance Diagonally
            ix += step_x;
            iz += step_z;
            t_max_x += t_delta_x;
            t_max_z += t_delta_z;

            // If Later We Hit Diagonal Wall Tile at 
            // Same Distance, Pick Consistent Normal
            step_normal = if dx.abs() >= dz.abs() {
                Vec3::new(-(step_x as f32), 0.0, 0.0)
            } else {
                Vec3::new(0.0, 0.0, -(step_z as f32))
            };

            dist
        };

        if dist > max_dist {
            return None;
        }

        // Bounds
        if ix < 0 || iz < 0 || ix >= grid.width as i32 || iz >= grid.height as i32 {
            return None;
        }

        // Intentionally Do NOT Stop on SolidStatics Here Either
        // Movement Collision Still Uses SolidStatics
        // Hitscan Should Pass Through Decorations

        let tile = grid.tile(ix as usize, iz as usize);

        // Stops on Walls + Closed Doors
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
