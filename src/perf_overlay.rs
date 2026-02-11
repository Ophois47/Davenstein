/*
Davenstein - by David Petnick
*/
use bevy::diagnostic::{
	DiagnosticsStore,
	EntityCountDiagnosticsPlugin,
	FrameTimeDiagnosticsPlugin,
};
use bevy::prelude::*;

pub const PERF_OVERLAY_TOGGLE_KEY: KeyCode = KeyCode::F3;

pub struct PerfOverlayPlugin;

impl Plugin for PerfOverlayPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PerfOverlayState>()
			.add_plugins((
				FrameTimeDiagnosticsPlugin::default(),
				EntityCountDiagnosticsPlugin::default(),
			))
			.add_systems(Startup, perf_overlay_setup)
			.add_systems(Update, (toggle_perf_overlay, update_perf_overlay_text));
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

#[derive(Component)]
struct PerfOverlayRoot;

#[derive(Component)]
struct PerfFpsText;

#[derive(Component)]
struct PerfFrameMsText;

#[derive(Component)]
struct PerfEntityCountText;

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
					font: ui_font.clone(),
					font_size: 32.0,
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: ui_font.clone(),
					font_size: 32.0,
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfFpsText,
			));

			root.spawn((
				Text::new("Frame ms: "),
				TextFont {
					font: ui_font.clone(),
					font_size: 32.0,
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: ui_font.clone(),
					font_size: 32.0,
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfFrameMsText,
			));

			root.spawn((
				Text::new("Entities: "),
				TextFont {
					font: ui_font.clone(),
					font_size: 32.0,
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
			))
			.with_child((
				TextSpan::default(),
				TextFont {
					font: ui_font.clone(),
					font_size: 32.0,
					..default()
				},
				TextColor(Color::srgba(1.0, 1.0, 1.0, 1.0)),
				PerfEntityCountText,
			));
		});
}

fn toggle_perf_overlay(
	keys: Res<ButtonInput<KeyCode>>,
	mut state: ResMut<PerfOverlayState>,
	mut q_root_vis: Query<&mut Visibility, With<PerfOverlayRoot>>,
) {
	if !keys.just_pressed(PERF_OVERLAY_TOGGLE_KEY) {
		return;
	}

	state.enabled = !state.enabled;

	if let Ok(mut vis) = q_root_vis.single_mut() {
		*vis = if state.enabled {
			Visibility::Visible
		} else {
			Visibility::Hidden
		};
	}
}

fn update_perf_overlay_text(
	time: Res<Time>,
	mut state: ResMut<PerfOverlayState>,
	diagnostics: Res<DiagnosticsStore>,
	mut spans: ParamSet<(
		Query<&mut TextSpan, With<PerfFpsText>>,
		Query<&mut TextSpan, With<PerfFrameMsText>>,
		Query<&mut TextSpan, With<PerfEntityCountText>>,
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
}
