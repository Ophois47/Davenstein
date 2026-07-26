/*
Davenstein - by David Petnick

Central Home for Player Input

Pipeline
Devices -> Source Systems -> PlayerIntent -> Gameplay -> World

InputPlugin owns everything that produces intent, including device reads, cursor
capture, and touch control geometry
Gameplay systems that consume intent remain registered in main.rs because their
run conditions reference binary crate resources such as LevelComplete that
davelib cannot access
*/

pub mod intent;
pub mod cursor;
pub mod sources;
pub mod gather;
pub mod menu;
pub mod touch_layout;

use bevy::prelude::*;

pub use intent::PlayerIntent;
pub use menu::MenuNav;
pub use sources::touch::TouchAssignments;
pub use touch_layout::TouchLayout;

// System Set Containing Per-Frame Intent Gathering
// Order Consumers After This Set When They Must Read Fresh Intent in the Same Schedule
// Example in main.rs: apply_look.after(davelib::input::InputGather)
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputGather;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlayerIntent>()
            .init_resource::<MenuNav>()
            .init_resource::<TouchAssignments>()
            .init_resource::<TouchLayout>()
            .add_systems(
                Update,
                // Geometry Must Be Current Before Anything Hit-Tests Against It, so
                // This Is Ordered Before the Gather Set Rather Than Merely Beside It.
                // A Frame of Stale Layout After a Rotation or Resize Would Put Every
                // Button Somewhere the Player Is No Longer Touching
                touch_layout::update_touch_layout.before(InputGather),
            )
            .add_systems(
                Update,
                gather::gather.in_set(InputGather),
            )
            .add_systems(Update, cursor::grab_mouse);
    }
}
