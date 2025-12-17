use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy::input::mouse::AccumulatedMouseMotion;
use crate::map::{
	DoorState,
	DoorTile,
	MapGrid,
	Tile,
};
use crate::audio::{PlaySfx, SfxKind};

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

// Left Click to Lock/Hide Cursor; Esc to Release
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
    return; // 0 or 2+ matching entities
};
    look.yaw -= delta.x * settings.sensitivity;
    look.pitch -= delta.y * settings.sensitivity;
    look.pitch = look.pitch.clamp(-1.54, 1.54); // ~ +/- 88 degrees

    transform.rotation = Quat::from_euler(EulerRot::YXZ, look.yaw, look.pitch, 0.0);
}

pub fn player_move(
    time: Res<Time<Fixed>>,
    keys: Res<ButtonInput<KeyCode>>,
    grid: Res<MapGrid>,
    mut q_player: Query<&mut Transform, With<Player>>,
    settings: Res<PlayerSettings>,
) {
    const PLAYER_RADIUS: f32 = 0.25; // in tile units (tile = 1.0)

    let Ok(mut transform) = q_player.single_mut() else {
        return;
    };

    // Movement basis (XZ only)
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

    // --- Helpers: world pos (x,z) -> tile index (x,z) ---
    // Tiles are centered on integer coords (wall cubes are centered at x=0,1,2...),
    // so "which tile am I in?" is floor(pos + 0.5).
    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    fn is_solid(grid: &MapGrid, tx: i32, tz: i32) -> bool {
        if tx < 0 || tz < 0 || tx >= grid.width as i32 || tz >= grid.height as i32 {
            return true; // outside map = solid
        }
        matches!(grid.tile(tx as usize, tz as usize), Tile::Wall | Tile::DoorClosed)
    }

    fn collides(grid: &MapGrid, pos_xz: Vec2, radius: f32) -> bool {
        let samples = [
            pos_xz + Vec2::new(-radius, -radius),
            pos_xz + Vec2::new(-radius,  radius),
            pos_xz + Vec2::new( radius, -radius),
            pos_xz + Vec2::new( radius,  radius),
        ];

        for s in samples {
            let t = world_to_tile(s);
            if is_solid(grid, t.x, t.y) {
                return true;
            }
        }
        false
    }

    // Current position in XZ
    let mut pos = Vec2::new(transform.translation.x, transform.translation.z);

    // Slide: resolve X, then Z
    let try_x = Vec2::new(pos.x + step.x, pos.y);
    if !collides(&grid, try_x, PLAYER_RADIUS) {
        pos.x = try_x.x;
    }

    let try_z = Vec2::new(pos.x, pos.y + step.z);
    if !collides(&grid, try_z, PLAYER_RADIUS) {
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

    // Player tile (same convention as collision: floor(pos + 0.5))
    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    let player_tile = world_to_tile(Vec2::new(
        player_tf.translation.x,
        player_tf.translation.z,
    ));

    // Facing direction -> choose the dominant axis (Wolf-style 4-way use)
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

    // Bounds check
    if target.x < 0
        || target.y < 0
        || target.x >= grid.width as i32
        || target.y >= grid.height as i32
    {
        return;
    }

    let (tx, tz) = (target.x as usize, target.y as usize);
    let cur = grid.tile(tx, tz);

    let (new_tile, new_vis, sfx_kind) = match cur {
        Tile::DoorClosed => (Tile::DoorOpen, Visibility::Hidden, SfxKind::DoorOpen),
        Tile::DoorOpen => (Tile::DoorClosed, Visibility::Visible, SfxKind::DoorClose),
        _ => return,
    };

    grid.set_tile(tx, tz, new_tile);

    // Update the door entity (visibility + timer)
	for (door, mut state, mut vis) in q_doors.iter_mut() {
	    if door.0 == target {
	        *vis = new_vis;

	        // Start/reset the auto-close timer
	        state.open_timer = match new_tile {
	            Tile::DoorOpen => DOOR_OPEN_SECS,
	            _ => 0.0,
	        };

	        break;
	    }
	}

    // Play SFX at the door center
    sfx.write(PlaySfx {
        kind: sfx_kind,
        pos: Vec3::new(target.x as f32 * TILE_SIZE, 0.6, target.y as f32 * TILE_SIZE),
    });
}

pub fn door_auto_close(
    time: Res<Time>,
    mut grid: ResMut<MapGrid>,
    q_player: Query<&Transform, With<Player>>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &mut Visibility)>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    const TILE_SIZE: f32 = 1.0;
    const RETRY_SECS_IF_BLOCKED: f32 = 0.2;

    let Ok(player_tf) = q_player.single() else { return; };

    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    let player_tile = world_to_tile(Vec2::new(
        player_tf.translation.x,
        player_tf.translation.z,
    ));

    for (door, mut state, mut vis) in q_doors.iter_mut() {
        if state.open_timer <= 0.0 {
            continue;
        }

        state.open_timer -= time.delta_secs();
        if state.open_timer > 0.0 {
            continue;
        }

        let dt = door.0;

        // If the player is standing in the doorway, don't slam it shut; retry shortly.
        if dt == player_tile {
            state.open_timer = RETRY_SECS_IF_BLOCKED;
            continue;
        }

        let (tx, tz) = (dt.x as usize, dt.y as usize);
        if grid.tile(tx, tz) != Tile::DoorOpen {
            continue;
        }

        // Close it (solid again)
        grid.set_tile(tx, tz, Tile::DoorClosed);
        *vis = Visibility::Visible;

        // Play close SFX at the door center
        sfx.write(PlaySfx {
            kind: SfxKind::DoorClose,
            pos: Vec3::new(dt.x as f32 * TILE_SIZE, 0.6, dt.y as f32 * TILE_SIZE),
        });
    }
}
