/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorTile(pub IVec2); // (X, Z) in Tile Coords

#[derive(Component, Debug, Clone, Copy)]
pub struct DoorState {
    // Seconds Remaining While Open (Countdown Starts When Fully Open)
    // 0 = No Pending Close
    pub open_timer: f32,
    // Door Becomes Passable Once Fully Open
    pub want_open: bool,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DoorAnim {
    pub progress: f32,    // 0.0 = Closed, 1.0 = Open
    pub closed_pos: Vec3, // World-space Position When Fully Closed
    pub slide_axis: Vec3, // World-space Unit Direction to Slide Into Wall
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Wall,
    DoorClosed,
    DoorOpen,
}

#[derive(Resource, Debug, Clone)]
pub struct MapGrid {
    pub width: usize,
    pub height: usize,
    pub plane0: Vec<u16>,
    pub tiles: Vec<Tile>,
}

impl MapGrid {
    pub fn idx(&self, x: usize, z: usize) -> usize {
        z * self.width + x
    }

    pub fn tile(&self, x: usize, z: usize) -> Tile {
        self.tiles[self.idx(x, z)]
    }

    /// Raw Wolfenstein 3D plane0 Code at (X,Z). For Walls, this Wall Texture ID
    pub fn set_tile(&mut self, x: usize, z: usize, t: Tile) {
        let i = self.idx(x, z);
        self.tiles[i] = t;
    }
    
    pub fn plane0_code(&self, x: usize, z: usize) -> u16 {
        self.plane0[self.idx(x, z)]
    }

    pub fn set_plane0_code(&mut self, x: usize, z: usize, code: u16) {
        let i = self.idx(x, z);
        self.plane0[i] = code;
    }

    pub fn from_ascii(lines: &[&str]) -> (Self, Option<IVec2>, Vec<IVec2>) {
        let height = lines.len();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

        let mut plane0: Vec<u16> = Vec::with_capacity(width * height);
        let mut tiles: Vec<Tile> = Vec::with_capacity(width * height);

        let mut player_spawn: Option<IVec2> = None;
        let mut guards: Vec<IVec2> = Vec::new();

        for (z, line) in lines.iter().enumerate() {
            let mut chars = line.chars().collect::<Vec<_>>();
            while chars.len() < width {
                chars.push(' ');
            }

            for (x, c) in chars.into_iter().enumerate() {
                match c {
                    '#' => {
                        plane0.push(1);
                        tiles.push(Tile::Wall);
                    }
                    'D' => {
                        plane0.push(90);
                        tiles.push(Tile::DoorClosed);
                    }
                    'P' => {
                        plane0.push(0);
                        tiles.push(Tile::Empty);
                        player_spawn = Some(IVec2::new(x as i32, z as i32));
                    }
                    'G' => {
                        plane0.push(0);
                        tiles.push(Tile::Empty);
                        guards.push(IVec2::new(x as i32, z as i32));
                    }
                    '.' | ' ' => {
                        plane0.push(0);
                        tiles.push(Tile::Empty);
                    }
                    _ => {
                        plane0.push(0);
                        tiles.push(Tile::Empty);
                    }
                }
            }
        }

        (
            Self {
                width,
                height,
                plane0,
                tiles,
            },
            player_spawn,
            guards,
        )
    }

    pub fn parse_u16_grid(text: &str, width: usize, height: usize) -> Vec<u16> {
        let mut out = Vec::with_capacity(width * height);

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            for tok in line.split_whitespace() {
                if let Ok(v) = tok.parse::<u16>() {
                    out.push(v);
                } else {
                    bevy::log::warn!("parse_u16_grid: failed to parse token '{tok}'");
                }
            }
        }

        if out.len() != width * height {
            bevy::log::warn!(
                "parse_u16_grid: expected {} values ({}x{}), got {}",
                width * height,
                width,
                height,
                out.len()
            );
            out.resize(width * height, 0);
        }

        out
    }

    /// Convert Wolf3D plane0/plane1 data into our current collision grid + basic spawns.
    /// - plane0: walls/doors/floors (1-63=wall, 90-95/100-101=door, otherwise walkable)
    /// - plane1: things (19-22=player start, 108-115=guards any difficulty)
    pub fn from_wolf_planes(
        width: usize,
        height: usize,
        plane0: &[u16],
        plane1: &[u16],
    ) -> (
        Self,
        Option<(IVec2, f32)>,
        Vec<IVec2>,
        Vec<IVec2>,
        Vec<IVec2>,
        Vec<IVec2>,
    ) {
        let mut raw_plane0: Vec<u16> = Vec::with_capacity(width * height);
        let mut tiles: Vec<Tile> = Vec::with_capacity(width * height);

        let mut player_spawn: Option<(IVec2, f32)> = None;
        let mut guards: Vec<IVec2> = Vec::new();
        let mut ss: Vec<IVec2> = Vec::new();
        let mut dogs: Vec<IVec2> = Vec::new();
        let mut hans: Vec<IVec2> = Vec::new();

        let idx = |x: usize, z: usize| -> usize { z * width + x };

        for z in 0..height {
            for x in 0..width {
                let v0 = plane0[idx(x, z)];
                let v1 = plane1[idx(x, z)];

                raw_plane0.push(v0);

                // Plane0 Collision:
                //   1..=63    => Walls
                //   90..=101  => Doors (Normal / Locked / Elevator)
                //   Otherwise => Walkable
                if (1..=63).contains(&v0) {
                    tiles.push(Tile::Wall);
                } else if (90..=101).contains(&v0) {
                    tiles.push(Tile::DoorClosed);
                } else {
                    tiles.push(Tile::Empty);
                }

                // Player Start: 19..=22 (N/E/S/W)
                if (19..=22).contains(&v1) && player_spawn.is_none() {
                    let yaw = match v1 {
                        19 => 0.0,
                        20 => -std::f32::consts::FRAC_PI_2, // East  (+X)
                        21 => std::f32::consts::PI,         // South (+Z)
                        22 => std::f32::consts::FRAC_PI_2,  // West  (-X)
                        _ => 0.0,
                    };
                    player_spawn = Some((IVec2::new(x as i32, z as i32), yaw));
                }

                let t = IVec2::new(x as i32, z as i32);

                // Enemies: Include the any difficulty codes, plus the medium/hard ranges,
                // because E1M2 plane1 clearly contains values outside the 108..=115 set
                // (NOT implementing difficulty selection yet — we're just ensuring they spawn)
                if (108..=115).contains(&v1) || (144..=151).contains(&v1) || (180..=187).contains(&v1) {
                    guards.push(t);
                } else if (126..=133).contains(&v1) || (162..=169).contains(&v1) || (198..=205).contains(&v1) {
                    ss.push(t);
                } else if (134..=141).contains(&v1) || (170..=177).contains(&v1) || (206..=213).contains(&v1) {
                    dogs.push(t);
                } else if v1 == 214 {
                    // Boss Hans Grosse in E1M9
                    hans.push(t);
                }
            }
        }

        (
            Self {
                width,
                height,
                plane0: raw_plane0,
                tiles,
            },
            player_spawn,
            guards,
            ss,
            dogs,
            hans,
        )
    }
}
