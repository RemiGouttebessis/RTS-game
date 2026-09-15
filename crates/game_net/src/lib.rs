use bevy::prelude::*;

/// Optional networked multiplayer (`bevy_replicon` or `lightyear`, TBD).
/// Not wired into `game` yet — pull it in once an authority model
/// (lockstep vs. server-authoritative) is decided. No systems yet.
pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, _app: &mut App) {}
}
