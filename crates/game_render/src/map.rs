use bevy::prelude::*;
use game_core::GameState;

const GROUND_SIZE: f32 = 100.0;

/// Marks entities spawned by this plugin, so they can be cleaned up on
/// `OnExit(GameState::InGame)` (e.g. "Quit to Main Menu") without leaving
/// stale ground/light entities around for the next `OnEnter` to duplicate.
#[derive(Component)]
struct MapEntity;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), (spawn_ground, spawn_light))
            .add_systems(OnExit(GameState::InGame), despawn_map);
    }
}

fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        MapEntity,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(GROUND_SIZE, GROUND_SIZE))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.5, 0.2))),
        Transform::default(),
    ));
}

fn spawn_light(mut commands: Commands) {
    commands.spawn((
        MapEntity,
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn despawn_map(mut commands: Commands, entities: Query<Entity, With<MapEntity>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
