use bevy::prelude::*;

mod camera;
mod map;
mod units;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "RTS Game".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((camera::CameraPlugin, map::MapPlugin, units::UnitsPlugin))
        .run();
}
