/*
Davenstein - by David Petnick

Central Home for Player Input

Pipeline
Devices -> Source Systems -> PlayerIntent -> Gameplay -> World

InputPlugin owns everything that produces intent, including device reads and
cursor capture
Gameplay systems that consume intent remain registered in main.rs because their
run conditions reference binary crate resources such as LevelComplete that
davelib cannot access
*/

pub mod intent;
pub mod cursor;
pub mod sources;

use bevy::prelude::*;

pub use intent::PlayerIntent;

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
            .add_systems(
                Update,
                sources::keyboard_mouse::gather_input.in_set(InputGather),
            )
            .add_systems(Update, cursor::grab_mouse);
    }
}
