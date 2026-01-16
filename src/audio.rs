/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::audio::{
	AudioPlayer,
	AudioSource,
    PlaybackMode,
	PlaybackSettings,
    SpatialScale,
    Volume,
};
use std::collections::HashMap;
use rand::Rng;

use crate::enemies::EnemyKind;
use crate::level::{CurrentLevel, LevelId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxKind {
    // Sfx - World
    DoorOpen,
    DoorClose,
    NoWay,
    Pushwall,
    ElevatorSwitch,

    // Sfx - Menu / UI
    MenuMove,
    MenuSelect,
    MenuBack,

    // Sfx - Intermission / Stats
    IntermissionTick,
    IntermissionConfirm,
    IntermissionNoBonus,
    IntermissionPercent100,
    IntermissionBonusApply,
    
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

    // Pickups - Key
    PickupKey,

    // Enemies
    EnemyAlert(EnemyKind),
    EnemyShoot(EnemyKind),
    EnemyDeath(EnemyKind),
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LevelTrack {
    GETTHEM_MUS,
    SEARCHN_MUS,
    POW_MUS,
    SUSPENSE_MUS,
    WARMARCH_MUS,
    CORNER_MUS,

    NAZI_OMI_MUS,
    PREGNANT_MUS,
    GOINGAFT_MUS,
    HEADACHE_MUS,
    DUNGEON_MUS,

    INTROCW3_MUS,
    NAZIRAP_MUS,
    TWELFTH_MUS,
    ZEROHOUR_MUS,
    ULTIMATE_MUS,
    PACMAN_MUS,
    FUNKYOU_MUS,
}

impl LevelTrack {
    fn asset_path(self) -> &'static str {
        match self {
            LevelTrack::GETTHEM_MUS => "sounds/music/GETTHEM_MUS.ogg",
            LevelTrack::SEARCHN_MUS => "sounds/music/SEARCHN_MUS.ogg",
            LevelTrack::POW_MUS => "sounds/music/POW_MUS.ogg",
            LevelTrack::SUSPENSE_MUS => "sounds/music/SUSPENSE_MUS.ogg",
            LevelTrack::WARMARCH_MUS => "sounds/music/WARMARCH_MUS.ogg",
            LevelTrack::CORNER_MUS => "sounds/music/CORNER_MUS.ogg",

            LevelTrack::NAZI_OMI_MUS => "sounds/music/NAZI_OMI_MUS.ogg",
            LevelTrack::PREGNANT_MUS => "sounds/music/PREGNANT_MUS.ogg",
            LevelTrack::GOINGAFT_MUS => "sounds/music/GOINGAFT_MUS.ogg",
            LevelTrack::HEADACHE_MUS => "sounds/music/HEADACHE_MUS.ogg",
            LevelTrack::DUNGEON_MUS => "sounds/music/DUNGEON_MUS.ogg",

            LevelTrack::INTROCW3_MUS => "sounds/music/INTROCW3_MUS.ogg",
            LevelTrack::NAZIRAP_MUS => "sounds/music/NAZIRAP_MUS.ogg",
            LevelTrack::TWELFTH_MUS => "sounds/music/TWELFTH_MUS.ogg",
            LevelTrack::ZEROHOUR_MUS => "sounds/music/ZEROHOUR_MUS.ogg",
            LevelTrack::ULTIMATE_MUS => "sounds/music/ULTIMATE_MUS.ogg",
            LevelTrack::PACMAN_MUS => "sounds/music/PACMAN_MUS.ogg",
            LevelTrack::FUNKYOU_MUS => "sounds/music/FUNKYOU_MUS.ogg",
        }
    }
}

fn track_for_level(level: LevelId) -> LevelTrack {
    match level {
        LevelId::E1M1 | LevelId::E1M5 | LevelId::E4M1 | LevelId::E4M5 => LevelTrack::GETTHEM_MUS,
        LevelId::E1M2 | LevelId::E1M6 | LevelId::E4M2 | LevelId::E4M6 => LevelTrack::SEARCHN_MUS,
        LevelId::E1M3 | LevelId::E1M7 | LevelId::E4M3 | LevelId::E4M7 => LevelTrack::POW_MUS,
        LevelId::E1M4 | LevelId::E1M8 | LevelId::E4M4 | LevelId::E4M8 => LevelTrack::SUSPENSE_MUS,
        LevelId::E1M9 | LevelId::E2M9 | LevelId::E4M9 | LevelId::E5M9 => LevelTrack::WARMARCH_MUS,

        LevelId::E1M10 => LevelTrack::CORNER_MUS,

        LevelId::E2M1 | LevelId::E2M5 | LevelId::E5M1 | LevelId::E5M5 => LevelTrack::NAZI_OMI_MUS,
        LevelId::E2M2 | LevelId::E2M6 | LevelId::E5M2 | LevelId::E5M6 => LevelTrack::PREGNANT_MUS,
        LevelId::E2M3 | LevelId::E2M8 | LevelId::E5M3 | LevelId::E5M8 => LevelTrack::GOINGAFT_MUS,
        LevelId::E2M4 | LevelId::E2M7 | LevelId::E5M4 | LevelId::E5M7 => LevelTrack::HEADACHE_MUS,

        LevelId::E3M1 | LevelId::E3M5 | LevelId::E6M1 | LevelId::E6M5 => LevelTrack::INTROCW3_MUS,
        LevelId::E3M2 | LevelId::E3M6 | LevelId::E6M2 | LevelId::E6M6 => LevelTrack::NAZIRAP_MUS,
        LevelId::E3M3 | LevelId::E3M7 | LevelId::E6M3 | LevelId::E6M7 => LevelTrack::TWELFTH_MUS,
        LevelId::E3M4 | LevelId::E3M8 | LevelId::E6M4 | LevelId::E6M8 => LevelTrack::ZEROHOUR_MUS,
        LevelId::E3M9 | LevelId::E6M9 => LevelTrack::ULTIMATE_MUS,
    }
}

#[derive(Clone, Copy, Debug, Message)]
pub struct PlaySfx {
    pub kind: SfxKind,
    pub pos: Vec3,
}

#[derive(Component)]
pub struct ActiveMenuSfx;

#[derive(Component)]
pub struct ActivePickupSfx;

#[derive(Component)]
pub struct ActiveEnemyVoiceSfx;

#[derive(Component)]
pub struct ActiveIntermissionSfx;

#[derive(Resource, Default)]
pub struct SfxLibrary {
    pub map: HashMap<SfxKind, Vec<Handle<AudioSource>>>,
}

impl SfxLibrary {
    pub fn insert_one(&mut self, k: SfxKind, h: Handle<AudioSource>) {
        self.map.entry(k).or_default().push(h);
    }
}

#[derive(Component)]
pub struct Music;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicModeKind {
    Splash,
    Menu,
    Gameplay,
    LevelEnd,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicMode(pub MusicModeKind);

impl Default for MusicMode {
    fn default() -> Self {
        Self(MusicModeKind::Splash)
    }
}

#[derive(Resource)]
pub struct GameAudio {
    pub door_open: Handle<AudioSource>,
    pub door_close: Handle<AudioSource>,
    pub music_splash: Handle<AudioSource>,
    pub music_main_menu: Handle<AudioSource>,
    pub music_level_end: Handle<AudioSource>,
    pub music_levels: HashMap<LevelTrack, Handle<AudioSource>>,
}

pub fn setup_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut music_levels: HashMap<LevelTrack, Handle<AudioSource>> = HashMap::new();

    for t in [
        LevelTrack::GETTHEM_MUS,
        LevelTrack::SEARCHN_MUS,
        LevelTrack::POW_MUS,
        LevelTrack::SUSPENSE_MUS,
        LevelTrack::WARMARCH_MUS,
        LevelTrack::CORNER_MUS,
        LevelTrack::NAZI_OMI_MUS,
        LevelTrack::PREGNANT_MUS,
        LevelTrack::GOINGAFT_MUS,
        LevelTrack::HEADACHE_MUS,
        LevelTrack::DUNGEON_MUS,
        LevelTrack::INTROCW3_MUS,
        LevelTrack::NAZIRAP_MUS,
        LevelTrack::TWELFTH_MUS,
        LevelTrack::ZEROHOUR_MUS,
        LevelTrack::ULTIMATE_MUS,
        LevelTrack::PACMAN_MUS,
        LevelTrack::FUNKYOU_MUS,
    ] {
        music_levels.insert(t, asset_server.load(t.asset_path()));
    }

    commands.insert_resource(GameAudio {
        door_open: asset_server.load("sounds/sfx/door_open.ogg"),
        door_close: asset_server.load("sounds/sfx/door_close.ogg"),
        music_splash: asset_server.load("sounds/music/splash.ogg"),
        music_main_menu: asset_server.load("sounds/music/main_menu.ogg"),
        music_level_end: asset_server.load("sounds/music/level_end.ogg"),
        music_levels,
    });

    // Default Boot Mode
    commands.insert_resource(MusicMode(MusicModeKind::Splash));

    // Library That Supports 1 or Many Clips per SfxKind
    let mut lib = SfxLibrary::default();

    // Menu / UI
    lib.insert_one(
        SfxKind::MenuMove,
        asset_server.load("sounds/sfx/menu/menu_move.ogg"),
    );
    lib.insert_one(
        SfxKind::MenuSelect,
        asset_server.load("sounds/sfx/menu/menu_select.ogg"),
    );
    lib.insert_one(
        SfxKind::MenuBack,
        asset_server.load("sounds/sfx/menu/menu_back.ogg"),
    );

    // Intermission / Score Tally
    lib.insert_one(
        SfxKind::IntermissionTick,
        asset_server.load("sounds/sfx/stats/tally_tick_b.ogg"),
    );
    lib.insert_one(
        SfxKind::IntermissionConfirm,
        asset_server.load("sounds/sfx/stats/tally_tick_a.ogg"),
    );
    lib.insert_one(
        SfxKind::IntermissionNoBonus,
        asset_server.load("sounds/sfx/stats/no_bonus.ogg"),
    );
    lib.insert_one(
        SfxKind::IntermissionPercent100,
        asset_server.load("sounds/sfx/stats/percent_100.ogg"),
    );
    lib.insert_one(
        SfxKind::IntermissionBonusApply,
        asset_server.load("sounds/sfx/stats/bonus_apply.ogg"),
    );

    // Doors
    lib.insert_one(SfxKind::DoorOpen, asset_server.load("sounds/sfx/door_open.ogg"));
    lib.insert_one(SfxKind::DoorClose, asset_server.load("sounds/sfx/door_close.ogg"));
    lib.insert_one(SfxKind::NoWay, asset_server.load("sounds/sfx/no_way.ogg"));
    lib.insert_one(SfxKind::Pushwall, asset_server.load("sounds/sfx/pushwall.ogg"));
    lib.insert_one(SfxKind::ElevatorSwitch, asset_server.load("sounds/sfx/elevator_switch.ogg"));

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

    // Keys
    lib.insert_one(
        SfxKind::PickupKey,
        asset_server.load("sounds/sfx/pickups/key.ogg"),
    );

    // Guard Alert
    lib.insert_one(
        SfxKind::EnemyAlert(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_alert.ogg"),
    );
    // SS Alert
    lib.insert_one(
        SfxKind::EnemyAlert(EnemyKind::Ss),
        asset_server.load("sounds/sfx/enemies/ss/ss_alert.ogg"),
    );
    // Dog Alert
    lib.insert_one(
        SfxKind::EnemyAlert(EnemyKind::Dog),
        asset_server.load("sounds/sfx/enemies/dog/dog_alert.ogg"),
    );

    // Guard Shoot
    lib.insert_one(
        SfxKind::EnemyShoot(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_shoot.ogg"),
    );
    // SS Shoot
    lib.insert_one(
        SfxKind::EnemyShoot(EnemyKind::Ss),
        asset_server.load("sounds/sfx/enemies/ss/ss_shoot.ogg"),
    );
    // Dog Bite
    lib.insert_one(
        SfxKind::EnemyShoot(EnemyKind::Dog),
        asset_server.load("sounds/sfx/enemies/dog/dog_bite.ogg"),
    );

    // Guard Death Set (Random Pick in play_sfx_events)
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_death_0.ogg"),
    );
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_death_1.ogg"),
    );
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_death_2.ogg"),
    );
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_death_3.ogg"),
    );
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_death_4.ogg"),
    );
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_death_5.ogg"),
    );
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Guard),
        asset_server.load("sounds/sfx/enemies/guard/guard_death_6.ogg"),
    );
    // SS Death
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Ss),
        asset_server.load("sounds/sfx/enemies/ss/ss_death_0.ogg"),
    );
    // Dog Death
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Dog),
        asset_server.load("sounds/sfx/enemies/dog/dog_death_0.ogg"),
    );

    // Bosses
    // Hans Alert
    lib.insert_one(
        SfxKind::EnemyAlert(EnemyKind::Hans),
        asset_server.load("sounds/sfx/enemies/hans/hans_alert.ogg"),
    );

    // Hans Shoot
    lib.insert_one(
        SfxKind::EnemyShoot(EnemyKind::Hans),
        asset_server.load("sounds/sfx/enemies/hans/hans_shoot.ogg"),
    );

    // Hans Death
    lib.insert_one(
        SfxKind::EnemyDeath(EnemyKind::Hans),
        asset_server.load("sounds/sfx/enemies/hans/hans_death.ogg"),
    );

    commands.insert_resource(lib);
}

pub fn start_music(
    mut commands: Commands,
    audio: Res<GameAudio>,
    mode: Res<MusicMode>,
    q_music: Query<(), With<Music>>,
) {
    if q_music.iter().next().is_some() {
        return;
    }

    let clip = match mode.0 {
        MusicModeKind::Splash => audio.music_splash.clone(),
        MusicModeKind::Menu => audio.music_main_menu.clone(),
        MusicModeKind::Gameplay => audio
            .music_levels
            .get(&LevelTrack::GETTHEM_MUS)
            .cloned()
            .unwrap_or_default(),
        MusicModeKind::LevelEnd => audio.music_level_end.clone(),
    };

    commands.spawn((
        Music,
        AudioPlayer::new(clip),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(0.45)),
    ));
}

pub fn sync_boot_music(
    mut commands: Commands,
    audio: Res<GameAudio>,
    mode: Res<MusicMode>,
    q_music: Query<Entity, With<Music>>,
    mut last: Local<Option<MusicModeKind>>,
) {
    if mode.0 == MusicModeKind::Gameplay {
        *last = Some(MusicModeKind::Gameplay);
        return;
    }

    if *last == Some(mode.0) {
        return;
    }

    for e in q_music.iter() {
        commands.entity(e).try_despawn();
    }

    let clip = match mode.0 {
        MusicModeKind::Splash => audio.music_splash.clone(),
        MusicModeKind::Menu => audio.music_main_menu.clone(),
        MusicModeKind::LevelEnd => audio.music_level_end.clone(),
        MusicModeKind::Gameplay => unreachable!(),
    };

    commands.spawn((
        Music,
        AudioPlayer::new(clip),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(1.4)),
    ));

    *last = Some(mode.0);
}

pub fn sync_level_music(
    mut commands: Commands,
    audio: Res<GameAudio>,
    mode: Res<MusicMode>,
    level: Res<CurrentLevel>,
    music: Query<Entity, With<Music>>,
    mut last: Local<Option<LevelId>>,
) {
    // If we're not in gameplay, clear the cached gameplay-level marker
    // Otherwise returning to gameplay on the same level won't restart the level music
    if mode.0 != MusicModeKind::Gameplay {
        *last = None;
        return;
    }

    // If we're already on this level and we have music, do nothing
    if *last == Some(level.0) && !music.is_empty() {
        return;
    }

    // Always remove whatever "Music" is currently playing (menu/levelend/etc)
    for e in music.iter() {
        commands.entity(e).try_despawn();
    }

    let track = track_for_level(level.0);

    if let Some(handle) = audio.music_levels.get(&track).cloned() {
        commands.spawn((
            Music,
            AudioPlayer::new(handle),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                ..default()
            },
        ));
    }

    *last = Some(level.0);
}

fn is_pickup_kind(k: SfxKind) -> bool {
    matches!(
        k,
        // Pickups - Weapons
        SfxKind::PickupChaingun
            | SfxKind::PickupMachineGun
            | SfxKind::PickupAmmo

            // Pickups - Key
            | SfxKind::PickupKey

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
    q_active_enemy_voice: Query<Entity, With<ActiveEnemyVoiceSfx>>,
    q_active_intermission: Query<Entity, With<ActiveIntermissionSfx>>,
    q_active_menu: Query<Entity, With<ActiveMenuSfx>>,
) {
    // Collect Events: Play All Non-Pickups, Only Last Pickup (No Overlap)
    let mut last_pickup: Option<PlaySfx> = None;
    let mut non_pickups: Vec<PlaySfx> = Vec::new();

    for e in ev.read() {
        if is_pickup_kind(e.kind) {
            last_pickup = Some(*e);
        } else {
            non_pickups.push(*e);
        }
    }

    for e in non_pickups {
        let is_intermission = matches!(
            e.kind,
            SfxKind::IntermissionTick
                | SfxKind::IntermissionConfirm
                | SfxKind::IntermissionNoBonus
                | SfxKind::IntermissionPercent100
                | SfxKind::IntermissionBonusApply
        );

        let is_menu = matches!(e.kind, SfxKind::MenuMove | SfxKind::MenuSelect | SfxKind::MenuBack);

        // Menu UI: single channel, hard-cut previous
        if is_menu {
            let Some(list) = lib.map.get(&e.kind) else {
                warn!("Missing SFX for {:?}", e.kind);
                continue;
            };
            if list.is_empty() {
                continue;
            }

            for ent in q_active_menu.iter() {
                commands.entity(ent).despawn();
            }

            let i = rand::rng().random_range(0..list.len());
            let clip = list[i].clone();

            let settings = PlaybackSettings::DESPAWN
                .with_spatial(false)
                .with_volume(Volume::Linear(1.0));

            commands.spawn((
                ActiveMenuSfx,
                Transform::from_translation(e.pos),
                AudioPlayer::new(clip),
                settings,
            ));

            continue;
        }

        // Intermission: single channel, hard-cut previous
        if is_intermission {
            let Some(list) = lib.map.get(&e.kind) else {
                warn!("Missing SFX for {:?}", e.kind);
                continue;
            };
            if list.is_empty() {
                continue;
            }

            for ent in q_active_intermission.iter() {
                commands.entity(ent).despawn();
            }

            let i = rand::rng().random_range(0..list.len());
            let clip = list[i].clone();

            let settings = PlaybackSettings::DESPAWN
                .with_spatial(false)
                .with_volume(Volume::Linear(1.0));

            commands.spawn((
                ActiveIntermissionSfx,
                Transform::from_translation(e.pos),
                AudioPlayer::new(clip),
                settings,
            ));

            continue;
        }

        // Normal non-pickups path
        let Some(list) = lib.map.get(&e.kind) else {
            warn!("Missing SFX for {:?}", e.kind);
            continue;
        };
        if list.is_empty() {
            continue;
        }

        let i = rand::rng().random_range(0..list.len());
        let clip = list[i].clone();

        let is_enemy_voice = matches!(e.kind, SfxKind::EnemyAlert(_) | SfxKind::EnemyDeath(_));

        if is_enemy_voice {
            for ent in q_active_enemy_voice.iter() {
                commands.entity(ent).despawn();
            }
        }

        let settings = match e.kind {
            SfxKind::DoorOpen
            | SfxKind::DoorClose
            | SfxKind::NoWay
            | SfxKind::Pushwall
            | SfxKind::ElevatorSwitch => PlaybackSettings::DESPAWN
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
                .with_volume(Volume::Linear(1.5)),

            SfxKind::EnemyAlert(_) => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15)),

            SfxKind::EnemyShoot(_) => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.25))
                .with_volume(Volume::Linear(1.4)),

            SfxKind::EnemyDeath(_) => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.15))
                .with_volume(Volume::Linear(1.3)),

            SfxKind::PickupChaingun
            | SfxKind::PickupMachineGun
            | SfxKind::PickupAmmo
            | SfxKind::PickupKey => PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(0.12))
                .with_volume(Volume::Linear(1.15)),

            SfxKind::MenuMove
            | SfxKind::MenuSelect
            | SfxKind::MenuBack => PlaybackSettings::DESPAWN.with_spatial(false),

            SfxKind::IntermissionTick
            | SfxKind::IntermissionConfirm
            | SfxKind::IntermissionNoBonus
            | SfxKind::IntermissionPercent100
            | SfxKind::IntermissionBonusApply => PlaybackSettings::DESPAWN.with_spatial(false),
        };

        if is_enemy_voice {
            commands.spawn((
                ActiveEnemyVoiceSfx,
                Transform::from_translation(e.pos),
                AudioPlayer::new(clip),
                settings,
            ));
        } else {
            commands.spawn((
                Transform::from_translation(e.pos),
                AudioPlayer::new(clip),
                settings,
            ));
        }
    }

    // Pickups: ONLY last pickup
    let Some(e) = last_pickup else { return; };

    let Some(list) = lib.map.get(&e.kind) else {
        warn!("Missing SFX for {:?}", e.kind);
        return;
    };
    if list.is_empty() {
        return;
    }

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
