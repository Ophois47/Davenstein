use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy::input::mouse::AccumulatedMouseMotion;

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
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<&mut Transform, With<Player>>,
    settings: Res<PlayerSettings>,
) {
    let Ok(mut transform) = q_player.single_mut() else {
		return;
	};

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

    transform.translation += wish.normalize_or_zero() * settings.speed * time.delta_secs();
}
