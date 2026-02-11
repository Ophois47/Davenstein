/*
Davenstein - by David Petnick
*/
use bevy::prelude::*;
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::window::{
	MonitorSelection,
	PresentMode,
	PrimaryWindow,
	VideoModeSelection,
	WindowMode,
};

pub struct OptionsPlugin;

impl Plugin for OptionsPlugin {
	fn build(&self, app: &mut App) {
		app
			// Resources
			.init_resource::<VideoSettings>()
			.init_resource::<ControlSettings>()
			.init_resource::<SoundSettings>()
			// Startup: Apply All Settings Once on Launch
			.add_systems(Startup, (
				apply_video_settings_startup,
				apply_sound_settings_startup,
			))
			// Update: Deal With Changes
			.add_systems(Update, (
				apply_video_settings_on_change,
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

/// Which MSAA Preset User has Chosen
/// Bevy 0.18 Treats 'MSAA' as a *Camera Component*, so Apply System
/// Will Insert / Mutate it on any Camera Entity Tagged
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
			resolution: (1280, 720),
			fov: 90.0,
			view_size: 20,
			msaa: MsaaSetting::Off,
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
			fire:          KeyCode::Space,
			use_door:      KeyCode::KeyE,
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
	mut q_camera: Query<&mut Msaa, With<Camera>>,
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
	for mut cam_msaa in q_camera.iter_mut() {
		*cam_msaa = msaa;
	}
}

/// React Whenever *ANY* Field in 'VideoSettings' is Mutated
fn apply_video_settings_on_change(
	settings: Res<VideoSettings>,
	mut q_window: Query<&mut Window, With<PrimaryWindow>>,
	mut q_camera: Query<&mut Msaa, With<Camera>>,
) {
	if !settings.is_changed() {
		return;
	}

	if let Some(mut window) = q_window.iter_mut().next() {
		window.present_mode = desired_present_mode(&settings);
		window.mode = desired_window_mode(&settings);
		if settings.display_mode == DisplayMode::Windowed {
			let (w, h) = settings.resolution;
			window.resolution.set(w as f32, h as f32);
		}
	}

	let msaa = desired_msaa(&settings);
	for mut cam_msaa in q_camera.iter_mut() {
		*cam_msaa = msaa;
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
	for mut sink in q_music.iter_mut() {
		sink.set_volume(Volume::Linear(settings.music_volume));
		if settings.music_enabled {
			sink.play();
		} else {
			sink.pause();
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
		self.fov.clamp(60.0, 120.0).to_radians()
	}
}

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
