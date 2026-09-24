pub const DEFAULT_PLAYER_COUNT: usize = 32;
pub const DEFAULT_MAP_WIDTH: usize = 128;
pub const DEFAULT_MAP_HEIGHT: usize = 64;
pub const SPAWN_CELL: usize = 16;
pub const TREAT_SCORE: u64 = 10;
pub const KILL_BASE_SCORE: u64 = 100;
/// Rough treat density: one treat per N free tiles.
pub const TREAT_TILE_DIVISOR: usize = 40;
pub const VISION_DEPTH: usize = 6;
pub const OBSERVATION_SIZE: usize = 2 + 8 + (VISION_DEPTH * 2 + 1) * (VISION_DEPTH * 2 + 1);
pub const ACTIONS: [&str; 5] = ["left", "right", "forward", "shoot", "wait"];
pub const DIRECTIONS: [(isize, isize); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];
