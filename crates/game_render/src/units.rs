use bevy::prelude::*;
use game_core::Unit;

const UNIT_COUNT: i32 = 5;
const UNIT_SPACING: f32 = 2.0;

/// Spawns unit visuals directly, as a placeholder. Once `game_sim` owns unit
/// spawning, this becomes a system that reacts to newly spawned sim units
/// (`Added<Unit>`) instead of creating the entities itself.
pub struct UnitsPlugin;

impl Plugin for UnitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_units);
    }
}

fn spawn_units(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let material = materials.add(Color::srgb(0.8, 0.2, 0.2));

    for i in 0..UNIT_COUNT {
        let x = (i as f32 - UNIT_COUNT as f32 / 2.0) * UNIT_SPACING;
        commands.spawn((
            Unit,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, 0.5, 0.0),
        ));
    }
}
