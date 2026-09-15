use bevy::dev_tools::diagnostics_overlay::{DiagnosticsOverlay, DiagnosticsOverlayPlugin};
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, Diagnostics, EntityCountDiagnosticsPlugin,
    FrameTimeDiagnosticsPlugin, RegisterDiagnostic,
};
use bevy::prelude::*;
use bevy::time::Real;
use bevy::ui::{Display, Node};
use game_input::{InputSet, ToggleDebugOverlay};

/// Sim ticks executed per real second. Unlike `Time<Fixed>`'s delta (which is
/// constant by definition), this drops below the configured `FixedUpdate` rate
/// if the sim can't keep up under load — the actual "is it keeping pace" signal.
const TPS: DiagnosticPath = DiagnosticPath::const_new("tps");

pub fn install(app: &mut App) {
    app.add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
        DiagnosticsOverlayPlugin,
    ))
    .register_diagnostic(Diagnostic::new(TPS).with_suffix(" tps"))
    .init_resource::<SimTickCount>()
    .add_systems(FixedUpdate, count_sim_tick)
    .add_systems(Update, (update_tps, toggle_overlay.after(InputSet)))
    .add_systems(Startup, spawn_overlay);
}

#[derive(Resource, Default)]
struct SimTickCount(u64);

fn count_sim_tick(mut count: ResMut<SimTickCount>) {
    count.0 += 1;
}

fn update_tps(
    mut diagnostics: Diagnostics,
    count: Res<SimTickCount>,
    mut last_count: Local<u64>,
    time: Res<Time<Real>>,
) {
    let delta_seconds = time.delta_secs_f64();
    if delta_seconds == 0.0 {
        return;
    }

    let ticks = count.0.saturating_sub(*last_count);
    *last_count = count.0;
    diagnostics.add_measurement(&TPS, || ticks as f64 / delta_seconds);
}

fn toggle_overlay(
    mut toggled: MessageReader<ToggleDebugOverlay>,
    mut overlays: Query<&mut Node, With<DiagnosticsOverlay>>,
) {
    if toggled.read().count() == 0 {
        return;
    }

    for mut node in &mut overlays {
        node.display = if node.display == Display::None {
            Display::DEFAULT
        } else {
            Display::None
        };
    }
}

fn spawn_overlay(mut commands: Commands) {
    commands.spawn(DiagnosticsOverlay::new(
        "Performance",
        vec![
            FrameTimeDiagnosticsPlugin::FPS.into(),
            FrameTimeDiagnosticsPlugin::FRAME_TIME.into(),
            TPS.into(),
            EntityCountDiagnosticsPlugin::ENTITY_COUNT.into(),
        ],
    ));
}
