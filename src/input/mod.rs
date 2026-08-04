/*
Davenstein - by David Petnick

Central Home for Player Input

Pipeline
Devices -> Source Systems -> PlayerIntent -> Gameplay -> World

InputPlugin owns everything that produces intent, including device reads, cursor
capture, and touch control geometry
Gameplay systems that consume intent remain registered in the shared application host
because their run conditions depend on application-owned resources such as
LevelComplete
*/

pub mod intent;
pub mod cursor;
pub mod devices;
pub mod sources;
pub mod gather;
pub mod menu;
pub mod touch_layout;

use bevy::prelude::*;

pub use devices::{ActiveGamepad, ActiveInputDevice};
pub use intent::PlayerIntent;
pub use menu::MenuNav;
pub use sources::touch::{TouchAssignments, TouchUiMode};
pub use touch_layout::TouchLayout;

// System Set Containing Per-Frame Intent Gathering
// Order Consumers After This Set When They Must Read Fresh Intent in the Same Schedule
// Example in app.rs: apply_look.after(davelib::input::InputGather)
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputGather;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlayerIntent>()
            .init_resource::<MenuNav>()
            .init_resource::<TouchAssignments>()
            .init_resource::<TouchUiMode>()
            .init_resource::<TouchLayout>()
            .init_resource::<ActiveGamepad>()
            .init_resource::<ActiveInputDevice>()
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
                // Before the Gather Set for the Same Reason the Touch Layout Is: the
                // Gamepad Source Reads the Binding This Maintains, and a Frame of Stale
                // Binding After a Disconnect Would Read a Device That No Longer Exists
                devices::bind_active_gamepad.before(InputGather),
            )
            .add_systems(
                Update,
                devices::log_gamepad_input.before(InputGather),
            )
            .add_systems(
                Update,
                gather::gather.in_set(InputGather),
            )
            .add_systems(Update, cursor::grab_mouse);
    }
}
