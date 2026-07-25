/*
Davenstein - by David Petnick

Wolfenstein 3-D's Area Model, Which the AI's Noticing Rules Are Gated On.

The Original Splits Every Level Into Numbered Floor Areas (plane0 Codes AREATILE
and Up) and Joins Two Areas *Only* Through a Door: `areaconnect[a][b]` is Bumped in
`DoorOpening` (WOLFSRC/WL_ACT1.C) and Nowhere Else. `ConnectAreas` /
`RecursiveConnect` Then Flood That Graph From the Player's Area to Fill
`areabyplayer`, and `SightPlayer` Refuses to Notice the Player at All Unless
`areabyplayer[ob->areanumber]` is Set (WOLFSRC/WL_STATE.C).

Open Geometry Never Connects Two Areas. That is the Quirk That Keeps a Gunshot
Local: in a Large Hall Painted With Several Area Codes, Guards in the Far Half
Genuinely Do Not Hear You Even Though You Could Walk to Them Without Opening a
Single Door. A Plain Geometric Flood Fill Collapses Such a Hall Into One Region and
Wakes All of Them, Which is Why This Module Replaces the Old `AreaMap`.
*/

use bevy::prelude::*;

use crate::map::{MapGrid, Tile};

/// plane0 Codes 107 and Up Are the Floor "Area Numbers" (AREATILE, WOLFSRC/WL_DEF.H).
/// Everything Below is a Wall, a Door, or the Ambush Marker
pub const AREATILE: u16 = 107;

/// NUMAREAS From WOLFSRC/WL_DEF.H. Valid Area Numbers Are 0..=36
pub const NUM_AREAS: usize = 37;

/// The Deaf-Guard Marker (WOLFSRC/WL_DEF.H). it Sits on a Walkable Tile but Carries
/// no Area Number of its Own, Which is Why SetupGameLevel Rewrites it to a
/// Neighbouring Floor Code and Why `adopt_missing_areas` Has to Do the Same
pub const AMBUSHTILE: u16 = 106;

/// A Door Tile Together With the Two Areas it Joins. Door Positions Are Fixed for
/// the Life of a Level, so the Link List is Built Once per Topology Change and Only
/// the Door's Open State is Polled Each Tic
#[derive(Debug, Clone, Copy)]
struct DoorLink {
    tile: IVec2,
    a: u8,
    b: u8,
}

/// Wolfenstein 3-D's `areabyplayer`, Solved From the Live Map.
///
/// Topology (the Per-Tile Area Table and the Door Link List) is Cached; the
/// Reachability Solve is Redone Every Tic Because it Walks at Most 37 Areas and a
/// Few Dozen Links.
///
/// Default is Hand-Written Rather Than Derived: std Only Implements `Default` for
/// Arrays up to Length 32, and `reachable` is `[bool; 37]`
#[derive(Resource, Debug)]
pub struct AreaGraph {
    width: usize,
    height: usize,

    /// Per-Tile Area Number, or -1 for Walls, Doors, and Out-of-Range Codes
    tile_area: Vec<i16>,

    /// Every Door Tile With the Two Areas it Joins
    door_links: Vec<DoorLink>,

    /// The Solved `areabyplayer` Table for This Tic
    reachable: [bool; NUM_AREAS],

    /// Last Area the Player Actually Stood In. The Original Only Refreshes
    /// `player->areanumber` From a Floor Tile, so Standing in a Doorway Has to Keep
    /// the Previous Value Rather Than Dropping the Player Out of Every Area
    player_area: Option<u8>,

    /// `MapGrid::generation` the Cached Topology Was Built From
    built_generation: Option<u64>,
}

impl Default for AreaGraph {
    /// Hand-Written Rather Than Derived: std Only Implements `Default` for Arrays up to
    /// Length 32, and `reachable` is `[bool; NUM_AREAS]` With NUM_AREAS == 37
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            tile_area: Vec::new(),
            door_links: Vec::new(),
            // Nothing is Reachable Until the First update_for_player, and No Actor
            // Can Notice the Player Until Then. One Tic of Silence on a Fresh Level
            // is Correct: the Original Also Waits for ConnectAreas
            reachable: [false; NUM_AREAS],
            player_area: None,
            built_generation: None,
        }
    }
}

impl AreaGraph {
    /// Force the Next `sync_topology` to Rebuild. Callers Use This When the
    /// `MapGrid` *Resource* Was Replaced Rather Than Mutated: a Fresh Level Starts
    /// at Generation 0 Again, so the Generation Alone Cannot Tell a New 64x64 Map
    /// From the Untouched Previous One
    pub fn invalidate(&mut self) {
        self.built_generation = None;
    }

    /// Rebuild the Per-Tile Area Table and the Door Link List When the Grid's
    /// Topology Generation Has Moved On. A Cheap No-Op on Every Other Tic
    pub fn sync_topology(&mut self, grid: &MapGrid) {
        if self.built_generation == Some(grid.generation)
            && self.width == grid.width
            && self.height == grid.height
        {
            return;
        }

        self.width = grid.width;
        self.height = grid.height;
        self.built_generation = Some(grid.generation);

        self.read_area_codes(grid);

        // Maps Built Without Wolfenstein Floor Codes (MapGrid::from_ascii Pushes
        // plane0 == 0 for Floor) Would Otherwise End Up With no Areas at All and
        // Every Actor Permanently Blind and Deaf. Fall Back to One Synthetic Area per
        // Door-Separated Region, Which is Exactly What This Port Did Before
        if self.tile_area.iter().all(|a| *a < 0) {
            self.assign_synthetic_areas(grid);
        } else {
            self.adopt_missing_areas(grid);
        }

        self.build_door_links(grid);
    }

    /// Refresh the Player's Area and Re-Solve `areabyplayer` for This Tic.
    /// Mirrors `ConnectAreas` Plus `RecursiveConnect`
    pub fn update_for_player(&mut self, grid: &MapGrid, player_tile: IVec2) {
        // Only a Floor Tile Updates the Player's Area, Exactly as Thrust Does. A
        // Doorway Keeps the Previous Value Instead of Blanking It
        if let Some(a) = self.area_at(player_tile) {
            self.player_area = Some(a);
        }

        self.reachable = [false; NUM_AREAS];

        let Some(start) = self.player_area else {
            return;
        };
        self.reachable[start as usize] = true;

        // RecursiveConnect, Iteratively. Each Pass Propagates Through Every Currently
        // Open Door; With at Most 37 Areas This Settles in a Handful of Passes and
        // Avoids Rebuilding an Adjacency List Every Tic
        let mut changed = true;
        while changed {
            changed = false;

            for link in &self.door_links {
                // The Original Connects Areas the Instant a Door *Starts* Opening.
                // This Port Only Flips the Tile to DoorOpen at Full Open, Which is
                // Stricter and Therefore Safe: Noise Arrives Slightly Later, Never
                // Sooner
                if !matches!(
                    grid.tile(link.tile.x as usize, link.tile.y as usize),
                    Tile::DoorOpen
                ) {
                    continue;
                }

                let (a, b) = (link.a as usize, link.b as usize);
                if self.reachable[a] && !self.reachable[b] {
                    self.reachable[b] = true;
                    changed = true;
                } else if self.reachable[b] && !self.reachable[a] {
                    self.reachable[a] = true;
                    changed = true;
                }
            }
        }
    }

    /// Area Number at a Tile, or None for Walls, Doors, and Out-of-Bounds
    pub fn area_at(&self, t: IVec2) -> Option<u8> {
        if t.x < 0 || t.y < 0 || t.x as usize >= self.width || t.y as usize >= self.height {
            return None;
        }
        let a = self.tile_area[t.y as usize * self.width + t.x as usize];
        if a < 0 { None } else { Some(a as u8) }
    }

    /// The `areabyplayer[ob->areanumber]` Gate. False Means the Actor Cannot Notice
    /// the Player This Tic, Whether by Sight or by Noise.
    ///
    /// An Actor Standing in a Doorway Has no Area of its Own, so Fall Back to its
    /// Neighbours. The Original Never Hits This Case Because `ob->areanumber` is
    /// Sticky, but Here the Actor's Occupied Tile is the Only State We Have
    pub fn hears_player(&self, actor_tile: IVec2) -> bool {
        if let Some(a) = self.area_at(actor_tile) {
            return self.reachable[a as usize];
        }

        for step in [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
        ] {
            if let Some(a) = self.area_at(actor_tile + step) {
                if self.reachable[a as usize] {
                    return true;
                }
            }
        }

        false
    }

    /// plane0 Code a Newly Revealed Floor Tile Should Take, Matching MovePWalls's
    /// `player->areanumber + AREATILE`. Falls Back to the First Area so a Revealed
    /// Tile is Never Left Without One
    pub fn player_area_code(&self) -> u16 {
        AREATILE + u16::from(self.player_area.unwrap_or(0))
    }

    /// Read the plane0 Floor Codes Straight Off the Grid. Only a Tile::Empty Tile Can
    /// Carry an Area Number: a Wall or Door Keeps a Texture or Door Code in That Same
    /// plane0 Slot, so Reading One as an Area Would Invent Connections That Do not Exist.
    /// Codes Below AREATILE and Codes Past NUM_AREAS Are Left at -1 for
    /// `adopt_missing_areas` or `assign_synthetic_areas` to Fill In
    fn read_area_codes(&mut self, grid: &MapGrid) {
        self.tile_area.clear();
        self.tile_area.resize(self.width * self.height, -1);

        for z in 0..self.height {
            for x in 0..self.width {
                // Walls and Doors Never Belong to an Area: Their plane0 Slot Holds a
                // Texture or Door Code, Not a Floor Code
                if !matches!(grid.tile(x, z), Tile::Empty) {
                    continue;
                }

                let code = grid.plane0_code(x, z);
                if code < AREATILE {
                    continue;
                }

                let a = code - AREATILE;
                if (a as usize) < NUM_AREAS {
                    self.tile_area[z * self.width + x] = a as i16;
                }
            }
        }
    }

    /// Walkable Tiles With no Floor Code Adopt a Neighbour's Area. This Covers the
    /// AMBUSHTILE (106) Fixup the Original Performs in `SetupGameLevel`, Any plane0
    /// Code Below AREATILE That This Port Treats as Floor, and Tiles a Pushwall Has
    /// Already Vacated. Without it Such a Tile Would Belong to no Area at All and an
    /// Actor Standing There Could Never Notice the Player
    fn adopt_missing_areas(&mut self, grid: &MapGrid) {
        let w = self.width as i32;
        let h = self.height as i32;

        let mut queue: Vec<IVec2> = Vec::new();
        for z in 0..self.height {
            for x in 0..self.width {
                if self.tile_area[z * self.width + x] >= 0 {
                    queue.push(IVec2::new(x as i32, z as i32));
                }
            }
        }

        let mut head = 0usize;
        while head < queue.len() {
            let cur = queue[head];
            head += 1;

            let area = self.tile_area[cur.y as usize * self.width + cur.x as usize];

            for step in [
                IVec2::new(1, 0),
                IVec2::new(-1, 0),
                IVec2::new(0, 1),
                IVec2::new(0, -1),
            ] {
                let n = cur + step;
                if n.x < 0 || n.y < 0 || n.x >= w || n.y >= h {
                    continue;
                }

                // Spread Through Open Floor Only. Door Tiles Stay Area-Less on
                // Purpose; They Are Represented by the Link List Instead
                if !matches!(grid.tile(n.x as usize, n.y as usize), Tile::Empty) {
                    continue;
                }

                let ni = n.y as usize * self.width + n.x as usize;
                if self.tile_area[ni] >= 0 {
                    continue;
                }

                self.tile_area[ni] = area;
                queue.push(n);
            }
        }
    }

    /// One Synthetic Area per Door-Separated Region, for Maps That Carry no
    /// Wolfenstein Floor Codes. Regions Beyond NUM_AREAS Share the Last Slot, Which
    /// Can Only Over-Connect on Maps That Have no Real Area Data to Begin With
    fn assign_synthetic_areas(&mut self, grid: &MapGrid) {
        let w = self.width as i32;
        let h = self.height as i32;
        let mut next_id: usize = 0;

        for z in 0..self.height {
            for x in 0..self.width {
                let idx = z * self.width + x;
                if self.tile_area[idx] >= 0 {
                    continue;
                }
                if !matches!(grid.tile(x, z), Tile::Empty) {
                    continue;
                }

                let id = next_id.min(NUM_AREAS - 1) as i16;
                self.tile_area[idx] = id;

                let mut stack = vec![IVec2::new(x as i32, z as i32)];
                while let Some(p) = stack.pop() {
                    for step in [
                        IVec2::new(1, 0),
                        IVec2::new(-1, 0),
                        IVec2::new(0, 1),
                        IVec2::new(0, -1),
                    ] {
                        let n = p + step;
                        if n.x < 0 || n.y < 0 || n.x >= w || n.y >= h {
                            continue;
                        }
                        if !matches!(grid.tile(n.x as usize, n.y as usize), Tile::Empty) {
                            continue;
                        }
                        let ni = n.y as usize * self.width + n.x as usize;
                        if self.tile_area[ni] >= 0 {
                            continue;
                        }
                        self.tile_area[ni] = id;
                        stack.push(n);
                    }
                }

                next_id += 1;
            }
        }
    }

    /// Index Every Door Tile Against the Two Areas it Joins. This is the Standing
    /// Equivalent of the Original's `areaconnect` Matrix, Except the Original Bumps a
    /// Counter as Each Door Opens Whereas This Records the Adjacency Once and Lets
    /// `update_for_player` Poll the Live Open State. Safe Because Door Positions Never
    /// Move Within a Level -- Only Whether They Are Open Changes
    fn build_door_links(&mut self, grid: &MapGrid) {
        self.door_links.clear();

        for z in 0..self.height {
            for x in 0..self.width {
                if !matches!(grid.tile(x, z), Tile::DoorClosed | Tile::DoorOpen) {
                    continue;
                }

                let t = IVec2::new(x as i32, z as i32);

                // A Door Sits in a Wall Run, so Exactly One Axis Has Floor on Both
                // Sides. Probe Both and Keep the First That Resolves Two Areas.
                // Back-to-Back Door Tiles Resolve Neither and Are Skipped, Which
                // Under-Connects Rather Than Over-Connects and So Fails Safe
                for axis in [IVec2::new(1, 0), IVec2::new(0, 1)] {
                    let (Some(a), Some(b)) = (self.area_at(t - axis), self.area_at(t + axis))
                    else {
                        continue;
                    };
                    self.door_links.push(DoorLink { tile: t, a, b });
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a MapGrid From a Small ASCII Floor Plan so Each Test Reads as a Map.
    /// Legend: '#' Wall, 'D' Closed Door, 'd' Open Door, '1'..'9' Floor Carrying Area
    /// Code AREATILE + Digit, 'a' AMBUSHTILE Floor, '.' Floor With no plane0 Code
    fn grid(rows: &[&str]) -> MapGrid {
        let height = rows.len();
        let width = rows[0].chars().count();
        assert!(
            rows.iter().all(|r| r.chars().count() == width),
            "ragged floor plan"
        );

        let mut plane0 = Vec::with_capacity(width * height);
        let mut tiles = Vec::with_capacity(width * height);

        for row in rows {
            for c in row.chars() {
                let (tile, code) = match c {
                    '#' => (Tile::Wall, 1u16),
                    'D' => (Tile::DoorClosed, 90u16),
                    'd' => (Tile::DoorOpen, 90u16),
                    'a' => (Tile::Empty, AMBUSHTILE),
                    '.' => (Tile::Empty, 0u16),
                    '1'..='9' => (Tile::Empty, AREATILE + (c as u16 - '0' as u16)),
                    other => panic!("unknown floor plan char {other:?}"),
                };
                tiles.push(tile);
                plane0.push(code);
            }
        }

        MapGrid {
            width,
            height,
            plane0,
            tiles,
            generation: 0,
        }
    }

    /// Build the Grid, Solve areabyplayer for a Player Standing at `player`, and Hand
    /// Both Back so a Test Can Keep Mutating Doors and Re-Solving
    fn solve(rows: &[&str], player: IVec2) -> (AreaGraph, MapGrid) {
        let g = grid(rows);
        let mut a = AreaGraph::default();
        a.sync_topology(&g);
        a.update_for_player(&g, player);
        (a, g)
    }

    fn t(x: i32, z: i32) -> IVec2 {
        IVec2::new(x, z)
    }

    #[test]
    fn closed_door_blocks_noise() {
        let (a, _g) = solve(&["#####", "#1D2#", "#####"], t(1, 1));

        assert!(a.hears_player(t(1, 1)), "player's own area must be reachable");
        assert!(
            !a.hears_player(t(3, 1)),
            "a closed door must not join two areas"
        );
    }

    #[test]
    fn open_door_joins_two_areas() {
        let (a, _g) = solve(&["#####", "#1d2#", "#####"], t(1, 1));

        assert!(a.hears_player(t(3, 1)), "an open door must join two areas");
    }

    #[test]
    fn opening_a_door_is_picked_up_without_a_topology_rebuild() {
        let (mut a, mut g) = solve(&["#####", "#1D2#", "#####"], t(1, 1));
        assert!(!a.hears_player(t(3, 1)));

        // Door Links Are Position-Based, so Only the Live Tile State Has to Change.
        // This is the Path door_animate Takes When a Door Finishes Opening
        g.set_tile(2, 1, Tile::DoorOpen);
        a.update_for_player(&g, t(1, 1));

        assert!(a.hears_player(t(3, 1)));
    }

    #[test]
    fn open_geometry_never_joins_two_areas() {
        // The Whole Point of the Area Model: Two Differently Coded Floor Tiles Sitting
        // Side by Side With no Door Between Them Are NOT Connected, Even Though You
        // Could Walk From One to the Other. A Geometric Flood Fill Would Merge Them
        let (a, _g) = solve(&["####", "#12#", "####"], t(1, 1));

        assert!(a.hears_player(t(1, 1)));
        assert!(
            !a.hears_player(t(2, 1)),
            "areaconnect is bumped by doors only, never by open geometry"
        );
    }

    #[test]
    fn a_wall_blocks_the_link() {
        // An Unpushed Pushwall is Just a Wall Tile, Which is Why a Guard Inside a
        // Secret Room Cannot Hear Gunfire Until the Wall Has Moved
        let (a, _g) = solve(&["#####", "#1#2#", "#####"], t(1, 1));

        assert!(!a.hears_player(t(3, 1)));
    }

    #[test]
    fn reachability_is_transitive_through_open_doors() {
        let (mut a, mut g) = solve(&["#######", "#1d2d3#", "#######"], t(1, 1));
        assert!(a.hears_player(t(5, 1)), "two open doors must chain");

        // Shut the Near Door and the Far Room Drops Out Again
        g.set_tile(2, 1, Tile::DoorClosed);
        a.update_for_player(&g, t(1, 1));

        assert!(!a.hears_player(t(3, 1)));
        assert!(!a.hears_player(t(5, 1)));
    }

    #[test]
    fn ambush_tile_adopts_a_neighbouring_area() {
        // SetupGameLevel Rewrites AMBUSHTILE to a Neighbouring Floor Code. Without the
        // Equivalent Fixup a Guard Standing on One Would Belong to no Area and Could
        // Never Notice the Player at All
        let (a, _g) = solve(&["####", "#1a#", "####"], t(1, 1));

        assert_eq!(a.area_at(t(2, 1)), Some(1));
        assert!(a.hears_player(t(2, 1)));
    }

    #[test]
    fn codeless_floor_adopts_a_neighbouring_area() {
        // Covers Tiles a Pushwall Has Vacated and Any plane0 Code This Port Treats as
        // Floor but the Original Did Not Give an Area Number
        let (a, _g) = solve(&["#####", "#1..#", "#####"], t(1, 1));

        assert_eq!(a.area_at(t(2, 1)), Some(1));
        assert_eq!(a.area_at(t(3, 1)), Some(1));
    }

    #[test]
    fn maps_without_any_area_codes_fall_back_to_synthetic_areas() {
        // MapGrid::from_ascii Pushes plane0 == 0 for Floor. A Strict Port Would Leave
        // Every Tile Area-Less and Every Actor Permanently Blind and Deaf, so the
        // Fallback Has to Produce One Area per Door-Separated Region
        let (mut a, mut g) = solve(&["#####", "#.D.#", "#####"], t(1, 1));

        assert!(
            a.hears_player(t(1, 1)),
            "the AI must not go inert on a code-less map"
        );
        assert!(!a.hears_player(t(3, 1)), "the closed door still separates");

        g.set_tile(2, 1, Tile::DoorOpen);
        a.update_for_player(&g, t(1, 1));
        assert!(a.hears_player(t(3, 1)));
    }

    #[test]
    fn standing_in_a_doorway_keeps_the_previous_player_area() {
        // Thrust Only Refreshes player->areanumber From a Floor Tile. Blanking it in a
        // Doorway Would Drop the Player Out of Every Area for a Tic
        let (mut a, g) = solve(&["#####", "#1d2#", "#####"], t(1, 1));
        assert_eq!(a.player_area_code(), AREATILE + 1);

        a.update_for_player(&g, t(2, 1));

        assert_eq!(
            a.player_area_code(),
            AREATILE + 1,
            "a doorway must not clear the player's area"
        );
        assert!(a.hears_player(t(1, 1)));
    }

    #[test]
    fn an_actor_in_a_doorway_still_notices_via_its_neighbours() {
        // A Door Tile Carries no Area of its Own. Falling Back to the Four Neighbours
        // Stops an Actor Freezing While it Stands in an Open Doorway
        let (a, _g) = solve(&["#####", "#1d2#", "#####"], t(1, 1));

        assert_eq!(a.area_at(t(2, 1)), None);
        assert!(a.hears_player(t(2, 1)));
    }

    #[test]
    fn player_area_code_round_trips_for_the_pushwall_reveal() {
        // use_pushwalls Stamps This Onto Revealed Tiles, Matching MovePWalls's
        // player->areanumber + AREATILE
        let (a, _g) = solve(&["#####", "#5#6#", "#####"], t(1, 1));

        assert_eq!(a.player_area_code(), AREATILE + 5);
    }

    #[test]
    fn out_of_bounds_tiles_have_no_area() {
        let (a, _g) = solve(&["####", "#12#", "####"], t(1, 1));

        assert_eq!(a.area_at(t(-1, 1)), None);
        assert_eq!(a.area_at(t(99, 1)), None);
        assert!(!a.hears_player(t(-5, -5)));
    }
}
