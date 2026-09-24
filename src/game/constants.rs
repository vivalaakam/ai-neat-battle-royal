pub const DEFAULT_PLAYER_COUNT: usize = 32;
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
