use bevy::prelude::*;

use crate::game::Game;

pub const TILE: f32 = 18.0;
pub const MAP_ZOOM_MIN: f32 = 0.2;
pub const MAP_ZOOM_MAX: f32 = 5.0;
pub const MAP_ZOOM_FACTOR: f32 = 1.12;
pub const MAP_PAN_SPEED: f32 = 480.0;
pub const LEADERBOARD_PANEL_PX: f32 = 420.0;
pub const AUTO_TURN_INTERVAL_SECS: f32 = 0.04;

#[derive(Resource)]
pub struct MapView {
    pub pan: Vec2,
    pub zoom: f32,
}

#[derive(Resource)]
pub struct Battle {
    pub game: Game,
    pub seed: String,
    pub generation: u64,
    pub paused: bool,
    pub dirty: bool,
    pub max_generation: Option<u64>,
    pub breed_lineage: bool,
    pub lineage_serial: u64,
}

#[derive(Resource)]
pub struct AutoRun {
    pub max_step: Option<u64>,
    pub armed: bool,
    pub running: bool,
    pub turns_this_generation: u64,
}

impl AutoRun {
    pub fn new(max_step: Option<u64>) -> Self {
        Self {
            max_step,
            armed: false,
            running: false,
            turns_this_generation: 0,
        }
    }

    pub fn configured(&self) -> bool {
        self.max_step.is_some()
    }
}
