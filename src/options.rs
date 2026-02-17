/*
Davenstein - by David Petnick
*/
use bevy::camera;
use bevy::prelude::*;
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::window::{
	Monitor,
	MonitorSelection,
	PresentMode,
	PrimaryWindow,
	VideoModeSelection,
	WindowMode,
};

use crate::player;

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
	fn build(&self, app: &mut App) {
		app
			// Resources
			.init_resource::<VideoSettings>()
			.init_resource::<ControlSettings>()
			.init_resource::<SoundSettings>()
			.init_resource::<ResolutionList>()
			// Startup: Apply All Settings Once on Launch
			.add_systems(Startup, (
				populate_resolution_list,
				apply_video_settings_startup,
				apply_sound_settings_startup,
			).chain())
			// Update: Deal With Changes
			.add_systems(Update, (
				apply_video_settings_on_change,
				apply_view_size_on_change,
				apply_sound_settings_on_change,
				apply_control_settings_on_change,
			))
			// Debug Hotkeys (Gate Behind DEV Flag Later)
			.add_systems(Update, debug_toggle_vsync);
	}
}

//  VIDEO SETTINGS (Change View Screen)
/// Simplified Display Mode Which Maps to Bevy's 'WindowMode' Variants
/// Hide 'MonitorSelection' / 'VideoModeSelection' Complexity
/// Behind Sensible Defaults (Always use Current Monitor)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
	Windowed,
	#[default]
	BorderlessFullscreen,
	ExclusiveFullscreen,
}

impl DisplayMode {
	/// True if Exclusive Fullscreen Should Be Skipped
	/// (Wayland Does Not Support It)
	fn skip_exclusive() -> bool {
		std::env::var("WAYLAND_DISPLAY").is_ok()
	}

	/// Cycle Forward Through Display Modes (Wraps Around)
	/// Skips Exclusive Fullscreen on Wayland
	pub fn next(self) -> Self {
		let skip = Self::skip_exclusive();
		match self {
			DisplayMode::Windowed => DisplayMode::BorderlessFullscreen,
			DisplayMode::BorderlessFullscreen => {
				if skip {
					DisplayMode::Windowed
				} else {
					DisplayMode::ExclusiveFullscreen
				}
			}
			DisplayMode::ExclusiveFullscreen  => DisplayMode::Windowed,
		}
	}

	/// Cycle backward through display modes (wraps around)
	/// Skips Exclusive Fullscreen on Wayland
	pub fn prev(self) -> Self {
		let skip = Self::skip_exclusive();
		match self {
			DisplayMode::Windowed => {
				if skip {
					DisplayMode::BorderlessFullscreen
				} else {
					DisplayMode::ExclusiveFullscreen
				}
			}
			DisplayMode::BorderlessFullscreen => DisplayMode::Windowed,
			DisplayMode::ExclusiveFullscreen => DisplayMode::BorderlessFullscreen,
		}
	}

	/// Human readable label for the menu
	pub fn label(self) -> &'static str {
		match self {
			DisplayMode::Windowed => "Windowed",
			DisplayMode::BorderlessFullscreen => "Borderless",
			DisplayMode::ExclusiveFullscreen => "Fullscreen",
		}
	}
}

/// Which MSAA Preset User has Chosen
/// Bevy 0.18 Treats 'MSAA' as a *Camera Component*, so Apply System
/// Will Insert / Mutate it on any Camera Entity Tagged
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MsaaSetting {
	#[default]
	Off,
	Sample4,
}

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct VideoSettings {
	pub vsync: bool,
	pub display_mode: DisplayMode,
	/// Logical Resolution Used for 'Windowed' Mode
	/// Ignored in Fullscreen Modes (Monitor Decides)
	pub resolution: (u32, u32),
	/// Vertical FOV in *Degrees*. Clamped to 60..=120
	/// Camera Setup Should Read This via 'Res<VideoSettings>'
	pub fov: f32,
	/// Classic Wolfenstein 3D "View Size" (How Much Screen the 3-D
	/// Viewport Occupies vs HUD Border) Range 4..=20
	/// HUD / Viewport Layout Reads This
	pub view_size: u8,
	pub msaa: MsaaSetting,
}

impl Default for VideoSettings {
	fn default() -> Self {
		Self {
			vsync: true,
			display_mode: DisplayMode::default(),
			resolution: (1024, 768),
			fov: 40.0,
			view_size: 20,
			msaa: MsaaSetting::Off,
		}
	}
}

/// List of Available Resolutions for Windowed Mode
/// Populated at Startup from Monitor Query, Falls Back to
/// Common 16:9 Presets if Query Yields Nothing
#[derive(Resource, Clone)]
pub struct ResolutionList {
	pub entries: Vec<(u32, u32)>,
}

impl Default for ResolutionList {
	fn default() -> Self {
		Self {
			entries: vec![
				(640, 480),
				(800, 600),
				(1024, 768),
				(1280, 720),
				(1366, 768),
				(1600, 900),
				(1920, 1080),
				(2560, 1440),
				(3840, 2160),
			],
		}
	}
}

impl ResolutionList {
	/// Find the Index of the Given Resolution, or the Closest Match
	pub fn index_of(&self, res: (u32, u32)) -> usize {
		self.entries
			.iter()
			.position(|&r| r == res)
			.unwrap_or_else(|| {
				// Find closest by total pixel count
				let target = res.0 as i64 * res.1 as i64;
				self.entries
					.iter()
					.enumerate()
					.min_by_key(|(_, r)| {
						let (w, h) = **r;
						((w as i64 * h as i64) - target).abs()
					})
					.map(|(i, _)| i)
					.unwrap_or(0)
			})
	}

	/// Format a Resolution as a Menu Label
	pub fn label_at(&self, idx: usize) -> String {
		if let Some(&(w, h)) = self.entries.get(idx) {
			format!("{}x{}", w, h)
		} else {
			"???".to_string()
		}
	}
}

//  CONTROL SETTINGS (Controls Screen)
/// Rebindable Key Map for Modern WASD + Mouselook
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyBindings {
	pub move_forward:  KeyCode,
	pub move_backward: KeyCode,
	pub strafe_left:   KeyCode,
	pub strafe_right:  KeyCode,
	pub fire:          KeyCode,
	pub use_door:      KeyCode,
	pub run:           KeyCode,
	pub weapon_1:      KeyCode,
	pub weapon_2:      KeyCode,
	pub weapon_3:      KeyCode,
	pub weapon_4:      KeyCode,
}

impl Default for KeyBindings {
	fn default() -> Self {
		Self {
			move_forward:  KeyCode::KeyW,
			move_backward: KeyCode::KeyS,
			strafe_left:   KeyCode::KeyA,
			strafe_right:  KeyCode::KeyD,
			fire:          KeyCode::ControlLeft,
			use_door:      KeyCode::Space,
			run:           KeyCode::ShiftLeft,
			weapon_1:      KeyCode::Digit1,
			weapon_2:      KeyCode::Digit2,
			weapon_3:      KeyCode::Digit3,
			weapon_4:      KeyCode::Digit4,
		}
	}
}

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct ControlSettings {
	/// Multiplier Applied to Raw 'MouseMotion' Deltas
	/// Range: 0.1 ..= 10.0
	/// Default: 1.0
	pub mouse_sensitivity: f32,
	/// When True, Positive Mouse Y Input Looks *Down*
	pub invert_y: bool,
	/// Multiplier Applied to Right Stick Axes
	/// Range: 0.1 ..= 10.0
	/// Default: 1.0
	pub gamepad_sensitivity: f32,
	/// Inner Deadzone Radius for Gamepad Sticks
	/// Range: 0.0 ..= 0.5
	/// Default: 0.1
	/// Applied to 'GamepadSettings.default_axis_settings' on Every
	/// Connected Gamepad Entity
	pub gamepad_deadzone: f32,
	pub key_bindings: KeyBindings,
}

impl Default for ControlSettings {
	fn default() -> Self {
		Self {
			mouse_sensitivity: 1.0,
			invert_y: false,
			gamepad_sensitivity: 1.0,
			gamepad_deadzone: 0.1,
			key_bindings: KeyBindings::default(),
		}
	}
}

//  SOUND SETTINGS (Sound Screen)
/// Marker Component: Put This Bbackground Music Entity
/// so the Apply System can Find its 'AudioSink'
#[derive(Component)]
pub struct MusicTrack;

/// Marker Component: Put This on Sound Effect Entities
/// for Per Category Volume Control via 'AudioSink'
#[derive(Component)]
pub struct SfxSound;

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct SoundSettings {
	/// Overall Volume Multiplier (Written to 'GlobalVolume')
	/// Range: 0.0 ..= 1.0
	/// Default: 1.0
	pub master_volume: f32,
	/// Volume Scalar for Music Sinks
	/// Range: 0.0 ..= 1.0
	/// Default: 1.0
	pub music_volume: f32,
	/// Volume Scalar for SFX Sinks
	/// Range: 0.0 ..= 1.0
	/// Default: 1.0
	pub sfx_volume: f32,
	/// When False, Music Sinks Paused
	pub music_enabled: bool,
	/// When False, SFX Spawning Systems
	/// Should Early Return (Check Before Playing SFX)
	pub sfx_enabled: bool,
}

impl Default for SoundSettings {
	fn default() -> Self {
		Self {
			master_volume: 1.0,
			music_volume: 1.0,
			sfx_volume: 1.0,
			music_enabled: true,
			sfx_enabled: true,
		}
	}
}

//  Debug Hotkeys (Feature Gate Later)
pub const VSYNC_TOGGLE_KEY: KeyCode = KeyCode::F4;

fn debug_toggle_vsync(
	keys: Res<ButtonInput<KeyCode>>,
	mut settings: ResMut<VideoSettings>,
) {
	if keys.just_pressed(VSYNC_TOGGLE_KEY) {
		settings.vsync = !settings.vsync;
		info!("VSync toggled → {}", settings.vsync);
	}
}

//  VIDEO: Apply Systems
/// Try to Populate Resolution List from Monitor's Reported Video Modes
/// Falls Back to Default Preset List if Query Returns Nothing
fn populate_resolution_list(
	mut res_list: ResMut<ResolutionList>,
	q_monitors: Query<&Monitor>,
) {
	use std::collections::BTreeSet;

	let mut merged: BTreeSet<(u32, u32)> = res_list.entries.iter().copied().collect();
	let before = merged.len();

	let mut monitor_found = 0usize;

	for monitor in q_monitors.iter() {
		for mode in &monitor.video_modes {
			let w = mode.physical_size.x;
			let h = mode.physical_size.y;
			if w >= 640 && h >= 480 {
				monitor_found += 1;
				merged.insert((w, h));
			}
		}
	}

	if monitor_found == 0 {
		info!("No Monitor Video Modes Found, Keeping Fallback Resolution List");
		return;
	}

	let mut out: Vec<(u32, u32)> = merged.into_iter().collect();
	out.sort_by_key(|&(w, h)| ((w as u64) * (h as u64), w as u64, h as u64));

	info!(
		"Resolution list merged: {} -> {} entries ({} monitor modes seen)",
		before,
		out.len(),
		monitor_found
	);

	res_list.entries = out;
}

fn desired_present_mode(s: &VideoSettings) -> PresentMode {
	if s.vsync {
		PresentMode::AutoVsync
	} else {
		PresentMode::AutoNoVsync
	}
}

fn desired_window_mode(s: &VideoSettings) -> WindowMode {
	match s.display_mode {
		DisplayMode::Windowed            => WindowMode::Windowed,
		DisplayMode::BorderlessFullscreen => WindowMode::BorderlessFullscreen(
			MonitorSelection::Current,
		),
		DisplayMode::ExclusiveFullscreen  => WindowMode::Fullscreen(
			MonitorSelection::Current,
			VideoModeSelection::Current,
		),
	}
}

fn desired_msaa(s: &VideoSettings) -> Msaa {
	match s.msaa {
		MsaaSetting::Off     => Msaa::Off,
		MsaaSetting::Sample4 => Msaa::Sample4,
	}
}

/// Run Once at Startup to Make Sure Window Matches Defaults
fn apply_video_settings_startup(
	settings: Res<VideoSettings>,
	mut q_window: Query<&mut Window, With<PrimaryWindow>>,
	mut q_camera: Query<(&mut Msaa, &mut Projection), With<Camera>>,
) {
	if let Some(mut window) = q_window.iter_mut().next() {
		window.present_mode = desired_present_mode(&settings);
		window.mode = desired_window_mode(&settings);
		if settings.display_mode == DisplayMode::Windowed {
			let (w, h) = settings.resolution;
			window.resolution.set(w as f32, h as f32);
		}
	}

	let msaa = desired_msaa(&settings);
	let want_fov = settings.fov_radians();
	for (mut cam_msaa, mut projection) in q_camera.iter_mut() {
		*cam_msaa = msaa;
		if let Projection::Perspective(ref mut persp) = *projection {
			persp.fov = want_fov;
		}
	}
}

/// React Whenever *ANY* Field in 'VideoSettings' is Mutated
/// Only Write Fields That Differ From Current Window State
/// to Avoid Unnecessary Mode Switches / Resize Cascades
fn apply_video_settings_on_change(
	settings: Res<VideoSettings>,
	mut q_window: Query<&mut Window, With<PrimaryWindow>>,
	mut q_camera: Query<(&mut Msaa, &mut Projection), With<Camera>>,
) {
	if !settings.is_changed() {
		return;
	}

	if let Some(mut window) = q_window.iter_mut().next() {
		let want_present = desired_present_mode(&settings);
		if window.present_mode != want_present {
			window.present_mode = want_present;
		}

		let want_mode = desired_window_mode(&settings);
		if std::mem::discriminant(&window.mode) != std::mem::discriminant(&want_mode) {
			window.mode = want_mode;
		}

		if settings.display_mode == DisplayMode::Windowed {
			let (w, h) = settings.resolution;
			let (cur_w, cur_h) = (
				window.resolution.width() as u32,
				window.resolution.height() as u32,
			);
			if cur_w != w || cur_h != h {
				window.resolution.set(w as f32, h as f32);
			}
		}
	}

	let msaa = desired_msaa(&settings);
	let want_fov = settings.fov_radians();
	for (mut cam_msaa, mut projection) in q_camera.iter_mut() {
		if *cam_msaa != msaa {
			*cam_msaa = msaa;
		}
		if let Projection::Perspective(ref mut persp) = *projection {
			if (persp.fov - want_fov).abs() > 0.001 {
				persp.fov = want_fov;
			}
		}
	}
}

/// Apply Classic Wolfenstein 3D "View Size" by Setting Camera Viewport
/// view_size 20 = Full Viewport (No Border)
/// view_size 4  = Maximum Border (~80% Inset)
/// The Camera Viewport is Inset Symmetrically, Leaving a Border Area
/// That Shows the Window's Clear Color (Typically Dark Gray or Black)
/// The Status Bar (44 Native Pixels) is Accounted For: the Viewport
/// Only Shrinks the Area *Above* the Status Bar
/// 
/// IMPORTANT: Only Applies During Gameplay (When Player Exists)
/// This Prevents View Size Changes in Menus From Affecting Menu Rendering
///
/// Tracks Last Applied State so the Viewport is Also Set When
/// Entering Gameplay From the Menu (Not Just on Settings Change)
fn apply_view_size_on_change(
	settings: Res<VideoSettings>,
	player_query: Query<(), With<player::Player>>,
	q_window: Query<&Window, With<PrimaryWindow>>,
	mut q_camera: Query<&mut Camera, With<Camera3d>>,
	mut last_applied: Local<Option<(u8, bool)>>,
) {
	let has_player = !player_query.is_empty();
	let current = (settings.view_size, has_player);

	// Check if anything changed: settings, player existence, or first frame
	let needs_apply = match *last_applied {
		None => true,
		Some(prev) => prev != current || settings.is_changed(),
	};

	if !needs_apply {
		return;
	}

	*last_applied = Some(current);

	// Only Apply View Size Changes During Gameplay (When Player Exists)
	// In Menus, Always Use Full Viewport
	if !has_player {
		for mut cam in q_camera.iter_mut() {
			cam.viewport = None;
		}
		return;
	}

	let Some(window) = q_window.iter().next() else { return; };

	let win_w = window.resolution.physical_width();
	let win_h = window.resolution.physical_height();

	if win_w == 0 || win_h == 0 {
		return;
	}

	let vs = settings.view_size.clamp(4, 20) as f32;

	if vs >= 20.0 {
		// Full Viewport: Remove any Viewport Restriction
		for mut cam in q_camera.iter_mut() {
			cam.viewport = None;
		}
		return;
	}

	// Status Bar Height in Physical Pixels
	const HUD_W: f32 = 320.0;
	const STATUS_H: f32 = 44.0;
	let hud_scale = (win_w as f32 / HUD_W).floor().max(1.0);
	let status_h_phys = (STATUS_H * hud_scale) as u32;

	// Available Area Above Status Bar
	let view_h = win_h.saturating_sub(status_h_phys);
	if view_h == 0 {
		return;
	}

	// Inset Fraction: at view_size 4 Inset ~50%, at 19 Inset ~3%
	// Linear Mapping: Fraction = (20 - view_size) / 32
	// This Gives a Subtle Border at 19 and Large Border at 4
	let inset_frac = (20.0 - vs) / 32.0;

	let inset_x = (win_w as f32 * inset_frac).round() as u32;
	let inset_y = (view_h as f32 * inset_frac).round() as u32;

	let vp_x = inset_x;
	let vp_y = inset_y;
	let vp_w = win_w.saturating_sub(inset_x * 2).max(1);
	let vp_h = view_h.saturating_sub(inset_y * 2).max(1);

	let viewport = camera::Viewport {
		physical_position: UVec2::new(vp_x, vp_y),
		physical_size: UVec2::new(vp_w, vp_h),
		..default()
	};

	for mut cam in q_camera.iter_mut() {
		cam.viewport = Some(viewport.clone());
	}
}

//  SOUND: Apply Systems
/// Set the 'GlobalVolume' Resource on Startup
fn apply_sound_settings_startup(
	settings: Res<SoundSettings>,
	mut global_vol: ResMut<GlobalVolume>,
) {
	global_vol.volume = Volume::Linear(settings.master_volume);
}

/// React to *ANY* Change in 'SoundSettings'
///  'master_volume'  -> Written to 'GlobalVolume'
///  'music_volume'   -> Written to Every 'AudioSink' Tagged 'MusicTrack'
///  'sfx_volume'     -> Written to Every 'AudioSink' Tagged 'SfxSound'
///  'music_enabled'  -> Pause / Unpause Music Sinks
///  'sfx_enabled'    -> (Checked at *Play Time* by SFX Systems)
fn apply_sound_settings_on_change(
	settings: Res<SoundSettings>,
	mut global_vol: ResMut<GlobalVolume>,
	mut q_music: Query<&mut AudioSink, (With<MusicTrack>, Without<SfxSound>)>,
	mut q_sfx:   Query<&mut AudioSink, (With<SfxSound>, Without<MusicTrack>)>,
) {
	if !settings.is_changed() {
		return;
	}

	// Master
	global_vol.volume = Volume::Linear(settings.master_volume);

	// Music Sinks
	// In Bevy 0.18, AudioSink implements AudioSinkPlayback trait
	for mut sink in q_music.iter_mut() {
		sink.set_volume(Volume::Linear(settings.music_volume));
		if settings.music_enabled {
			AudioSinkPlayback::play(&*sink);
		} else {
			AudioSinkPlayback::pause(&*sink);
		}
	}

	// SFX Sinks (Any Currently Playing Sounds)
	for mut sink in q_sfx.iter_mut() {
		sink.set_volume(Volume::Linear(settings.sfx_volume));
	}
}

//  CONTROLS: Apply Systems
/// Push User's Deadzone Preference into Every Connected Gamepad's
/// 'GamepadSettings' Component
/// Mouse Sensitivity, Invert Y, Gamepad Sensitivity, and Key Bindings
/// Read Directly by Player Controller Systems From
/// 'Res<ControlSettings>', They Don't Need "Apply" System
fn apply_control_settings_on_change(
	settings: Res<ControlSettings>,
	mut q_gamepad: Query<&mut GamepadSettings>,
) {
	if !settings.is_changed() {
		return;
	}

	let dz = settings.gamepad_deadzone;
	for mut gp_settings in q_gamepad.iter_mut() {
		// Deadzone Defines "Ignore" Band Around Centre
		// These Setters Return Result and Silently Ignore Errors
		// (Which Only Occur if Lower > Upper, Which Shouldn't Happen Here)
		let _ = gp_settings.default_axis_settings.set_deadzone_lowerbound(-dz);
		let _ = gp_settings.default_axis_settings.set_deadzone_upperbound(dz);
	}
}

//  Public Helpers for Player Controller
#[allow(dead_code)]
impl ControlSettings {
	/// Returns Sensitivity Scaled, Invert Aware Look Delta
	/// From Raw 'MouseMotion' Input. Feed Result Straight
	/// into Camera Yaw / Pitch
	///
	/// ```ignore
	/// for ev in mouse_motion.read() {
	///     let (dx, dy) = controls.scaled_mouse_look(ev.delta);
	///     yaw   -= dx * delta_time;
	///     pitch -= dy * delta_time;
	/// }
	/// ```
	pub fn scaled_mouse_look(&self, raw_delta: Vec2) -> (f32, f32) {
		let dx = raw_delta.x * self.mouse_sensitivity;
		let dy = if self.invert_y {
			-raw_delta.y * self.mouse_sensitivity
		} else {
			raw_delta.y * self.mouse_sensitivity
		};
		(dx, dy)
	}

	/// Returns Sensitivity Scaled Right Stick Vector From Raw
	/// Gamepad Axis Values (-1..1 Each)
	pub fn scaled_gamepad_look(&self, stick_x: f32, stick_y: f32) -> (f32, f32) {
		(
			stick_x * self.gamepad_sensitivity,
			stick_y * self.gamepad_sensitivity,
		)
	}
}

impl VideoSettings {
	/// Returns FOV in *Radians*, Clamped, Ready for
	/// 'PerspectiveProjection { fov, .. }'
	pub fn fov_radians(&self) -> f32 {
		self.fov.clamp(40.0, 120.0).to_radians()
	}

	/// Nudge FOV by `delta` Degrees, Clamped to 40..=120
	pub fn nudge_fov(&mut self, delta: f32) {
		self.fov = (self.fov + delta).clamp(40.0, 120.0);
	}

	/// Nudge View Size by `delta`, Clamped to 4..=20
	pub fn nudge_view_size(&mut self, delta: i8) {
		let new_val = (self.view_size as i16 + delta as i16).clamp(4, 20) as u8;
		self.view_size = new_val;
	}

	/// Format FOV as Menu Label
	pub fn fov_label(&self) -> String {
		format!("{}", self.fov.clamp(40.0, 120.0) as u32)
	}

	/// Format View Size as Menu Label
	pub fn view_size_label(&self) -> String {
		format!("{}", self.view_size)
	}
}

#[allow(dead_code)]
impl SoundSettings {
	/// Quick Check SFX Spawning Systems Should Call Before
	/// Spawning New Sound Entity
	pub fn should_play_sfx(&self) -> bool {
		self.sfx_enabled && self.sfx_volume > 0.0
	}

	/// Effective Linear Volume to set on *NEW* SFX 'PlaybackSettings'
	/// Combines Per Category Scalar so 'GlobalVolume' can Stay as
	/// True Master Knob
	pub fn effective_sfx_volume(&self) -> f32 {
		self.sfx_volume
	}

	/// Effective Linear Volume to set on *NEW* Music 'PlaybackSettings'
	pub fn effective_music_volume(&self) -> f32 {
		self.music_volume
	}
}
