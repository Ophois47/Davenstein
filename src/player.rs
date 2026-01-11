/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy::input::mouse::AccumulatedMouseMotion;

use crate::actors::{Dead, OccupiesTile};
use crate::ai::EnemyFire;
use crate::audio::{PlaySfx, SfxKind};
use crate::enemies::EnemyKind;
use crate::map::{
	DoorAnim,
	DoorState,
	DoorTile,
	MapGrid,
	Tile,
};

#[derive(Component)]
pub struct Player;

#[derive(Component, Default, Clone, Copy)]
pub struct PlayerKeys {
    pub gold: bool,
    pub silver: bool,
}

#[derive(Component, Default)]
pub struct LookAngles {
	yaw: f32,
	pitch: f32,
}

impl LookAngles {
    pub fn new(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch }
    }
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

/// When True, Player Input (Move / Look / Use) Ignored
/// For Player Death
#[derive(Resource, Default)]
pub struct PlayerControlLock(pub bool);

/// Prevents Decrementing Lives Every Frame While hp == 0
/// false = Alive (or not yet processed), true = Death Handled
#[derive(Resource, Default)]
pub struct PlayerDeathLatch(pub bool);

#[derive(Resource, Debug, Clone, Copy)]
pub struct GodMode(pub bool);

impl Default for GodMode {
    fn default() -> Self {
        Self(false)
    }
}

pub fn toggle_god_mode(keys: Res<ButtonInput<KeyCode>>, mut god: ResMut<GodMode>) {
    if keys.just_pressed(KeyCode::F9) {
        god.0 = !god.0;
        info!("God Mode: {}", if god.0 { "ON" } else { "OFF" });
    }
}

// Left Click to Lock/Hide Cursor, Esc to Release
pub fn grab_mouse(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    lock: Res<PlayerControlLock>,
) {
    if lock.0 {
        return;
    }

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
    lock: Res<PlayerControlLock>,
) {
    if lock.0 {
        return;
    }

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
    lock: Res<PlayerControlLock>,
    grid: Res<MapGrid>,
    solid: Res<crate::decorations::SolidStatics>,
    q_enemies: Query<&OccupiesTile, Without<Dead>>,
    mut q_player: Query<&mut Transform, With<Player>>,
    settings: Res<PlayerSettings>,
    push_occ: Res<crate::pushwalls::PushwallOcc>,
) {
    if lock.0 {
        return;
    }

    // Tile Units (Tile = 1.0)
    const PLAYER_RADIUS: f32 = 0.20;
    const RUN_MULTIPLIER: f32 = 1.6;

    let Ok(mut transform) = q_player.single_mut() else {
        return;
    };

    // Snapshot Occupied Tiles (Enemies / Actors)
    let occupied: Vec<IVec2> = q_enemies.iter().map(|t| t.0).collect();

    // Movement Basis (XZ Only)
    let mut forward = transform.rotation * Vec3::NEG_Z;
    forward.y = 0.0;
    forward = forward.normalize_or_zero();

    let mut right = transform.rotation * Vec3::X;
    right.y = 0.0;
    right = right.normalize_or_zero();

    let mut wish = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        wish += forward;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        wish -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        wish += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        wish -= right;
    }

    let wish = wish.normalize_or_zero();
    if wish == Vec3::ZERO {
        return;
    }

    let running = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let speed = if running {
        settings.speed * RUN_MULTIPLIER
    } else {
        settings.speed
    };

    let step = wish * speed * time.delta_secs();

    // Collision Helpers
    // IMPORTANT: Tiles Centered on Integer Coords
    // Convert World->Tile With +0.5
    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    fn is_occupied(occupied: &[IVec2], tx: i32, tz: i32) -> bool {
        occupied.iter().any(|t| t.x == tx && t.y == tz)
    }

    fn is_solid(
        grid: &MapGrid,
        solid: &crate::decorations::SolidStatics,
        push: &crate::pushwalls::PushwallOcc,
        occupied: &[IVec2],
        tx: i32,
        tz: i32,
    ) -> bool {
        if tx < 0 || tz < 0 {
            return true;
        }
        let txu = tx as usize;
        let tzu = tz as usize;
        if txu >= grid.width || tzu >= grid.height {
            return true;
        }

        // Moving Pushwalls Behave Like Solid Walls
        if push.blocks_tile(tx, tz) {
            return true;
        }

        match grid.tile(txu, tzu) {
            Tile::Wall | Tile::DoorClosed => true,
            _ => {
                // Blocking Statics (Decorations)
                if solid.is_solid(tx, tz) {
                    return true;
                }
                // Living Enemies / Actors Block
                is_occupied(occupied, tx, tz)
            }
        }
    }

    // Keep Original Fast / Simple Approach
    // Sample 4 Corners of Player's Collision Circle
    fn collides(
        grid: &MapGrid,
        solid: &crate::decorations::SolidStatics,
        push: &crate::pushwalls::PushwallOcc,
        occupied: &[IVec2],
        pos_xz: Vec2,
        radius: f32,
    ) -> bool {
        let samples = [
            pos_xz + Vec2::new(-radius, -radius),
            pos_xz + Vec2::new(-radius,  radius),
            pos_xz + Vec2::new( radius, -radius),
            pos_xz + Vec2::new( radius,  radius),
        ];

        for s in samples {
            let t = world_to_tile(s);
            if is_solid(grid, solid, push, occupied, t.x, t.y) {
                return true;
            }
        }
        false
    }

    // Apply Movement With Sliding (X then Z)
    let mut pos = Vec2::new(transform.translation.x, transform.translation.z);

    let try_x = Vec2::new(pos.x + step.x, pos.y);
    if !collides(&grid, &solid, &push_occ, &occupied, try_x, PLAYER_RADIUS) {
        pos.x = try_x.x;
    }

    let try_z = Vec2::new(pos.x, pos.y + step.z);
    if !collides(&grid, &solid, &push_occ, &occupied, try_z, PLAYER_RADIUS) {
        pos.y = try_z.y;
    }

    transform.translation.x = pos.x;
    transform.translation.z = pos.y;
}

pub fn use_doors(
    keys: Res<ButtonInput<KeyCode>>,
    lock: Res<PlayerControlLock>,
    mut grid: ResMut<MapGrid>,
    q_player: Query<&Transform, With<Player>>,
    q_keys: Query<&PlayerKeys, With<Player>>,
    q_occupied: Query<&OccupiesTile>,
    q_dead_enemies: Query<&GlobalTransform, (With<EnemyKind>, With<Dead>)>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &mut Visibility)>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    if lock.0 {
        return;
    }

    const TILE_SIZE: f32 = 1.0;
    const DOOR_OPEN_SECS: f32 = 4.5;
    const RETRY_SECS_IF_BLOCKED: f32 = 0.2;
    const CORPSE_RADIUS: f32 = 0.35;
    const CORPSE_PAD: f32 = 0.02;

    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok(player_tf) = q_player.single() else {
        return;
    };

    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    fn circle_overlaps_tile(circle: Vec2, r: f32, tile: IVec2) -> bool {
        let cx = tile.x as f32;
        let cz = tile.y as f32;

        let min = Vec2::new(cx - 0.5, cz - 0.5);
        let max = Vec2::new(cx + 0.5, cz + 0.5);

        let closest = Vec2::new(circle.x.clamp(min.x, max.x), circle.y.clamp(min.y, max.y));
        (circle - closest).length_squared() <= r * r
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

    // Locked Doors Encoded in plane0 Codes,
    // But Share Same Tile State
    let plane0 = grid.plane0_code(tx, tz);
    let needs_gold = matches!(plane0, 92 | 93);
    let needs_silver = matches!(plane0, 94 | 95);
    let locked = needs_gold || needs_silver;

    let pk = q_keys.iter().next().copied().unwrap_or_default();
    let has_gold = pk.gold;
    let has_silver = pk.silver;

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
                // Don't Allow Doors to Close on Living Enemies or Dead Bodies
                let dead_blocks = q_dead_enemies.iter().any(|gt| {
                    let p = gt.translation();
                    let xz = Vec2::new(p.x, p.z);
                    circle_overlaps_tile(xz, CORPSE_RADIUS + CORPSE_PAD, target)
                });

                if q_occupied.iter().any(|o| o.0 == target) || dead_blocks {
                    state.want_open = true;
                    state.open_timer = RETRY_SECS_IF_BLOCKED;
                    sfx_kind = Some(SfxKind::NoWay);
                } else {
                    state.want_open = false;
                    state.open_timer = 0.0;
                    grid.set_tile(tx, tz, Tile::DoorClosed);
                    sfx_kind = Some(SfxKind::DoorClose);
                }
            }
            Tile::DoorClosed => {
                if locked && ((needs_gold && !has_gold) || (needs_silver && !has_silver)) {
                    sfx_kind = Some(SfxKind::NoWay);
                    break;
                }

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
    q_occupied: Query<&crate::actors::OccupiesTile>,
    q_dead_enemies: Query<&GlobalTransform, (With<EnemyKind>, With<Dead>)>,
    mut q_doors: Query<(&DoorTile, &mut DoorState, &DoorAnim, &mut Visibility)>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    const TILE_SIZE: f32 = 1.0;
    const RETRY_SECS_IF_BLOCKED: f32 = 0.2;
    const FULLY_OPEN_EPS: f32 = 0.999;

    const CORPSE_RADIUS: f32 = 0.35;
    const CORPSE_PAD: f32 = 0.02;

    // Must Match player_move()
    const PLAYER_RADIUS: f32 = 0.20;
    const BLOCK_PAD: f32 = 0.02;

    let Ok(player_tf) = q_player.single() else { return; };

    fn world_to_tile(p: Vec2) -> IVec2 {
        IVec2::new((p.x + 0.5).floor() as i32, (p.y + 0.5).floor() as i32)
    }

    fn circle_overlaps_tile(circle: Vec2, r: f32, tile: IVec2) -> bool {
        let cx = tile.x as f32;
        let cz = tile.y as f32;

        // Tile Square Bounds
        // (Tile Centers at Integer Coords, Edges at +/- 0.5)
        let min = Vec2::new(cx - 0.5, cz - 0.5);
        let max = Vec2::new(cx + 0.5, cz + 0.5);

        // Closest Point on AABB to Circle Center
        let closest = Vec2::new(circle.x.clamp(min.x, max.x), circle.y.clamp(min.y, max.y));
        (circle - closest).length_squared() <= r * r
    }

    let player_xz = Vec2::new(player_tf.translation.x, player_tf.translation.z);
    let _player_tile = world_to_tile(player_xz);

    // Snapshot Occupied Tiles
    let occupied_tiles: Vec<IVec2> = q_occupied.iter().map(|o| o.0).collect();

    let dead_xz: Vec<Vec2> = q_dead_enemies
        .iter()
        .map(|gt| {
            let p = gt.translation();
            Vec2::new(p.x, p.z)
        })
        .collect();

    for (door, mut state, anim, mut vis) in q_doors.iter_mut() {
        let dt = door.0;
        if dt.x < 0 || dt.y < 0 || dt.x >= grid.width as i32 || dt.y >= grid.height as i32 {
            continue;
        }
        let (tx, tz) = (dt.x as usize, dt.y as usize);

        // Only Once Door is Actually Passable
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

        // Block Closing if Player Overlapping Doorway in World Space
        if circle_overlaps_tile(player_xz, PLAYER_RADIUS + BLOCK_PAD, dt) {
            state.open_timer = RETRY_SECS_IF_BLOCKED;
            continue;
        }

        // Block Closing if Dead Enemy Body Overlaps Doorway in World Space
        if dead_xz.iter().any(|p| circle_overlaps_tile(*p, CORPSE_RADIUS + CORPSE_PAD, dt)) {
            state.open_timer = RETRY_SECS_IF_BLOCKED;
            continue;
        }

        // Block Closing if Something Still Claims Doorway Tile (Alive / Dying)
        if occupied_tiles.iter().any(|t| *t == dt) {
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
    mut enemy_fire: MessageReader<EnemyFire>,
    mut q_player: Query<&mut PlayerVitals, With<Player>>,
    god: Option<Res<GodMode>>,
) {
    let Some(mut vitals) = q_player.iter_mut().next() else {
        return;
    };

    let god_on = god.as_ref().map_or(false, |g| g.0);
    if god_on {
        // Still Consume Events (Read Them), Ignore Damage
        for _ in enemy_fire.read() {}
        return;
    }

    for e in enemy_fire.read() {
        vitals.hp = (vitals.hp - e.damage).max(0);
    }
}
