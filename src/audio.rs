/*
Davenstein - by David Petnick
*/
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
    // Sfx - World
    DoorOpen,
    DoorClose,
    NoWay,
    Pushwall,
    
    // Sfx - Weapons
    KnifeSwing,
    PistolFire,
    MachineGunFire,
    ChaingunFire,

    // Pickups - Weapons
    PickupChaingun,
    PickupMachineGun,
    PickupAmmo,

    // Pickups - Health
    PickupHealthFirstAid,
	PickupHealthDinner,
	PickupHealthDogFood,
	PickupOneUp,

    // Pickups - Treasure
    PickupTreasureCross,
    PickupTreasureChalice,
    PickupTreasureChest,
    PickupTreasureCrown,

    // Enemies
    EnemyAlert(EnemyKind),
    EnemyShoot(EnemyKind),
    EnemyDeath(EnemyKind),
}

#[derive(Clone, Copy, Debug, Message)]
pub struct PlaySfx {
    pub kind: SfxKind,
    pub pos: Vec3,
}

#[derive(Component)]
pub struct ActivePickupSfx;

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
    commands.insert_resource(GameAudio {
        door_open: asset_server.load("sounds/sfx/door_open.ogg"),
        door_close: asset_server.load("sounds/sfx/door_close.ogg"),
        music_level: asset_server.load("sounds/music/level1.ogg"),
    });

    // Library That Supports 1 or Many Clips per SfxKind
    let mut lib = SfxLibrary::default();

    // Doors
    lib.insert_one(SfxKind::DoorOpen, asset_server.load("sounds/sfx/door_open.ogg"));
    lib.insert_one(SfxKind::DoorClose, asset_server.load("sounds/sfx/door_close.ogg"));
    lib.insert_one(SfxKind::NoWay, asset_server.load("sounds/sfx/no_way.ogg"));
    lib.insert_one(SfxKind::Pushwall, asset_server.load("sounds/sfx/pushwall.ogg"));

    // Weapon Attack
    lib.insert_one(
    	SfxKind::KnifeSwing,
    	asset_server.load("sounds/sfx/weapons/knife/knife_jab.ogg"),
    );
    lib.insert_one(
    	SfxKind::PistolFire,
    	asset_server.load("sounds/sfx/weapons/pistol/pistol_fire.ogg"),
    );
    lib.insert_one(
    	SfxKind::MachineGunFire,
    	asset_server.load("sounds/sfx/weapons/machinegun/machinegun_fire_0.ogg"),
    );
    lib.insert_one(
    	SfxKind::ChaingunFire,
    	asset_server.load("sounds/sfx/weapons/chaingun/chaingun_fire_0.ogg"),
    );

    // Weapon / Ammo Pickups
    lib.insert_one(
    	SfxKind::PickupChaingun,
    	asset_server.load("sounds/sfx/weapons/chaingun/chaingun_pickup.ogg"),
    );
    lib.insert_one(
    	SfxKind::PickupMachineGun,
    	asset_server.load("sounds/sfx/weapons/machinegun/machinegun_pickup.ogg"),
    );
    lib.insert_one(
    	SfxKind::PickupAmmo,
    	asset_server.load("sounds/sfx/weapons/ammo/ammo_pickup.ogg"),
    );

	// Health Pickups
	lib.insert_one(SfxKind::PickupHealthFirstAid, asset_server.load("sounds/sfx/health/first_aid.ogg"));
	lib.insert_one(SfxKind::PickupHealthDinner, asset_server.load("sounds/sfx/health/dinner.ogg"));
	lib.insert_one(SfxKind::PickupHealthDogFood, asset_server.load("sounds/sfx/health/dog_food.ogg"));
	lib.insert_one(SfxKind::PickupOneUp, asset_server.load("sounds/sfx/health/oneup.ogg"));

    // Treasure
    lib.insert_one(
        SfxKind::PickupTreasureCross,
        asset_server.load("sounds/sfx/treasure/cross.ogg"),
    );
    lib.insert_one(
        SfxKind::PickupTreasureChalice,
        asset_server.load("sounds/sfx/treasure/chalice.ogg"),
    );
    lib.insert_one(
        SfxKind::PickupTreasureChest,
        asset_server.load("sounds/sfx/treasure/chest.ogg"),
    );
    lib.insert_one(
        SfxKind::PickupTreasureCrown,
        asset_server.load("sounds/sfx/treasure/crown.ogg"),
    );

    // Guard Alert
    lib.insert_one(
        SfxKind::EnemyAlert(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_alert.ogg"),
    );

    // Guard Shoot
    lib.insert_one(
        SfxKind::EnemyShoot(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_shoot.ogg"),
    );

    // Guard Death Set (Random Pick in play_sfx_events)
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
    // Prevent Duplicates if Startup Runs Again
    if q_music.iter().next().is_some() {
        return;
    }

    commands.spawn((
        Music,
        AudioPlayer::new(audio.music_level.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.45)),
    ));
}

fn is_pickup_kind(k: SfxKind) -> bool {
    matches!(
        k,
        // Pickups - Weapons
        SfxKind::PickupChaingun
            | SfxKind::PickupMachineGun
            | SfxKind::PickupAmmo

            // Pickups - Health
            | SfxKind::PickupHealthFirstAid
            | SfxKind::PickupHealthDinner
            | SfxKind::PickupHealthDogFood
            | SfxKind::PickupOneUp

            // Pickups - Treasure
            | SfxKind::PickupTreasureCross
            | SfxKind::PickupTreasureChalice
            | SfxKind::PickupTreasureChest
            | SfxKind::PickupTreasureCrown
    )
}

pub fn play_sfx_events(
    lib: Res<SfxLibrary>,
    mut commands: Commands,
    mut ev: MessageReader<PlaySfx>,
    q_active_pickup: Query<Entity, With<ActivePickupSfx>>,
) {
    // Collect Events: Play All Non-Pickups, Only Last Pickup (No Overlap)
    let mut last_pickup: Option<PlaySfx> = None;
    let mut non_pickups: Vec<PlaySfx> = Vec::new();

    for e in ev.read() {
        if is_pickup_kind(e.kind) {
            last_pickup = Some(e.clone());
        } else {
            non_pickups.push(e.clone());
        }
    }

    // Play Non-Pickup SFX Normally (Overlap Permitted)
    for e in non_pickups {
        let Some(list) = lib.map.get(&e.kind) else {
            warn!("Missing SFX for {:?}", e.kind);
            continue;
        };
        if list.is_empty() {
            continue;
        }

        let i = rand::rng().random_range(0..list.len());
        let clip = list[i].clone();

        let settings = match e.kind {
            SfxKind::DoorOpen | SfxKind::DoorClose | SfxKind::NoWay | SfxKind::Pushwall => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.12))
                .with_volume(Volume::Linear(1.0)),

            SfxKind::KnifeSwing
            | SfxKind::PistolFire
            | SfxKind::MachineGunFire
            | SfxKind::ChaingunFire => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.12))
                .with_volume(Volume::Linear(1.3)),

            SfxKind::PickupHealthFirstAid
			| SfxKind::PickupHealthDinner
			| SfxKind::PickupHealthDogFood
			| SfxKind::PickupOneUp => PlaybackSettings::DESPAWN
			    .with_spatial(true)
			    .with_spatial_scale(SpatialScale::new(0.10))
			    .with_volume(Volume::Linear(1.25)),

            SfxKind::PickupTreasureCross
            | SfxKind::PickupTreasureChalice
            | SfxKind::PickupTreasureChest
            | SfxKind::PickupTreasureCrown => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.25)),

            SfxKind::EnemyAlert(_) => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15)),

            SfxKind::EnemyShoot(_) => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.25))
                .with_volume(Volume::Linear(1.3)),

            SfxKind::EnemyDeath(_) => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.3)),

            SfxKind::PickupChaingun | SfxKind::PickupMachineGun | SfxKind::PickupAmmo => {
                unreachable!()
            }
        };

        commands.spawn((
            Transform::from_translation(e.pos),
            AudioPlayer::new(clip),
            settings,
        ));
    }

    // Only Newest Pickup Plays, Cutting Off Any Previous Pickup
    let Some(e) = last_pickup else { return; };

    let Some(list) = lib.map.get(&e.kind) else {
        warn!("Missing SFX for {:?}", e.kind);
        return;
    };
    if list.is_empty() {
        return;
    }

    // Stop ONLY Previous Pickup Sound, Not Weapon Fire / Deaths / Doors
    for ent in q_active_pickup.iter() {
        commands.entity(ent).despawn();
    }

    let i = rand::rng().random_range(0..list.len());
    let clip = list[i].clone();

    let settings = PlaybackSettings::DESPAWN
        .with_spatial(true)
        .with_spatial_scale(SpatialScale::new(0.12))
        .with_volume(Volume::Linear(1.15));

    commands.spawn((
        ActivePickupSfx,
        Transform::from_translation(e.pos),
        AudioPlayer::new(clip),
        settings,
    ));
}
