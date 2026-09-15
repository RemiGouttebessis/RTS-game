use bevy::prelude::*;
use game_config::CameraSettings;
use game_input::{CameraPanAction, CameraZoomAction, InputSet};

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
    let direction = Vec3::new(pan.0.x, 0.0, pan.0.y);
    transform.translation += direction * settings.pan_speed * time.delta_secs();
}

fn zoom_camera(
    mut zoom_events: MessageReader<CameraZoomAction>,
    settings: Res<CameraSettings>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let zoom: f32 = zoom_events.read().map(|event| event.0).sum();
    if zoom == 0.0 {
        return;
    }

    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    transform.translation.y = (transform.translation.y - zoom * settings.zoom_speed)
        .clamp(settings.min_height, settings.max_height);
}
