/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy::input::mouse::AccumulatedMouseMotion;

use crate::actors::{Dead, OccupiesTile};
use crate::ai::EnemyFire;
use crate::audio::{PlaySfx, SfxKind};
use crate::map::{
	DoorAnim,
	DoorState,
	DoorTile,
	MapGrid,
	Tile,
};

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct LookAngles {
	yaw: f32,
	pitch: f32,
}

#[derive(Resource)]
pub struct PlayerSettings {
	speed: f32,
	sensitivity: f32,
}

impl Default for PlayerSettings {
	fn default() -> Self {
		Self {
			speed: 3.5,
			sensitivity: 0.002,
		}
	}
}

#[derive(Component, Debug, Clone, Copy)]
pub struct PlayerVitals {
    pub hp: i32,
    pub hp_max: i32,
}

impl Default for PlayerVitals {
    fn default() -> Self {
        Self { hp: 100, hp_max: 100 }
    }
}

// Left Click to Lock/Hide Cursor, Esc to Release
pub fn grab_mouse(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
    if key.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}

pub fn mouse_look(
    cursor_options: Single<&CursorOptions>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut q: Query<(&mut Transform, &mut LookAngles), With<Player>>,
    settings: Res<PlayerSettings>,
) {
    if cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }
    let delta = mouse_motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    let Ok((mut transform, mut look)) = q.single_mut() else {
    return;
};
    look.yaw -= delta.x * settings.sensitivity;
    look.pitch -= delta.y * settings.sensitivity;
    // ~ +/- 88 Degrees
    look.pitch = look.pitch.clamp(-1.54, 1.54);

    transform.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
}

pub fn player_move(
    time: Res<Time<Fixed>>,
    keys: Res<ButtonInput<KeyCode>>,
    grid: Res<MapGrid>,
    q_enemies: Query<&OccupiesTile, Without<Dead>>,
    mut q_player: Query<&mut Transform, With<Player>>,
    settings: Res<PlayerSettings>,
) {
    // Tile Units (Tile = 1.0)
    const PLAYER_RADIUS: f32 = 0.25;

    let Ok(mut transform) = q_player.single_mut() else {
        return;
    };

    // Snapshot Occupied Tiles (No Allocations Beyond Vec)
    let occupied: Vec<IVec2> = q_enemies.iter().map(|t| t.0).collect();

    // Movement Basis (XZ Only)
    let mut forward = transform.rotation * Vec3::NEG_Z;
    forward.y = 0.0;
    forward = forward.normalize_or_zero();

    let mut right = transform.rotation * Vec3::X;
    right.y = 0.0;
    right = right.normalize_or_zero();

    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { wish += forward; }
    if keys.pressed(KeyCode::KeyS) { wish -= forward; }
    if keys.pressed(KeyCode::KeyD) { wish += right; }
    if keys.pressed(KeyCode::KeyA) { wish -= right; }

    let wish = wish.normalize_or_zero();
    if wish == Vec3::ZERO {
        return;
    }

    let step = wish * settings.speed * time.delta_secs();

    // World POS (X,Z) -> Tile Index (X,Z)
    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    fn is_occupied(occupied: &[IVec2], tx: i32, tz: i32) -> bool {
        occupied.iter().any(|t| t.x == tx && t.y == tz)
    }

    fn is_solid(grid: &MapGrid, occupied: &[IVec2], tx: i32, tz: i32) -> bool {
        if tx < 0 || tz < 0 || tx >= grid.width as i32 || tz >= grid.height as i32 {
            // Outside Map = Solid
            return true;
        }

        // Living Enemies == Solid, Corpses Excluded by Query Later
        if is_occupied(occupied, tx, tz) {
            return true;
        }

        matches!(grid.tile(tx as usize, tz as usize), Tile::Wall | Tile::DoorClosed)
    }

    fn collides(grid: &MapGrid, occupied: &[IVec2], pos_xz: Vec2, radius: f32) -> bool {
        let samples = [
            pos_xz + Vec2::new(-radius, -radius),
            pos_xz + Vec2::new(-radius,  radius),
            pos_xz + Vec2::new( radius, -radius),
            pos_xz + Vec2::new( radius,  radius),
        ];

        for s in samples {
            let t = world_to_tile(s);
            if is_solid(grid, occupied, t.x, t.y) {
                return true;
            }
        }
        false
    }

    // Current Position in XZ
    let mut pos = Vec2::new(transform.translation.x, transform.translation.z);

    // Slide: Resolve X, then Z
    let try_x = Vec2::new(pos.x + step.x, pos.y);
    if !collides(&grid, &occupied, try_x, PLAYER_RADIUS) {
        pos.x = try_x.x;
    }

    let try_z = Vec2::new(pos.x, pos.y + step.z);
    if !collides(&grid, &occupied, try_z, PLAYER_RADIUS) {
        pos.y = try_z.y;
    }

    transform.translation.x = pos.x;
    transform.translation.z = pos.y;
}

pub fn use_doors(
    keys: Res<ButtonInput<KeyCode>>,
    mut grid: ResMut<MapGrid>,
    q_player: Query<&Transform, With<Player>>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &mut Visibility)>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    const TILE_SIZE: f32 = 1.0;
    const DOOR_OPEN_SECS: f32 = 4.5;

    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok(player_tf) = q_player.single() else {
        return;
    };

    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    let player_tile = world_to_tile(Vec2::new(player_tf.translation.x, player_tf.translation.z));

    // 4-Way Use
    let mut fwd = player_tf.rotation * Vec3::NEG_Z;
    fwd.y = 0.0;
    if fwd.length_squared() < 1e-6 {
        return;
    }
    let fwd = fwd.normalize();

    let (dx, dz) = if fwd.x.abs() > fwd.z.abs() {
        (fwd.x.signum() as i32, 0)
    } else {
        (0, fwd.z.signum() as i32)
    };

    let target = IVec2::new(player_tile.x + dx, player_tile.y + dz);

    if target.x < 0
        || target.y < 0
        || target.x >= grid.width as i32
        || target.y >= grid.height as i32
    {
        return;
    }

    let (tx, tz) = (target.x as usize, target.y as usize);
    let cur = grid.tile(tx, tz);

    if !matches!(cur, Tile::DoorClosed | Tile::DoorOpen) {
        return;
    }

    let mut sfx_kind: Option<SfxKind> = None;

    for (door, mut state, mut vis) in q_doors.iter_mut() {
        if door.0 != target {
            continue;
        }

        *vis = Visibility::Visible;

        match cur {
            Tile::DoorOpen => {
                state.want_open = false;
                state.open_timer = 0.0;
                grid.set_tile(tx, tz, Tile::DoorClosed);
                sfx_kind = Some(SfxKind::DoorClose);
            }
            Tile::DoorClosed => {
                state.want_open = true;
                state.open_timer = DOOR_OPEN_SECS;
                sfx_kind = Some(SfxKind::DoorOpen);
            }
            _ => {}
        }

        break;
    }

    if let Some(kind) = sfx_kind {
        sfx.write(PlaySfx {
            kind,
            pos: Vec3::new(target.x as f32 * TILE_SIZE, 0.6, target.y as f32 * TILE_SIZE),
        });
    }
}

pub fn door_animate(
    time: Res<Time<Fixed>>,
    mut grid: ResMut<MapGrid>,
    mut q_doors: Query<(&DoorTile, &DoorState, &mut DoorAnim, &mut Transform, &mut Visibility)>,
) {
    const TILE_SIZE: f32 = 1.0;
    const SLIDE_SPEED: f32 = 2.0;

    for (door, state, mut anim, mut tf, mut vis) in q_doors.iter_mut() {
        let tx = door.0.x;
        let tz = door.0.y;

        if tx < 0 || tz < 0 || tx >= grid.width as i32 || tz >= grid.height as i32 {
            continue;
        }

        let (ux, uz) = (tx as usize, tz as usize);

        let want_open = state.want_open;
        let target = if want_open { 1.0 } else { 0.0 };

        // If Closing, Ensure Grid is Solid Immediately
        if !want_open && grid.tile(ux, uz) == Tile::DoorOpen {
            grid.set_tile(ux, uz, Tile::DoorClosed);
        }

        let step = SLIDE_SPEED * time.delta_secs();
        if anim.progress < target {
            anim.progress = (anim.progress + step).min(1.0);
        } else if anim.progress > target {
            anim.progress = (anim.progress - step).max(0.0);
        }

        tf.translation = anim.closed_pos + anim.slide_axis * (anim.progress * TILE_SIZE);

        // Only When Fully Open Tile Becomes Passable / Able to be Shot Through
        if want_open && anim.progress >= 0.999 {
            if grid.tile(ux, uz) != Tile::DoorOpen {
                grid.set_tile(ux, uz, Tile::DoorOpen);
            }
            *vis = Visibility::Hidden;
        } else {
            *vis = Visibility::Visible;
        }
    }
}

pub fn door_auto_close(
    time: Res<Time<Fixed>>,
    mut grid: ResMut<MapGrid>,
    q_player: Query<&Transform, With<Player>>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &DoorAnim, &mut Visibility)>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    const TILE_SIZE: f32 = 1.0;
    const RETRY_SECS_IF_BLOCKED: f32 = 0.2;
    const FULLY_OPEN_EPS: f32 = 0.999;

    // Must match player_move
    const PLAYER_RADIUS: f32 = 0.25;
    const BLOCK_PAD: f32 = 0.02;

    let Ok(player_tf) = q_player.single() else { return; };

    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    fn circle_overlaps_tile(circle: Vec2, r: f32, tile: IVec2) -> bool {
        let cx = tile.x as f32;
        let cz = tile.y as f32;

        // Tile square bounds (tile centers at integer coords; edges at +/- 0.5)
        let min = Vec2::new(cx - 0.5, cz - 0.5);
        let max = Vec2::new(cx + 0.5, cz + 0.5);

        // Closest point on AABB to circle center
        let closest = Vec2::new(circle.x.clamp(min.x, max.x), circle.y.clamp(min.y, max.y));
        (circle - closest).length_squared() <= r * r
    }

    let player_xz = Vec2::new(player_tf.translation.x, player_tf.translation.z);
    let _player_tile = world_to_tile(player_xz);

    for (door, mut state, anim, mut vis) in q_doors.iter_mut() {
        let dt = door.0;
        if dt.x < 0 || dt.y < 0 || dt.x >= grid.width as i32 || dt.y >= grid.height as i32 {
            continue;
        }
        let (tx, tz) = (dt.x as usize, dt.y as usize);

        // Only once door is actually passable
        if grid.tile(tx, tz) != Tile::DoorOpen {
            continue;
        }

        if anim.progress < FULLY_OPEN_EPS {
            continue;
        }

        state.open_timer -= time.delta_secs();
        if state.open_timer > 0.0 {
            continue;
        }

        // Block closing if player is still overlapping the doorway in world space
        if circle_overlaps_tile(player_xz, PLAYER_RADIUS + BLOCK_PAD, dt) {
            state.open_timer = RETRY_SECS_IF_BLOCKED;
            continue;
        }

        state.want_open = false;
        grid.set_tile(tx, tz, Tile::DoorClosed);
        *vis = Visibility::Visible;

        sfx.write(PlaySfx {
            kind: SfxKind::DoorClose,
            pos: Vec3::new(dt.x as f32 * TILE_SIZE, 0.6, dt.y as f32 * TILE_SIZE),
        });
    }
}

pub fn apply_enemy_fire_to_player(
    mut q_player: Query<&mut PlayerVitals, With<crate::player::Player>>,
    mut ev: MessageReader<EnemyFire>,
) {
    let Some(mut vitals) = q_player.iter_mut().next() else { return; };

    for fire in ev.read() {
        if fire.damage <= 0 {
            continue;
        }

        vitals.hp = (vitals.hp - fire.damage).clamp(0, vitals.hp_max);
    }
}
