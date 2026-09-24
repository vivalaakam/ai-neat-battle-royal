mod brain;
mod constants;
mod human;
mod lineage;
pub mod models;
mod neat;
mod player;
mod rng;
mod selection;
mod state;
mod turn;
mod vision;

pub use brain::breed_next_generation;
pub use selection::tournament_select;

pub use lineage::organism_serial;
pub use models::{log_generation_best, max_lineage_serial, save_best_if_new};

pub use constants::{
    DEFAULT_MAP_HEIGHT, DEFAULT_MAP_WIDTH, DEFAULT_PLAYER_COUNT, OBSERVATION_SIZE, VISION_DEPTH,
};
pub use state::Game;
