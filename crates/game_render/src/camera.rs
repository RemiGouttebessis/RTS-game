use bevy::prelude::*;
use game_config::CameraSettings;
use game_core::GameState;
use game_input::{CameraPanAction, CameraZoomAction, InputSet};

const INITIAL_CAMERA_HEIGHT: f32 = 25.0;
/// Higher = the camera catches up to the target zoom height faster (less
/// "floaty" lag). Exponential smoothing rate, not a duration.
const ZOOM_SMOOTHING: f32 = 10.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraZoomTarget>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (pan_camera, (update_zoom_target, apply_smooth_zoom).chain())
                    .after(InputSet)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

/// The height scroll input is steering toward — the camera itself eases
/// toward this over several frames (`apply_smooth_zoom`) rather than
/// jumping straight to it, so zooming reads as a smooth glide instead of a
/// stepped snap on every scroll notch.
#[derive(Resource)]
struct CameraZoomTarget(f32);

impl Default for CameraZoomTarget {
    fn default() -> Self {
        Self(INITIAL_CAMERA_HEIGHT)
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, INITIAL_CAMERA_HEIGHT, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Panning covers a lot more ground at a high, zoomed-out camera height than
/// at `min_height` for the same screen-space input, so a flat `pan_speed`
/// feels brisk up close and glacial zoomed out — this scales it by how many
/// `min_height`s up the camera currently is. `sqrt` rather than linear:
/// linear would make max-height panning `max_height / min_height` (up to
/// 120x, see `CameraSettings`) times faster than min-height panning, which
/// covers ground so fast it's hard to control; `sqrt` still speeds up
/// noticeably with altitude without being *that* extreme.
fn pan_speed_scale(camera_height: f32, settings: &CameraSettings) -> f32 {
    (camera_height / settings.min_height).max(1.0).sqrt()
}

fn pan_camera(
    pan: Res<CameraPanAction>,
    settings: Res<CameraSettings>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    if pan.0 == Vec2::ZERO {
        return;
    }

    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    let scale = pan_speed_scale(transform.translation.y, &settings);
    let direction = Vec3::new(pan.0.x, 0.0, pan.0.y);
    transform.translation += direction * settings.pan_speed * scale * time.delta_secs();
}

fn update_zoom_target(
    mut zoom_events: MessageReader<CameraZoomAction>,
    settings: Res<CameraSettings>,
    mut target: ResMut<CameraZoomTarget>,
) {
    let zoom: f32 = zoom_events.read().map(|event| event.0).sum();
    if zoom == 0.0 {
        return;
    }

    target.0 =
        (target.0 - zoom * settings.zoom_speed).clamp(settings.min_height, settings.max_height);
}

/// Eases the camera's actual height toward `CameraZoomTarget` every frame —
/// exponential smoothing (`1 - exp(-rate * dt)`) rather than a linear
/// `lerp(.., dt * rate)`, so the glide's speed doesn't depend on frame rate.
fn apply_smooth_zoom(
    target: Res<CameraZoomTarget>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    let t = 1.0 - (-ZOOM_SMOOTHING * time.delta_secs()).exp();
    transform.translation.y = transform.translation.y.lerp(target.0, t);
}
