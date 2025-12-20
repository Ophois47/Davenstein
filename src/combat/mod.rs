use bevy::prelude::*;
use davelib::map::MapGrid;
use davelib::audio::{PlaySfx, SfxKind};
mod hitscan;
use hitscan::raycast_grid;

#[derive(Message, Debug, Clone, Copy)]
pub struct FireShot {
    pub origin: Vec3,
    pub dir: Vec3,
    pub max_dist: f32,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FireShot>()
            .add_systems(Update, process_fire_shots);
    }
}

fn process_fire_shots(
    grid: Res<MapGrid>,
    mut shots: MessageReader<FireShot>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    for shot in shots.read() {
        let Some(hit) = raycast_grid(&grid, shot.origin, shot.dir, shot.max_dist) else {
            continue;
        };

        // TEMP TEST FEEDBACK (easy to remove later):
        // play a “bullet hit wall” sound at the impact point.
        //
        // Add a variant + asset mapping for this in audio:
        // SfxKind::ShootWall
        if matches!(hit.tile, davelib::map::Tile::Wall | davelib::map::Tile::DoorClosed) {
            sfx.write(PlaySfx {
                kind: SfxKind::ShootWall,
                pos: Vec3::new(hit.pos.x, 0.6, hit.pos.z),
            });
        }
        if hit.tile_coord.x < 0 {
            // TEMP: reuse ricochet, or use a separate thud later
            sfx.write(PlaySfx {
                kind: SfxKind::ShootWall, // or BulletHitFloor if you add it
                pos: Vec3::new(hit.pos.x, 0.6, hit.pos.z),
            });
            continue;
        }
    }
}
