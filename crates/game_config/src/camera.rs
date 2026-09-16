use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct CameraSettings {
    /// Pan speed at `min_height` — `game_render::camera::pan_camera` scales
    /// this up with the camera's current height, so panning covers more
    /// real ground per second the further you've zoomed out (covering the
    /// same *screen* distance at a higher altitude means covering a lot
    /// more world, so the base speed alone would feel like crawling once
    /// zoomed out far).
    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed: 20.0,
            zoom_speed: 40.0,
            min_height: 5.0,
            max_height: 600.0,
        }
    }
}
