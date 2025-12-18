use bevy::prelude::*;
use bevy::audio::{
	AudioPlayer,
	AudioSource,
	PlaybackSettings,
    SpatialScale,
    Volume,
};

#[derive(Clone, Copy, Debug)]
pub enum SfxKind {
    DoorOpen,
    DoorClose,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct PlaySfx {
    pub kind: SfxKind,
    pub pos: Vec3,
}

#[derive(Resource)]
pub struct GameAudio {
    pub door_open: Handle<AudioSource>,
    pub door_close: Handle<AudioSource>,
    pub music_level: Handle<AudioSource>,
}

#[derive(Component)]
pub struct Music;

pub fn setup_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(GameAudio {
        door_open: asset_server.load("sounds/sfx/door_open.ogg"),
        door_close: asset_server.load("sounds/sfx/door_close.ogg"),
        music_level: asset_server.load("sounds/music/level1.ogg"),
    });
}

pub fn start_music(
    mut commands: Commands,
    audio: Res<GameAudio>,
    q_music: Query<(), With<Music>>,
) {
    // prevent duplicates if Startup runs again (hot reload etc)
    if q_music.iter().next().is_some() {
        return;
    }

    commands.spawn((
        Music,
        AudioPlayer::new(audio.music_level.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.45)),
    ));
}

pub fn play_sfx_events(
    audio: Res<GameAudio>,
    mut commands: Commands,
    mut ev: MessageReader<PlaySfx>,
) {
    for e in ev.read() {
        let (clip, settings) = match e.kind {
            SfxKind::DoorOpen => (
                audio.door_open.clone(),
                PlaybackSettings::DESPAWN
                    .with_spatial(true)
                    .with_spatial_scale(SpatialScale::new(0.60)),
            ),
            SfxKind::DoorClose => (
                audio.door_close.clone(),
                PlaybackSettings::DESPAWN
                    .with_spatial(true)
                    // smaller scale => "audio distance" grows slower => audible farther
                    .with_spatial_scale(SpatialScale::new(0.15))
                    .with_volume(Volume::Linear(1.25)),
            ),
        };

        commands.spawn((
            Transform::from_translation(e.pos),
            AudioPlayer::new(clip),
            settings,
        ));
    }
}
