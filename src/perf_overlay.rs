/*
Davenstein - by David Petnick
*/

use bevy::diagnostic::{
	DiagnosticsStore,
	EntityCountDiagnosticsPlugin,
	FrameTimeDiagnosticsPlugin,
};
use bevy::prelude::*;

use std::collections::VecDeque;

pub const PERF_OVERLAY_TOGGLE_KEY: KeyCode = KeyCode::F3;

pub struct PerfOverlayPlugin;

impl Plugin for PerfOverlayPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PerfOverlayState>()
			.init_resource::<FramePacing>()
			.add_plugins((
				FrameTimeDiagnosticsPlugin::default(),
				EntityCountDiagnosticsPlugin::default(),
			))
			.add_systems(Startup, perf_overlay_setup)
			// Count Fixed Simulation Tics as They Happen. 'FixedUpdate' Runs Zero or
			// More Times Inside Each Rendered Frame, Before 'Update', so by the Time
			// 'record_frame_pacing' Reads the Counter It Holds This Frame's Tic Count
			.add_systems(FixedUpdate, count_fixed_tics.run_if(|s: Res<PerfOverlayState>| s.enabled))
			// Chained so Each Frame Is: Handle Toggle, Record the Frame Sample,
			// Then Refresh the Text From the Freshly Recorded Window
			.add_systems(Update, (
				toggle_perf_overlay,
				record_frame_pacing.run_if(|s: Res<PerfOverlayState>| s.enabled),
				update_perf_overlay_text,
			).chain());
	}
}

#[derive(Resource)]
pub struct PerfOverlayState {
	pub enabled: bool,
	pub update_timer: Timer,
}

impl Default for PerfOverlayState {
	fn default() -> Self {
		Self {
			enabled: false,
			update_timer: Timer::from_seconds(0.25, TimerMode::Repeating),
		}
	}
}

/// Number of Recent Frames Kept in the Pacing Window (Two Seconds at 60 Hz)
const PACING_WINDOW: usize = 120;

/// A Frame Longer Than This Missed the 60 Hz Vsync Deadline (16.67 ms Plus a
/// Small Tolerance for Timer Jitter). Under FIFO Presentation Such a Frame Is
/// Held to the Next Refresh, Which Is Exactly What Makes an FPS Counter Bounce
const FRAME_BUDGET_MS: f32 = 17.0;

/// Rolling Record of Recent Frame Times Paired With the Number of Fixed
/// Simulation Tics That Ran Inside Each Frame
///
/// This Exists to Answer One Question Unambiguously: When a Frame Misses the
/// Vsync Budget, Was It a Frame That Ran *Two* 70 Hz Simulation Tics? A 70 Hz
/// Simulation Presented at 60 Hz Must Run Two Tics on One Frame in Six (Seven
/// Tics per Six Frames), so if the Late Frames Are Exactly the Two-Tic Frames,
/// the Fixed-Step Workload Is the Culprit; if Frames Run Late Even With One
/// Tic, the Cost Lives on the Render Side Instead. The Two Buffers Are Pushed
/// Together and Trimmed Together, so Index i in Each Describes the Same Frame
#[derive(Resource, Default)]
struct FramePacing {
	/// Duration of Each Recent Frame in Milliseconds, Oldest First
	frame_ms: VecDeque<f32>,
	/// Fixed Simulation Tics That Ran Inside Each Recent Frame, Oldest First
	frame_tics: VecDeque<u32>,
	/// Tics Counted so Far in the Frame Currently Being Measured
	tics_this_frame: u32,
}

impl FramePacing {
	/// Discard All Samples, Used When the Overlay Is Switched On so Stale Data
	/// From a Previous Session Never Colors the Fresh Readings
	fn reset(&mut self) {
		self.frame_ms.clear();
		self.frame_tics.clear();
		self.tics_this_frame = 0;
	}
}

#[derive(Component)]
struct PerfOverlayRoot;

#[derive(Component)]
struct PerfFpsText;

#[derive(Component)]
struct PerfFrameMsText;

#[derive(Component)]
struct PerfEntityCountText;

#[derive(Component)]
struct PerfWorstMsText;

#[derive(Component)]
struct PerfLateFramesText;

#[derive(Component)]
struct PerfTwoTicText;

fn perf_overlay_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	let ui_font = asset_server.load("fonts/honda_font.ttf");

	commands
		.spawn((
			Name::new("perf_overlay"),
			PerfOverlayRoot,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(8.0),
				top: Val::Px(8.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(2.0),
				padding: UiRect::all(Val::Px(6.0)),
				..default()
			},
			BackgroundColor(Srgba::new(0.0, 0.0, 0.0, 0.65).into()),
			Visibility::Hidden,
		))
		.with_children(|root| {
			root.spawn((
				Text::new("FPS: "),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfFpsText,
			));

			root.spawn((
				Text::new("Frame ms: "),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfFrameMsText,
			));

			root.spawn((
				Text::new("Entities: "),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfEntityCountText,
			));

			// Worst Frame in the Pacing Window: a Steady 60 Hz Should Hold This
			// Near 16.7; Any Excursion Past ~17 Is a Missed Vsync Deadline
			root.spawn((
				Text::new("Worst ms: "),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfWorstMsText,
			));

			// Late Frames Over the Window, and How Many of Those Ran Two Sim Tics
			// If the Two Numbers Match, the Doubled 70 Hz Tic Is the Culprit
			root.spawn((
				Text::new("Late: "),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfLateFramesText,
			));

			// Share of Frames That Ran Two or More Fixed Tics. Seventy Hz Sim on a
			// Sixty Hz Display Should Sit Near 17 Percent; This Row Is the Baseline
			// the 'Late' Row Is Judged Against
			root.spawn((
				Text::new("2-Tic: "),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: FontSource::Handle(ui_font.clone()),
					font_size: FontSize::Px(32.0),
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfTwoTicText,
			));
		});
}

fn toggle_perf_overlay(
	keys: Res<ButtonInput<KeyCode>>,
	mut state: ResMut<PerfOverlayState>,
	mut pacing: ResMut<FramePacing>,
	mut q_root_vis: Query<&mut Visibility, With<PerfOverlayRoot>>,
) {
	if !keys.just_pressed(PERF_OVERLAY_TOGGLE_KEY) {
		return;
	}

	state.enabled = !state.enabled;

	// Start Each Session From an Empty Window so the Readings Reflect Only
	// What Happens While the Overlay Is Actually Being Watched
	if state.enabled {
		pacing.reset();
	}

	if let Ok(mut vis) = q_root_vis.single_mut() {
		*vis = if state.enabled {
			Visibility::Visible
		} else {
			Visibility::Hidden
		};
	}
}

/// Runs Once per Fixed Simulation Tic ('FixedUpdate'). Because Every Fixed Tic
/// in a Frame Executes Before That Frame's 'Update' Schedule, the Counter Read
/// by 'record_frame_pacing' Is Exactly the Number of Tics This Frame Performed
fn count_fixed_tics(mut pacing: ResMut<FramePacing>) {
	pacing.tics_this_frame += 1;
}

/// Runs Every Rendered Frame While the Overlay Is Enabled. Appends This Frame's
/// Duration and Tic Count as One Paired Sample, Trims the Window to Its Fixed
/// Size, and Zeroes the Tic Counter for the Next Frame. Uses the Raw Frame
/// Delta ('Time::delta') Rather Than a Smoothed Diagnostic Because Missed
/// Deadlines Are Precisely the Outliers Smoothing Hides
fn record_frame_pacing(time: Res<Time>, mut pacing: ResMut<FramePacing>) {
	let ms = time.delta().as_secs_f32() * 1000.0;
	let tics = pacing.tics_this_frame;

	if pacing.frame_ms.len() >= PACING_WINDOW {
		pacing.frame_ms.pop_front();
		pacing.frame_tics.pop_front();
	}

	pacing.frame_ms.push_back(ms);
	pacing.frame_tics.push_back(tics);
	pacing.tics_this_frame = 0;
}

fn update_perf_overlay_text(
	time: Res<Time>,
	mut state: ResMut<PerfOverlayState>,
	diagnostics: Res<DiagnosticsStore>,
	pacing: Res<FramePacing>,
	mut spans: ParamSet<(
		Query<&mut TextSpan, With<PerfFpsText>>,
		Query<&mut TextSpan, With<PerfFrameMsText>>,
		Query<&mut TextSpan, With<PerfEntityCountText>>,
		Query<&mut TextSpan, With<PerfWorstMsText>>,
		Query<&mut TextSpan, With<PerfLateFramesText>>,
		Query<&mut TextSpan, With<PerfTwoTicText>>,
	)>,
) {
	if !state.enabled {
		return;
	}

	if !state.update_timer.tick(time.delta()).just_finished() {
		return;
	}

	let fps = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FPS)
		.and_then(|d| d.smoothed());

	let frame_ms = diagnostics
		.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
		.and_then(|d| d.smoothed());

	let entities = diagnostics
		.get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
		.and_then(|d| d.smoothed());

	// Walk the Paired Window Once: Worst Frame, Frames Over the Vsync Budget,
	// How Many of Those Late Frames Ran Two or More Sim Tics, and the Overall
	// Share of Two-Tic Frames. The Late-vs-Two-Tic Split Is the Verdict: Late
	// Only on Two-Tic Frames Means the Fixed-Step Workload Overruns the Budget;
	// Late on One-Tic Frames Means the Cost Lives Elsewhere in the Frame
	let mut worst_ms = 0.0f32;
	let mut late = 0usize;
	let mut late_two_tic = 0usize;
	let mut two_tic = 0usize;
	for (ms, tics) in pacing.frame_ms.iter().zip(pacing.frame_tics.iter()) {
		worst_ms = worst_ms.max(*ms);
		if *tics >= 2 {
			two_tic += 1;
		}
		if *ms > FRAME_BUDGET_MS {
			late += 1;
			if *tics >= 2 {
				late_two_tic += 1;
			}
		}
	}
	let samples = pacing.frame_ms.len();

	if let Some(mut span) = spans.p0().iter_mut().next() {
		span.0 = fps.map(|v| format!("{v:5.1}")).unwrap_or_else(|| "  n/a".to_string());
	}

	if let Some(mut span) = spans.p1().iter_mut().next() {
		span.0 = frame_ms.map(|v| format!("{v:5.2}")).unwrap_or_else(|| "  n/a".to_string());
	}

	if let Some(mut span) = spans.p2().iter_mut().next() {
		span.0 = entities
			.map(|v| format!("{:6}", v.round() as u64))
			.unwrap_or_else(|| "   n/a".to_string());
	}

	if let Some(mut span) = spans.p3().iter_mut().next() {
		span.0 = if samples == 0 {
			"  n/a".to_string()
		} else {
			format!("{worst_ms:5.1}")
		};
	}

	if let Some(mut span) = spans.p4().iter_mut().next() {
		span.0 = if samples == 0 {
			"  n/a".to_string()
		} else {
			format!("{late}/{samples} ({late_two_tic} on 2-tic)")
		};
	}

	if let Some(mut span) = spans.p5().iter_mut().next() {
		span.0 = if samples == 0 {
			"  n/a".to_string()
		} else {
			format!("{:3.0}%", two_tic as f32 * 100.0 / samples as f32)
		};
	}
}
