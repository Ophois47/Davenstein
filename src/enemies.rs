/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::time::Timer;

use crate::actors::{Dead, Health, OccupiesTile};
use crate::ai::EnemyMove;
use crate::audio::{PlaySfx, SfxKind};
use crate::player::Player;

const GUARD_MAX_HP: i32 = 25;
const SS_MAX_HP: i32 = 100;
const DOG_MAX_HP: i32 = 1;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    Guard,
    Ss,
    Dog,
    // TODO: Officer, Mutant, Boss, etc.
}

#[derive(Clone, Copy, Debug)]
pub enum AttackMode {
    Hitscan,
    Melee,
}

#[derive(Clone, Copy, Debug)]
pub struct EnemyTuning {
    pub max_hp: i32,
    pub wander_speed_tps: f32,
    pub chase_speed_tps: f32,
    pub can_shoot: bool,
    pub attack_damage: i32,
    pub attack_cooldown_secs: f32,
    pub attack_range_tiles: f32,
    pub reaction_time_secs: f32,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct EnemyTunings {
    pub guard: EnemyTuning,
    pub ss: EnemyTuning,
    pub dog: EnemyTuning,
}

impl EnemyTunings {
    /// Single source of truth for defaults without relying on Default/derive(Default)
    pub fn baseline() -> Self {
        Self {
            guard: EnemyTuning {
                max_hp: 25,
                wander_speed_tps: 0.9,
                chase_speed_tps: 1.6,
                can_shoot: true,
                attack_damage: 8,
                attack_cooldown_secs: 0.6,
                attack_range_tiles: 6.0,
                reaction_time_secs: 0.35,
            },
            ss: EnemyTuning {
                max_hp: 100,
                wander_speed_tps: 1.0,
                chase_speed_tps: 1.8,
                can_shoot: true,
                attack_damage: 10,
                attack_cooldown_secs: 0.55,
                attack_range_tiles: 7.0,
                reaction_time_secs: 0.30,
            },
            dog: EnemyTuning {
                max_hp: 1,
                wander_speed_tps: 1.2,
                chase_speed_tps: 2.2,
                can_shoot: false,
                attack_damage: 8,
                attack_cooldown_secs: 0.35,
                attack_range_tiles: 1.1,
                reaction_time_secs: 0.20,
            },
        }
    }

    pub fn for_kind(&self, kind: EnemyKind) -> EnemyTuning {
        match kind {
            EnemyKind::Guard => self.guard,
            EnemyKind::Ss => self.ss,
            EnemyKind::Dog => self.dog,
        }
    }
}

#[derive(Component)]
pub struct Guard;

#[derive(Component)]
pub struct GuardCorpse;

#[derive(Component, Debug, Default)]
pub struct GuardWalk {
    // Progress in "tiles moved"; frame = floor(phase*4) & 3
    pub phase: f32,
}

#[derive(Component)]
pub struct GuardPain {
    pub timer: Timer,
}

#[derive(Component, Debug)]
pub struct GuardShoot {
    pub timer: Timer,
}

#[derive(Resource)]
pub struct GuardSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],

    pub shoot_front_aim: Handle<Image>,
    pub shoot_front_fire: Handle<Image>,
    pub shoot_side_fire: Handle<Image>,

    pub pain: Handle<Image>,
    pub dying: [Handle<Image>; 4],
    pub corpse: Handle<Image>,
}

impl FromWorld for GuardSprites {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        // 8-dir idle frames (your files: guard_idle_a0..a7.png)
        let idle: [Handle<Image>; 8] = std::array::from_fn(|dir| {
            asset_server.load(format!("enemies/guard/guard_idle_a{}.png", dir))
        });

        // 4 walk frames x 8 directions (your files: guard_walk_r{row}_dir{dir}.png)
        let walk: [[Handle<Image>; 8]; 4] = std::array::from_fn(|row| {
            std::array::from_fn(|dir| {
                asset_server.load(format!(
                    "enemies/guard/guard_walk_r{}_dir{}.png",
                    row,
                    dir,
                ))
            })
        });

        // Single-frame states
        let pain: Handle<Image> = asset_server.load("enemies/guard/guard_pain.png");

        // Dying
        let dying: [Handle<Image>; 4] = std::array::from_fn(|i| {
            asset_server.load(format!("enemies/guard/guard_death_{}.png", i))
        });

        let corpse: Handle<Image> = asset_server.load("enemies/guard/guard_corpse.png");

        // Shooting
        let shoot_front_aim: Handle<Image> =
            asset_server.load("enemies/guard/guard_shoot_front_aim.png");
        let shoot_front_fire: Handle<Image> =
            asset_server.load("enemies/guard/guard_shoot_front_fire.png");
        let shoot_side_fire: Handle<Image> =
            asset_server.load("enemies/guard/guard_shoot_side_fire.png");

        Self {
            idle,
            walk,
            shoot_front_aim,
            shoot_front_fire,
            shoot_side_fire,
            pain,
            dying,
            corpse,
        }
    }
}

#[derive(Component)]
pub struct Ss;

#[derive(Component)]
pub struct Dog;

#[derive(Component)]
pub struct SsCorpse;

#[derive(Component)]
pub struct DogCorpse;

#[derive(Component)]
pub struct SsPain {
    pub timer: Timer,
}

#[derive(Component)]
pub struct DogPain {
    pub timer: Timer,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SsDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DogDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

#[derive(Component, Default)]
pub struct SsWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct SsShoot {
    pub t: Timer,
}

#[derive(Component, Default)]
pub struct DogWalk {
    pub phase: f32,
}

#[derive(Component)]
pub struct DogBite {
    pub t: Timer,
}

impl DogBite {
    pub fn new() -> Self {
        Self {
            t: Timer::from_seconds(DOG_BITE_SECS, TimerMode::Once),
        }
    }
}

#[derive(Component, Debug)]
pub struct DogBiteCooldown {
    pub t: Timer,
}

impl DogBiteCooldown {
    pub fn new(secs: f32) -> Self {
        Self {
            t: Timer::from_seconds(secs.max(0.0), TimerMode::Once),
        }
    }
}

const SS_WALK_FPS: f32 = 6.0;
const DOG_WALK_FPS: f32 = 8.0;
pub(crate) const SS_SHOOT_SECS: f32 = 0.35;
const DOG_BITE_SECS: f32 = 0.35;

fn attach_ss_walk(mut commands: Commands, q: Query<Entity, Added<Ss>>) {
    for e in q.iter() {
        commands.entity(e).insert(SsWalk::default());
    }
}

fn attach_dog_walk(mut commands: Commands, q: Query<Entity, Added<Dog>>) {
    for e in q.iter() {
        commands.entity(e).insert(DogWalk::default());
    }
}

fn tick_ss_walk(
    time: Res<Time>,
    mut q: Query<(&mut SsWalk, Option<&EnemyMove>), (With<Ss>, Without<SsDying>)>,
) {
    let dt = time.delta_secs();
    for (mut w, moving) in q.iter_mut() {
        if moving.is_some() {
            w.phase = (w.phase + dt * SS_WALK_FPS) % 1.0;
        } else {
            w.phase = 0.0;
        }
    }
}

fn tick_dog_walk(
    time: Res<Time>,
    mut q: Query<(&mut DogWalk, Option<&EnemyMove>), (With<Dog>, Without<DogDying>)>,
) {
    let dt = time.delta_secs();
    for (mut w, moving) in q.iter_mut() {
        if moving.is_some() {
            w.phase = (w.phase + dt * DOG_WALK_FPS) % 1.0;
        } else {
            w.phase = 0.0;
        }
    }
}

fn tick_ss_shoot(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut SsShoot)>) {
    for (e, mut s) in q.iter_mut() {
        s.t.tick(time.delta());
        if s.t.is_finished() {
            commands.entity(e).remove::<SsShoot>();
        }
    }
}

fn tick_dog_bite(
    time: Res<Time>,
    mut commands: Commands,
    tunings: Res<EnemyTunings>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut enemy_fire: MessageWriter<crate::ai::EnemyFire>,
    mut q: Query<(Entity, &GlobalTransform, &mut DogBite, Option<&DogPain>), With<Dog>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();
    let player_tile = IVec2::new(player_pos.x.round() as i32, player_pos.z.round() as i32);

    // Wolf3D (1992): dog bite hits ~70% of the time and does (US_RndT() >> 4) damage (0..15)
    // NOTE: This can yield 0 damage; if you decide you never want 0 from a *landed* bite
    // we can clamp it to at least 1
    const BITE_HIT_CHANCE: f32 = 0.70;

    for (e, gt, mut bite, pain) in q.iter_mut() {
        // Pain interrupts the bite immediately (no damage, no cooldown)
        if pain.is_some() {
            commands.entity(e).remove::<DogBite>();
            continue;
        }

        bite.t.tick(time.delta());
        if !bite.t.is_finished() {
            continue;
        }

        let dog_pos = gt.translation();
        let dog_tile = IVec2::new(dog_pos.x.round() as i32, dog_pos.z.round() as i32);

        let dx = (player_tile.x - dog_tile.x).abs();
        let dy = (player_tile.y - dog_tile.y).abs();
        let dist_tiles = dx.max(dy) as f32;

        if dist_tiles <= tunings.dog.attack_range_tiles && rand::random::<f32>() < BITE_HIT_CHANCE {
            let dmg = (rand::random::<u8>() >> 4) as i32;
            enemy_fire.write(crate::ai::EnemyFire {
                kind: EnemyKind::Dog,
                damage: dmg,
            });
        }

        let mut ec = commands.entity(e);
        ec.remove::<DogBite>();
        ec.insert(DogBiteCooldown::new(tunings.dog.attack_cooldown_secs));
    }
}

fn tick_dog_bite_cooldown(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut DogBiteCooldown), With<Dog>>,
) {
    for (e, mut cd) in q.iter_mut() {
        cd.t.tick(time.delta());
        if cd.t.is_finished() {
            commands.entity(e).remove::<DogBiteCooldown>();
        }
    }
}

fn tick_ss_pain(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut SsPain)>) {
    for (e, mut p) in q.iter_mut() {
        p.timer.tick(time.delta());
        if p.timer.is_finished() {
            commands.entity(e).remove::<SsPain>();
        }
    }
}

fn tick_dog_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut DogPain, Option<&DogBite>), With<Dog>>,
) {
    for (e, mut p, bite) in q.iter_mut() {
        // Pain Interrupts In-Progress Bite
        if bite.is_some() {
            commands.entity(e).remove::<DogBite>();
        }

        p.timer.tick(time.delta());
        if p.timer.is_finished() {
            commands.entity(e).remove::<DogPain>();
        }
    }
}

fn tick_ss_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut SsDying)>,
) {
    for (e, mut d) in q.iter_mut() {
        // Wolf-style: advance animation by tics, not by a Timer.
        d.tics = d.tics.saturating_add(1);

        // Every N tics, advance one frame.
        // (We'll tune N later; 8 is a sane starting point.)
        if d.tics >= 8 {
            d.tics = 0;
            d.frame = d.frame.saturating_add(1);

            // 4 death frames: 0,1,2,3. After that, become a corpse.
            if d.frame >= 4 {
                commands.entity(e).remove::<SsDying>();
                commands.entity(e).insert(SsCorpse);
            }
        }
    }
}

fn tick_dog_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut DogDying)>,
) {
    for (e, mut d) in q.iter_mut() {
        d.tics = d.tics.saturating_add(1);

        if d.tics >= 8 {
            d.tics = 0;
            d.frame = d.frame.saturating_add(1);

            if d.frame >= 4 {
                commands.entity(e).remove::<DogDying>();
                commands.entity(e).insert(DogCorpse);
            }
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct Dir8(pub u8);

// Cached to Avoid Redundant Texture Swaps
#[derive(Component, Clone, Copy)]
pub struct View8(pub u8);

#[derive(Resource)]
pub struct SsSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub shoot: [Handle<Image>; 3],
    pub pain: [[Handle<Image>; 8]; 2],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

#[derive(Resource)]
pub struct DogSprites {
    pub idle: [Handle<Image>; 8],
    pub walk: [[Handle<Image>; 8]; 4],
    pub bite: [Handle<Image>; 3],
    pub dying: [[Handle<Image>; 8]; 4],
    pub corpse: [Handle<Image>; 8],
}

impl FromWorld for SsSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle = std::array::from_fn(|i| server.load(format!("enemies/ss/ss_idle_a{i}.png")));
        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| server.load(format!("enemies/ss/ss_walk_r{row}_dir{dir}.png")))
        });

        let shoot: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/ss/ss_shoot_{f}.png")));

        let pain0: Handle<Image> = server.load("enemies/ss/ss_death_0.png");
        let pain1: Handle<Image> = server.load("enemies/ss/ss_death_1.png");
        let pain = [
            std::array::from_fn(|_| pain0.clone()),
            std::array::from_fn(|_| pain1.clone()),
        ];
        let dying = std::array::from_fn(|f| {
            let h: Handle<Image> = server.load(format!("enemies/ss/ss_death_{f}.png"));
            std::array::from_fn(|_| h.clone())
        });

        let corpse_one: Handle<Image> = server.load("enemies/ss/ss_corpse.png");
        let corpse = std::array::from_fn(|_| corpse_one.clone());

        Self { idle, walk, shoot, pain, dying, corpse }
    }
}

impl FromWorld for DogSprites {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();

        let idle = std::array::from_fn(|i| server.load(format!("enemies/dog/dog_idle_a{i}.png")));

        // matches: dog_walk_r{row}_dir{dir}.png
        let walk = std::array::from_fn(|row| {
            std::array::from_fn(|dir| {
                server.load(format!("enemies/dog/dog_walk_r{row}_dir{dir}.png"))
            })
        });

        // bite is a 3 frame animation, not 8-dir views
        let bite: [Handle<Image>; 3] =
            std::array::from_fn(|f| server.load(format!("enemies/dog/dog_bite_{f}.png")));

        // files on disk: dog_death_0.png..dog_death_3.png (no per-dir variants) -> duplicate across dirs
        let dying = std::array::from_fn(|f| {
            let h: Handle<Image> = server.load(format!("enemies/dog/dog_death_{f}.png"));
            std::array::from_fn(|_| h.clone())
        });

        // file on disk: dog_corpse.png (no per-dir variants) -> duplicate across dirs
        let corpse_one: Handle<Image> = server.load("enemies/dog/dog_corpse.png");
        let corpse = std::array::from_fn(|_| corpse_one.clone());

        Self { idle, walk, bite, dying, corpse }
    }
}

fn attach_guard_walk(mut commands: Commands, q: Query<Entity, (Added<Guard>, Without<GuardWalk>)>) {
    for e in q.iter() {
        commands.entity(e).insert(GuardWalk::default());
    }
}

fn tick_guard_walk(
    time: Res<Time>,
    mut q: Query<(&mut GuardWalk, Option<&crate::ai::EnemyMove>), (With<Guard>, Without<Dead>, Without<GuardDying>)>,
) {
    let dt = time.delta_secs();
    for (mut walk, mv) in q.iter_mut() {
        if let Some(mv) = mv {
            // 1.0 phase per tile; 4 frames per tile
            walk.phase += dt * mv.speed_tps;
        } else {
            walk.phase = 0.0;
        }
    }
}

pub fn tick_guard_pain(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardPain), With<Guard>>,
    mut started: Local<std::collections::HashMap<Entity, f32>>,
) {
    const PAIN_FLASH_SECS: f32 = 0.08;

    let now = time.elapsed_secs();

    // Track Entities Currently in Pain, Prune Stale Map Entries
    let mut live: Vec<Entity> = Vec::new();

    for (e, mut pain) in q.iter_mut() {
        live.push(e);

        // IMPORTANT: Do NOT Reset on Subsequent Hits
        // Stops Sustained Fire From Freezing Pain Sprite
        let start = started.entry(e).or_insert(now);

        // Tick Timer in Case Anything Relies on it
        // Clamp Visual Pain Duration Based on 'started'
        pain.timer.tick(time.delta());

        if now - *start >= PAIN_FLASH_SECS {
            commands.entity(e).remove::<GuardPain>();
            started.remove(&e);
        }
    }

    // Prevent Local<HashMap> from Growing if Entities Despawn While in Pain
    started.retain(|e, _| live.iter().any(|x| x == e));
}

fn tick_guard_shoot(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardShoot), With<Guard>>,
) {
    for (e, mut shoot) in q.iter_mut() {
        shoot.timer.tick(time.delta());
        if shoot.timer.is_finished() {
            commands.entity(e).remove::<GuardShoot>();
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GuardDying {
    pub frame: u8, // 0..DEATH_FRAMES-1
    pub tics: u8,  // Fixed-Step Counter
}

pub fn play_enemy_death_sfx(
    mut sfx: MessageWriter<PlaySfx>,
    q: Query<(&GlobalTransform, &EnemyKind), Added<Dead>>,
) {
    for (gt, kind) in q.iter() {
        let p = gt.translation();
        let pos = Vec3::new(p.x, 0.6, p.z);

        sfx.write(PlaySfx {
            kind: SfxKind::EnemyDeath(*kind),
            pos,
        });
    }
}

pub fn spawn_guard(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &GuardSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    // A Vertical Quad in XY Plane (Normal +Z), UVs "Upright"
    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,       // No Lighting on Sprites
        cull_mode: None,   // Safe for Billboards
        ..default()
    });

    commands.spawn((
        Guard,
        EnemyKind::Guard,
        Dir8(0),
        View8(0),
        Health::new(GUARD_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_ss(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &SsSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Ss,
        EnemyKind::Ss,
        Dir8(0),
        View8(0),
        Health::new(SS_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn spawn_dog(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    sprites: &DogSprites,
    tile: IVec2,
) {
    const TILE_SIZE: f32 = 1.0;
    const WALL_H: f32 = 1.0;

    let pos = Vec3::new(tile.x as f32 * TILE_SIZE, WALL_H * 0.5, tile.y as f32 * TILE_SIZE);

    let quad = meshes.add(Mesh::from(Rectangle::new(0.85, 1.0)));
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(sprites.idle[0].clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Dog,
        EnemyKind::Dog,
        Dir8(0),
        View8(0),
        Health::new(DOG_MAX_HP),
        OccupiesTile(tile),
        Mesh3d(quad),
        MeshMaterial3d(mat),
        Transform::from_translation(pos),
    ));
}

pub fn update_ss_views(
    sprites: Res<SsSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&SsCorpse>,
            Option<&SsDying>,
            Option<&SsPain>,
            Option<&SsShoot>,
            Option<&SsWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Ss>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (corpse, dying, pain, shoot, walk, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse[v as usize].clone()
        } else if let Some(d) = dying {
            let f = (d.frame as usize).min(3);
            sprites.dying[f][v as usize].clone()
        } else if let Some(p) = pain {
            let dur = p.timer.duration().as_secs_f32().max(1e-6);
            let t = p.timer.elapsed().as_secs_f32();
            let fi = ((t / dur) * 2.0).floor() as usize;
            let fi = fi.min(1);
            sprites.pain[fi][v as usize].clone()
        } else if let Some(s) = shoot {
            let dur = s.t.duration().as_secs_f32().max(1e-6);
            let t = s.t.elapsed().as_secs_f32();
            let fi = ((t / dur) * 3.0).floor() as usize;
            let fi = fi.min(2);
            sprites.shoot[fi].clone()
        } else if mv.is_some() {
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

pub fn update_dog_views(
    sprites: Res<DogSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&DogCorpse>,
            Option<&DogDying>,
            Option<&DogPain>,
            Option<&DogBite>,
            Option<&DogWalk>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Dog>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (corpse, dying, pain, bite, walk, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse[v as usize].clone()
        } else if let Some(d) = dying {
            let f = (d.frame as usize).min(3);
            sprites.dying[f][v as usize].clone()
        } else if pain.is_some() {
            // dog sheet has no dedicated pain frames in your zip; keep them “flinch-less” for now
            sprites.idle[v as usize].clone()
        } else if let Some(b) = bite {
            let dur = b.t.duration().as_secs_f32().max(1e-6);
            let t = b.t.elapsed().as_secs_f32();
            let frac = (t / dur).clamp(0.0, 0.999_9);

            let frame = (frac * 3.0).floor() as usize;
            sprites.bite[frame.min(2)].clone()
        } else if mv.is_some() {
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

fn quantize_view8(enemy_dir8: u8, enemy_pos: Vec3, player_pos: Vec3) -> u8 {
    use std::f32::consts::TAU;

    let to_player = player_pos - enemy_pos;
    let flat = Vec3::new(to_player.x, 0.0, to_player.z);
    if flat.length_squared() < 1e-6 {
        return 0;
    }

    let step = TAU / 8.0;
    let angle_to_player = flat.x.atan2(flat.z).rem_euclid(TAU);
    // Define Dir8(0) as Facing +Z, Dir8(2)=+X, Dir8(4)=-Z, Dir8(6)=-X
    let enemy_yaw = (enemy_dir8 as f32) * step;
    let rel = (angle_to_player - enemy_yaw).rem_euclid(TAU);

    (((rel + step * 0.5) / step).floor() as i32 & 7) as u8
}

pub fn tick_guard_dying(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GuardDying), With<Guard>>,
) {
    const DEATH_FRAMES: u8 = 4;
    const TICS_PER_FRAME: u8 = 6;

    for (e, mut dying) in q.iter_mut() {
        dying.tics = dying.tics.saturating_add(1);

        if dying.tics >= TICS_PER_FRAME {
            dying.tics = 0;
            dying.frame = dying.frame.saturating_add(1);

            if dying.frame >= DEATH_FRAMES {
                // End of Animation -> Permanent Corpse
                commands.entity(e).remove::<GuardDying>();
                commands.entity(e).insert(GuardCorpse);
            }
        }
    }
}

pub fn apply_guard_corpses(
    sprites: Res<GuardSprites>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(
        &MeshMaterial3d<StandardMaterial>,
        &mut Transform,
        Option<&mut Visibility>,
    ), (With<Guard>, Added<GuardCorpse>)>,
) {
    // Push Corpses Slightly "Back" so Item Drops
    // at Same Tile Can Win Depth Ties
    const CORPSE_DEPTH_BIAS: f32 = 250.0;

    for (mat3d, mut tf, vis) in q.iter_mut() {
        if let Some(mat) = materials.get_mut(&mat3d.0) {
            mat.base_color_texture = Some(sprites.corpse.clone());

            // Corpses Should NOT be Blend, or They'll Fight / Cover Drops
            mat.alpha_mode = AlphaMode::Mask(0.5);

            mat.unlit = true;
            mat.cull_mode = None;

            // Make Corpse Slightly Farther in Depth Than Drops
            mat.depth_bias = CORPSE_DEPTH_BIAS;
        }

        if let Some(mut v) = vis {
            *v = Visibility::Visible;
        }

        tf.translation.y = 0.5;
    }
}

pub fn update_guard_views(
    sprites: Res<GuardSprites>,
    q_player: Query<&GlobalTransform, With<Player>>,
    mut q: Query<
        (
            Option<&Dead>,
            Option<&GuardCorpse>,
            Option<&GuardDying>,
            Option<&GuardPain>,
            Option<&GuardWalk>,
            Option<&GuardShoot>,
            Option<&EnemyMove>,
            &GlobalTransform,
            &Dir8,
            &mut View8,
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (With<Guard>, Without<Player>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(player_gt) = q_player.iter().next() else { return; };
    let player_pos = player_gt.translation();

    for (_dead, corpse, dying, pain, walk, shoot, mv, gt, dir8, mut view, mat3d, mut tf) in q.iter_mut() {
        let enemy_pos = gt.translation();

        // Compute View Index (0..7) Relative to Enemy's Facing + Player Position
        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        // Rotate Quad to Face Player
        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        // Choose Texture in Priority Order:
        // Corpse > Dying > Pain > Shooting > Moving (Walk) > Idle
        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse.clone()
        } else if let Some(d) = dying {
            let i = (d.frame as usize).min(sprites.dying.len().saturating_sub(1));
            sprites.dying[i].clone()
        } else if pain.is_some() {
            sprites.pain.clone()
        } else if let Some(s) = shoot {
            let frontish = matches!(v, 0 | 1 | 7);

            // GuardShoot Has Only Timer', Pick Aim vs Fire Based on Timer Progress
            let dur = s.timer.duration().as_secs_f32().max(1e-6);
            let t = s.timer.elapsed().as_secs_f32();
            let fire_phase = t >= (dur * 0.5);

            if frontish {
                if fire_phase {
                    sprites.shoot_front_fire.clone()
                } else {
                    sprites.shoot_front_aim.clone()
                }
            } else {
                sprites.shoot_side_fire.clone()
            }
        } else if mv.is_some() {
            // Walk Frame Index From GuardWalk.phase (4 Frames Per Tile)
            let w = walk.map(|w| w.phase).unwrap_or(0.0);
            let frame_i = (((w * 4.0).floor() as i32) & 3) as usize;
            sprites.walk[frame_i][v as usize].clone()
        } else {
            sprites.idle[v as usize].clone()
        };

        if mat.base_color_texture.as_ref() != Some(&tex) {
            mat.base_color_texture = Some(tex);
        }
    }
}

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuardSprites>()
            .init_resource::<SsSprites>()
            .init_resource::<DogSprites>()
            .add_systems(
                Update,
                (
                    attach_guard_walk,
                    attach_ss_walk,
                    attach_dog_walk,
                    update_guard_views,
                    update_ss_views,
                    update_dog_views,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    tick_guard_walk,
                    tick_guard_pain,
                    tick_guard_shoot,
                    tick_guard_dying,
                    tick_ss_walk,
                    tick_ss_pain,
                    tick_ss_shoot,
                    tick_ss_dying,
                    tick_dog_walk,
                    tick_dog_pain,
                    tick_dog_bite_cooldown,
                    tick_dog_bite,
                    tick_dog_dying,
                )
                    .chain(),
            );
    }
}
