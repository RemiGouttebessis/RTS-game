use bevy::prelude::*;

/// Marks an entity as a controllable RTS unit.
///
/// Kept as a plain marker; unit data (health, orders, ownership) belongs in its
/// own small, orthogonal components rather than growing this into a god-component.
#[derive(Component)]
pub struct Unit;
