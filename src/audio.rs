use bevy::prelude::*;
use bevy::audio::{
	AudioPlayer,
	AudioSource,
	PlaybackSettings,
    SpatialScale,
    Volume,
};
use std::collections::HashMap;
use rand::Rng;
use crate::enemies::EnemyKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxKind {
    // World
    DoorOpen,
    DoorClose,
    
    // Weapons
    KnifeSwing,
    KnifeHit,
    PistolFire,
    MachineGunFire,
    ChaingunFire,

    // Enemies
    EnemyDeath(EnemyKind),
}

#[derive(Clone, Copy, Debug, Message)]
pub struct PlaySfx {
    pub kind: SfxKind,
    pub pos: Vec3,
}

#[derive(Resource, Default)]
pub struct SfxLibrary {
    pub map: HashMap<SfxKind, Vec<Handle<AudioSource>>>,
}

impl SfxLibrary {
    pub fn insert_one(&mut self, k: SfxKind, h: Handle<AudioSource>) {
        self.map.entry(k).or_default().push(h);
    }
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
    // Keep your existing "single handles" struct for music etc.
    commands.insert_resource(GameAudio {
        door_open: asset_server.load("sounds/sfx/door_open.ogg"),
        door_close: asset_server.load("sounds/sfx/door_close.ogg"),
        music_level: asset_server.load("sounds/music/level1.ogg"),
    });

    // NEW: library that supports 1-or-many clips per SfxKind (random selection happens later)
    let mut lib = SfxLibrary::default();

    // Door / weapon SFX (single-clip entries still go through the library)
    lib.insert_one(SfxKind::DoorOpen, asset_server.load("sounds/sfx/door_open.ogg"));
    lib.insert_one(SfxKind::DoorClose, asset_server.load("sounds/sfx/door_close.ogg"));
    lib.insert_one(SfxKind::KnifeSwing, asset_server.load("sounds/sfx/weapons/knife/knife_swing.ogg"));
    lib.insert_one(SfxKind::KnifeHit, asset_server.load("sounds/sfx/weapons/knife/knife_hit.ogg"));
    lib.insert_one(SfxKind::PistolFire, asset_server.load("sounds/sfx/weapons/pistol/pistol_fire.ogg"));
    lib.insert_one(SfxKind::MachineGunFire, asset_server.load("sounds/sfx/weapons/machinegun/machinegun_fire_0.ogg"));
    lib.insert_one(SfxKind::ChaingunFire, asset_server.load("sounds/sfx/weapons/chaingun/chaingun_fire_0.ogg"));

    // Guard death set (random pick in play_sfx_events)
    lib.insert_one(SfxKind::EnemyDeath(EnemyKind::Guard), asset_server.load("sounds/sfx/enemies/guard/guard_death_0.ogg"));
    lib.insert_one(SfxKind::EnemyDeath(EnemyKind::Guard), asset_server.load("sounds/sfx/enemies/guard/guard_death_1.ogg"));
    lib.insert_one(SfxKind::EnemyDeath(EnemyKind::Guard), asset_server.load("sounds/sfx/enemies/guard/guard_death_2.ogg"));
    lib.insert_one(SfxKind::EnemyDeath(EnemyKind::Guard), asset_server.load("sounds/sfx/enemies/guard/guard_death_3.ogg"));
    lib.insert_one(SfxKind::EnemyDeath(EnemyKind::Guard), asset_server.load("sounds/sfx/enemies/guard/guard_death_4.ogg"));
    lib.insert_one(SfxKind::EnemyDeath(EnemyKind::Guard), asset_server.load("sounds/sfx/enemies/guard/guard_death_5.ogg"));
    lib.insert_one(SfxKind::EnemyDeath(EnemyKind::Guard), asset_server.load("sounds/sfx/enemies/guard/guard_death_6.ogg"));

    commands.insert_resource(lib);
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
    lib: Res<SfxLibrary>,
    mut commands: Commands,
    mut ev: MessageReader<PlaySfx>,
) {
    for e in ev.read() {
        let Some(list) = lib.map.get(&e.kind) else {
            warn!("Missing SFX for {:?}", e.kind);
            continue;
        };
        if list.is_empty() {
            continue;
        }

        // Random choice (works for list len 1 too)
        let i = rand::rng().random_range(0..list.len());
        let clip = list[i].clone();

        // Settings by kind (keep your current style/values)
        let settings = match e.kind {
            SfxKind::DoorOpen => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.25)),

            SfxKind::DoorClose => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.25)),

            SfxKind::KnifeSwing => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.2)),

            SfxKind::KnifeHit => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.2)),
                
            SfxKind::PistolFire => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_speed(1.0)
                .with_volume(Volume::Linear(1.5)),

            SfxKind::MachineGunFire => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_speed(1.0)
                .with_volume(Volume::Linear(1.5)),

            SfxKind::ChaingunFire => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_speed(1.0)
                .with_volume(Volume::Linear(1.5)),

            SfxKind::EnemyDeath(_) => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.3)),
        };

        commands.spawn((
            Transform::from_translation(e.pos),
            AudioPlayer::new(clip),
            settings,
        ));
    }
}
