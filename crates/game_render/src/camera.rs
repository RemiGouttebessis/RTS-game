use bevy::prelude::*;
use game_config::CameraSettings;
use game_core::{GameState, GeneratedWorld};
use game_input::{CameraPanAction, CameraZoomAction, InputSet};
use game_worldgen::image_export;

const INITIAL_CAMERA_DISTANCE: f32 = 25.0;
/// Higher = the camera catches up to the target zoom distance faster (less
/// "floaty" lag). Exponential smoothing rate, not a duration.
const ZOOM_SMOOTHING: f32 = 10.0;
/// Same idea as `ZOOM_SMOOTHING` but for `GroundHeight` — lower than the zoom
/// smoothing rate, since terrain height changes (cresting a hill) should
/// read as the camera gliding over ground, not snapping the instant the
/// sampled cell's height changes.
const GROUND_SMOOTHING: f32 = 6.0;

/// Pitch (angle from horizontal the camera looks down at) and FOV both
/// interpolate between these as `zoom_fraction` goes from 0 (`min_height`)
/// to 1 (`max_height`) — this is what makes zooming out tilt the camera
/// toward top-down and flatten its perspective, the way Civ6's camera
/// actually behaves (it tilts more when zoomed in close, and flattens
/// toward near-vertical the further out you zoom), instead of the previous
/// fixed-angle camera that only ever moved straight up.
const PITCH_CLOSE: f32 = 0.611; // ~35° from horizontal
const PITCH_FAR: f32 = 1.484; // ~85° from horizontal — nearly top-down
/// `FOV_CLOSE` is Bevy's own `PerspectiveProjection` default (45°); `FOV_FAR`
/// is narrow enough that perspective distortion mostly disappears, reading
/// as close to an orthographic view without the complexity (and picking
/// implications) of actually switching `Projection` variants mid-flight.
const FOV_CLOSE: f32 = 0.785;
const FOV_FAR: f32 = 0.220;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraRig>()
            .init_resource::<CameraZoomTarget>()
            .init_resource::<CameraDistance>()
            .init_resource::<GroundHeight>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    pan_camera,
                    update_zoom_target,
                    apply_smooth_zoom,
                    follow_ground_height,
                    apply_camera_transform,
                )
                    .chain()
                    .after(InputSet)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

/// The ground point the camera orbits/looks at. Panning moves this in the
/// XZ plane (see `pan_speed_scale`) instead of moving the camera's own
/// translation directly the way the old fixed-angle camera did — the actual
/// camera position is now always derived from this target plus the current
/// distance/pitch (`apply_camera_transform`), so panning and zooming can't
/// disagree about where "the camera" conceptually is.
#[derive(Resource, Default)]
struct CameraRig {
    target: Vec3,
}

/// The distance from `CameraRig::target` scroll input is steering toward —
/// the camera itself eases toward this over several frames
/// (`apply_smooth_zoom`, into `CameraDistance`) rather than jumping straight
/// to it, so zooming reads as a smooth glide instead of a stepped snap on
/// every scroll notch.
#[derive(Resource)]
struct CameraZoomTarget(f32);

impl Default for CameraZoomTarget {
    fn default() -> Self {
        Self(INITIAL_CAMERA_DISTANCE)
    }
}

/// The camera's actual current distance from `CameraRig::target` — what
/// `apply_camera_transform` uses to place the camera and pick its
/// pitch/FOV. Separate from `CameraZoomTarget` (the *target* distance)
/// specifically so the smoothing in `apply_smooth_zoom` has somewhere to
/// ease *from* each frame. `pub(crate)` (unlike the other camera resources
/// here) because `game_render::map::toggle_world_map` needs it too — with
/// pitch now varying, the camera's `Transform.translation.y` is no longer
/// directly "how zoomed out is it" the way it was before this rig existed
/// (it's `distance * pitch.sin()`, which shrinks as pitch flattens), so
/// reading this resource is the only correct way left to ask that.
#[derive(Resource)]
pub(crate) struct CameraDistance(pub(crate) f32);

impl Default for CameraDistance {
    fn default() -> Self {
        Self(INITIAL_CAMERA_DISTANCE)
    }
}

/// Smoothed terrain height under `CameraRig::target`'s `(x, z)` — the orbit
/// rig's actual look-at point is `(target.x, ground.0, target.z)`, not
/// `target` itself (whose own `y` stays `0.0` forever; it's only ever
/// written to in the XZ plane by `pan_camera`). Without this, the rig pivots
/// around a fixed sea-level plane no matter what's actually under it, so
/// zooming in close over a mountain put the camera's `y` (derived from
/// `distance * pitch.sin()` around that fixed plane) *below* the peak,
/// clipping into the mesh. Tracking real ground height instead makes the
/// orbit hover a constant distance above whatever terrain is currently under
/// the target — smoothed (`follow_ground_height`) rather than snapped, so
/// cresting a hill reads as gliding over it, not a jump cut. Known
/// limitation: this only samples height under the *target*, not along the
/// line from the camera to it — a camera very close to a steep cliff face
/// between it and the target could still clip in principle; a real fix needs
/// a terrain raycast each frame, out of scope here.
#[derive(Resource, Default)]
struct GroundHeight(f32);

/// How far zoomed out `distance` is, `0.0` at `min_height` to `1.0` at
/// `max_height` — the one number pitch, FOV, and (in `game_render::map`)
/// the terrain/world-map crossover all derive from, so all three stay
/// consistent with each other as the camera zooms.
pub(crate) fn zoom_fraction(distance: f32, settings: &CameraSettings) -> f32 {
    let span = (settings.max_height - settings.min_height).max(0.001);
    ((distance - settings.min_height) / span).clamp(0.0, 1.0)
}

/// The camera's position relative to its look-at target, for a given
/// distance/pitch — always due "south" of the target (matching the original
/// fixed camera's fixed yaw), just at a distance/height set by
/// `distance`/`pitch` instead of a fixed angle.
fn camera_offset(distance: f32, pitch: f32) -> Vec3 {
    Vec3::new(0.0, distance * pitch.sin(), distance * pitch.cos())
}

fn spawn_camera(mut commands: Commands, settings: Res<CameraSettings>) {
    // No `GeneratedWorld` exists yet at `Startup` (world generation hasn't
    // run) — spawn at sea level (`y = 0`); `follow_ground_height` corrects
    // this the first frame `GameState::InGame` runs, before the player ever
    // sees it.
    let target = Vec3::ZERO;
    let fraction = zoom_fraction(INITIAL_CAMERA_DISTANCE, &settings);
    let pitch = PITCH_CLOSE.lerp(PITCH_FAR, fraction);
    let position = target + camera_offset(INITIAL_CAMERA_DISTANCE, pitch);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(position).looking_at(target, Vec3::Y),
    ));
}

/// Panning covers a lot more ground at a high, zoomed-out camera distance
/// than at `min_height` for the same screen-space input, so a flat
/// `pan_speed` feels brisk up close and glacial zoomed out — this scales it
/// by how many `min_height`s out the camera currently is. `sqrt` rather
/// than linear: linear would make max-distance panning `max_height /
/// min_height` (up to 120x, see `CameraSettings`) times faster than
/// min-distance panning, which covers ground so fast it's hard to control;
/// `sqrt` still speeds up noticeably with distance without being *that*
/// extreme.
fn pan_speed_scale(distance: f32, settings: &CameraSettings) -> f32 {
    (distance / settings.min_height).max(1.0).sqrt()
}

fn pan_camera(
    pan: Res<CameraPanAction>,
    settings: Res<CameraSettings>,
    time: Res<Time>,
    distance: Res<CameraDistance>,
    mut rig: ResMut<CameraRig>,
) {
    if pan.0 == Vec2::ZERO {
        return;
    }

    let scale = pan_speed_scale(distance.0, &settings);
    let direction = Vec3::new(pan.0.x, 0.0, pan.0.y);
    rig.target += direction * settings.pan_speed * scale * time.delta_secs();
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

/// Eases `CameraDistance` toward `CameraZoomTarget` every frame —
/// exponential smoothing (`1 - exp(-rate * dt)`) rather than a linear
/// `lerp(.., dt * rate)`, so the glide's speed doesn't depend on frame rate.
fn apply_smooth_zoom(
    target: Res<CameraZoomTarget>,
    time: Res<Time>,
    mut distance: ResMut<CameraDistance>,
) {
    let t = 1.0 - (-ZOOM_SMOOTHING * time.delta_secs()).exp();
    distance.0 = distance.0.lerp(target.0, t);
}

/// Samples real terrain height under `CameraRig::target` (via
/// `image_export::visual_height`, the exact same function the mesh itself is
/// built from — see `GroundHeight`'s doc comment) and exponential-smooths
/// `GroundHeight` toward it. A no-op if `GeneratedWorld` isn't populated yet
/// (`GroundHeight` just stays at its last/default value).
fn follow_ground_height(
    rig: Res<CameraRig>,
    generated: Res<GeneratedWorld>,
    time: Res<Time>,
    mut ground: ResMut<GroundHeight>,
) {
    let Some(generated) = generated.0.as_ref() else {
        return;
    };
    let world = &generated.world;
    let (col, row) =
        image_export::world_to_grid(world.width, world.height, rig.target.x, rig.target.z);
    let sampled = image_export::visual_height(world, generated.sea_level, col, row);

    let t = 1.0 - (-GROUND_SMOOTHING * time.delta_secs()).exp();
    ground.0 = ground.0.lerp(sampled, t);
}

/// Recomputes the camera's actual `Transform` and `PerspectiveProjection`
/// FOV from `CameraRig::target`/`GroundHeight` and `CameraDistance` every
/// frame — the one place any of pan/zoom/pitch/FOV/ground-following actually
/// touches the camera entity, so they can never fight each other or apply in
/// a stale order.
fn apply_camera_transform(
    rig: Res<CameraRig>,
    ground: Res<GroundHeight>,
    distance: Res<CameraDistance>,
    settings: Res<CameraSettings>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
) {
    let Ok((mut transform, mut projection)) = cameras.single_mut() else {
        return;
    };

    let fraction = zoom_fraction(distance.0, &settings);
    let pitch = PITCH_CLOSE.lerp(PITCH_FAR, fraction);
    let fov = FOV_CLOSE.lerp(FOV_FAR, fraction);

    let look_at = Vec3::new(rig.target.x, ground.0, rig.target.z);
    let position = look_at + camera_offset(distance.0, pitch);
    *transform = Transform::from_translation(position).looking_at(look_at, Vec3::Y);

    if let Projection::Perspective(perspective) = &mut *projection {
        perspective.fov = fov;
    }
}
