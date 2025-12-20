use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorTile(pub IVec2); // (x, z) in tile coords

#[derive(Component, Debug, Clone, Copy)]
pub struct DoorState {
    // Seconds remaining while open (countdown starts once fully open)
    // 0 = no pending close
    pub open_timer: f32,
    // Door only becomes passable once fully open
    pub want_open: bool,
}


#[derive(Component, Debug, Clone, Copy)]
pub struct DoorAnim {
    pub progress: f32,    // 0.0 = closed, 1.0 = open
    pub closed_pos: Vec3, // world-space position when fully closed
    pub slide_axis: Vec3, // world-space unit direction to slide into the wall
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

    // ASCII Legend:
    // '#' = wall, '.' or ' ' = empty, 'P' = player spawn (treated as empty)
    // 'G' = guard spawn (treated as empty)
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
}

