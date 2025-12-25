/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

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
    pub tiles: Vec<Tile>,
}

impl MapGrid {
    pub fn idx(&self, x: usize, z: usize) -> usize {
        z * self.width + x
    }

    pub fn tile(&self, x: usize, z: usize) -> Tile {
        self.tiles[self.idx(x, z)]
    }

    pub fn set_tile(&mut self, x: usize, z: usize, t: Tile) {
    	let i  = self.idx(x, z);
    	self.tiles[i] = t;
    }
    
    pub fn from_ascii(lines: &[&str]) -> (Self, Option<IVec2>, Vec<IVec2>) {
        let height = lines.len();
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

        let mut tiles = Vec::with_capacity(width * height);
        let mut player_spawn: Option<IVec2> = None;
        let mut guards: Vec<IVec2> = Vec::new();

        for (z, line) in lines.iter().enumerate() {
            let mut chars = line.chars().collect::<Vec<_>>();
            while chars.len() < width {
                chars.push(' ');
            }

            for (x, c) in chars.into_iter().enumerate() {
                match c {
                    '#' => tiles.push(Tile::Wall),
                    'D' => tiles.push(Tile::DoorClosed),
                    'O' => tiles.push(Tile::DoorOpen),
                    'P' => {
                        tiles.push(Tile::Empty);
                        player_spawn = Some(IVec2::new(x as i32, z as i32));
                    }
                    'G' => {
                        tiles.push(Tile::Empty);
                        guards.push(IVec2::new(x as i32, z as i32));
                    }
                    '.' | ' ' => tiles.push(Tile::Empty),
                    _ => tiles.push(Tile::Empty),
                }
            }
        }

        (
            Self { width, height, tiles },
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
    /// - plane0: walls/doors/floors (0-63=wall, 90-95/100-101=door, otherwise walkable)
    /// - plane1: things (19-22=player start, 108-115=guards any difficulty)
    pub fn from_wolf_planes(
        width: usize,
        height: usize,
        plane0: &[u16],
        plane1: &[u16],
    ) -> (Self, Option<(IVec2, f32)>, Vec<IVec2>) {
        let mut tiles = Vec::with_capacity(width * height);
        let mut player_spawn: Option<(IVec2, f32)> = None;
        let mut guards: Vec<IVec2> = Vec::new();

        let idx = |x: usize, z: usize| -> usize { z * width + x };

        for z in 0..height {
            for x in 0..width {
                let v0 = plane0[idx(x, z)];
                let v1 = plane1[idx(x, z)];

                // Plane0 -> collision tile
                let tile = if v0 <= 63 {
                    Tile::Wall
                } else if (90..=95).contains(&v0) || (100..=101).contains(&v0) {
                    Tile::DoorClosed
                } else {
                    Tile::Empty
                };
                tiles.push(tile);

                // Plane1 -> spawns
                if (19..=22).contains(&v1) {
                    // 19=N, 20=E, 21=S, 22=W
                    let yaw = match v1 {
                        19 => 0.0,
                        20 => -FRAC_PI_2,
                        21 => PI,
                        22 => FRAC_PI_2,
                        _ => 0.0,
                    };
                    player_spawn = Some((IVec2::new(x as i32, z as i32), yaw));
                }

                // Guards: any-difficulty standing/patrol (108-115)
                if (108..=115).contains(&v1) {
                    guards.push(IVec2::new(x as i32, z as i32));
                }
            }
        }

        (
            Self { width, height, tiles },
            player_spawn,
            guards,
        )
    }

}

