use bevy::prelude::*;
use game_input::{CameraPanAction, CameraZoomAction, InputSet};

const PAN_SPEED: f32 = 20.0;
const ZOOM_SPEED: f32 = 10.0;
const MIN_HEIGHT: f32 = 5.0;
const MAX_HEIGHT: f32 = 60.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (pan_camera, zoom_camera).after(InputSet));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn pan_camera(
    pan: Res<CameraPanAction>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    if pan.0 == Vec2::ZERO {
        return;
    }

    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    let direction = Vec3::new(pan.0.x, 0.0, pan.0.y);
    transform.translation += direction * PAN_SPEED * time.delta_secs();
}

fn zoom_camera(
    mut zoom_events: MessageReader<CameraZoomAction>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let zoom: f32 = zoom_events.read().map(|event| event.0).sum();
    if zoom == 0.0 {
        return;
    }

    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    transform.translation.y =
        (transform.translation.y - zoom * ZOOM_SPEED).clamp(MIN_HEIGHT, MAX_HEIGHT);
}
