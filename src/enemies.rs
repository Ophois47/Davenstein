/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::time::Timer;

use crate::actors::{Dead, Health, OccupiesTile};
use crate::ai::EnemyMove;
use crate::audio::{PlaySfx, SfxKind};
use crate::player::Player;

const GUARD_MAX_HP: i32 = 6;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    Guard,
    // TODO: Officer, SS, Dog, Boss, etc.
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

#[derive(Component, Clone, Copy)]
pub struct Dir8(pub u8);

// Cached to Avoid Redundant Texture Swaps
#[derive(Component, Clone, Copy)]
pub struct View8(pub u8);

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
) {
    for (e, mut pain) in q.iter_mut() {
        pain.timer.tick(time.delta());

        if pain.timer.is_finished() {
            commands.entity(e).remove::<GuardPain>();
        }
    }
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

        // Compute view index (0..7) relative to enemy's facing + player position
        let v = quantize_view8(dir8.0, enemy_pos, player_pos);
        view.0 = v;

        // Billboard: rotate quad to face player (don’t touch translation)
        let to_player = player_pos - enemy_pos;
        let flat_len2 = to_player.x * to_player.x + to_player.z * to_player.z;
        if flat_len2 > 1e-6 {
            let yaw = to_player.x.atan2(to_player.z);
            tf.rotation = Quat::from_rotation_y(yaw);
        }

        let Some(mat) = materials.get_mut(&mat3d.0) else { continue; };

        // Choose texture in priority order:
        // corpse > dying > pain > shooting > moving(walk) > idle
        let tex: Handle<Image> = if corpse.is_some() {
            sprites.corpse.clone()
        } else if let Some(d) = dying {
            let i = (d.frame as usize).min(sprites.dying.len().saturating_sub(1));
            sprites.dying[i].clone()
        } else if pain.is_some() {
            sprites.pain.clone()
        } else if let Some(s) = shoot {
            let frontish = matches!(v, 0 | 1 | 7);

            // GuardShoot has only `timer`, so pick aim vs fire based on timer progress.
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
            // Walk frame index from GuardWalk.phase (4 frames per tile)
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
            .add_systems(Update, (attach_guard_walk, update_guard_views))
            .add_systems(
                FixedUpdate,
                (
                    tick_guard_walk,
                    tick_guard_pain,
                    tick_guard_shoot,
                    tick_guard_dying,
                ),
            );
    }
}
