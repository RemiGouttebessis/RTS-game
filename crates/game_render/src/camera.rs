use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

const PAN_SPEED: f32 = 20.0;
const ZOOM_SPEED: f32 = 10.0;
const MIN_HEIGHT: f32 = 5.0;
const MAX_HEIGHT: f32 = 60.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (pan_camera, zoom_camera));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 25.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn pan_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction == Vec3::ZERO {
        return;
    }

    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    transform.translation += direction.normalize() * PAN_SPEED * time.delta_secs();
}

fn zoom_camera(
    mut scroll_events: MessageReader<MouseWheel>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let scroll: f32 = scroll_events.read().map(|event| event.y).sum();
    if scroll == 0.0 {
        return;
    }

    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    transform.translation.y =
        (transform.translation.y - scroll * ZOOM_SPEED).clamp(MIN_HEIGHT, MAX_HEIGHT);
}
