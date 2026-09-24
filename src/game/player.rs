#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub id: usize,
    pub x: usize,
    pub y: usize,
    pub direction: usize,
    pub alive: bool,
    pub kills: u64,
    pub tiles_walked: u64,
    pub born_turn: u64,
    pub died_turn: Option<u64>,
}
