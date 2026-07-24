/*
Davenstein - by David Petnick

Settings Persistence - Load Player Options at Startup and Save Them on Change

Layered Like save/: 'model' Is the Serializable DTO Plus Translation, 'store' Is
the Filesystem Seam. This Module Wires Them Into the App:

- A 'PreStartup' Load That Overwrites the Default Option Resources Before
  OptionsPlugin's 'Startup' Apply Systems Run, so Loaded Values Are Honored on the
  Very First Frame (the Apply Systems Do the Honoring; We Only Feed Them Values).
- A Debounced 'Update' Save That Writes Whenever Any Option Resource Changes,
  Coalescing a Burst of Menu Edits Into a Single Disk Write.

Key Bindings Are Not Persisted Yet (See model.rs); Everything Else Round-Trips
*/

pub mod model;
pub mod store;

use bevy::prelude::*;

use davelib::options::{ControlSettings, GameplaySettings, SoundSettings, VideoSettings};

use model::SettingsFile;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app
            // PreStartup Runs Before Startup, so the Loaded Values Are in Place
            // Before OptionsPlugin's 'apply_*_startup' Chain Reads Them
            .add_systems(PreStartup, load_settings)
            .add_systems(Update, save_settings_on_change);
    }
}

/// Read settings.ron and Stamp It Onto the Live Option Resources. A Missing File
/// (First Run) or an Unreadable One Leaves the Defaults in Place
fn load_settings(
    mut video: ResMut<VideoSettings>,
    mut control: ResMut<ControlSettings>,
    mut sound: ResMut<SoundSettings>,
    mut gameplay: ResMut<GameplaySettings>,
) {
    match store::load() {
        Ok(Some(file)) => {
            file.apply(&mut video, &mut control, &mut sound, &mut gameplay);
            info!("Loaded player settings from settings.ron");
        }
        Ok(None) => {
            // No File Yet, or a Version This Build Does Not Understand: Keep Defaults
        }
        Err(e) => {
            warn!("Could not load settings.ron ({e}); using defaults");
        }
    }
}

/// Debounced Save. Any Option Resource Changing Marks the File Dirty and
/// (Re)Arms a Short Timer; the Write Happens Once the Timer Elapses With No
/// Further Changes, so Dragging a Slider or Cycling an Option Does Not Hammer the
/// Disk. Writing Identical Content (e.g. the One-Shot Change the Load Itself
/// Produces at Startup) Is Harmless
fn save_settings_on_change(
    time: Res<Time>,
    video: Res<VideoSettings>,
    control: Res<ControlSettings>,
    sound: Res<SoundSettings>,
    gameplay: Res<GameplaySettings>,
    mut dirty: Local<bool>,
    mut debounce: Local<Option<Timer>>,
) {
    if video.is_changed()
        || control.is_changed()
        || sound.is_changed()
        || gameplay.is_changed()
    {
        *dirty = true;
        *debounce = Some(Timer::from_seconds(0.75, TimerMode::Once));
    }

    if !*dirty {
        return;
    }

    if let Some(timer) = debounce.as_mut() {
        if !timer.tick(time.delta()).finished() {
            return;
        }
    }

    let file = SettingsFile::from_resources(&video, &control, &sound, &gameplay);
    if let Err(e) = store::save(&file) {
        warn!("Could not write settings.ron ({e})");
    }

    *dirty = false;
    *debounce = None;
}
