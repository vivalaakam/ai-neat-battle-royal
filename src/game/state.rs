use vivalaakam_neuro_neat::Organism;

use super::brain::new_brains;
use super::constants::DIRECTIONS;
use super::player::Player;
use super::rng::Rng;

pub struct Game {
    pub seed: String,
    pub generation: u64,
    pub width: usize,
    pub height: usize,
    pub player_count: usize,
    pub turn: u64,
    pub walls: Vec<bool>,
    pub players: Vec<Player>,
    pub brains: Vec<Organism>,
}

impl Game {
    pub fn new(
        seed: String,
        generation: u64,
        width: usize,
        height: usize,
        player_count: usize,
    ) -> Self {
        let brains = new_brains(&seed, generation, player_count, None);
        Self::with_brains(seed, generation, width, height, player_count, brains)
    }

    pub fn with_brains(
        seed: String,
        generation: u64,
        width: usize,
        height: usize,
        player_count: usize,
        brains: Vec<Organism>,
    ) -> Self {
        assert_eq!(
            brains.len(),
            player_count,
            "brain count must match player count"
        );
        let width = width.max(20);
        let height = height.max(12);
        let mut game = Self {
            seed: seed.clone(),
            generation,
            width,
            height,
            player_count,
            turn: 0,
            walls: vec![false; width * height],
            players: Vec::with_capacity(player_count + 1),
            brains,
        };
        let mut rng = Rng::from_phrase(&seed, generation);
        game.generate(&mut rng);
        game
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn is_wall(&self, x: usize, y: usize) -> bool {
        self.walls[self.index(x, y)]
    }

    fn set_wall(&mut self, x: usize, y: usize) {
        let index = y * self.width + x;
        self.walls[index] = true;
    }

    fn generate(&mut self, rng: &mut Rng) {
        for y in 0..self.height {
            for x in 0..self.width {
                if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
                    self.set_wall(x, y);
                }
            }
        }
        for _ in 0..(self.width * self.height / 55) {
            let (mut x, mut y) = (
                2 + rng.range(self.width - 5),
                2 + rng.range(self.height - 5),
            );
            let (dx, dy) = DIRECTIONS[rng.range(DIRECTIONS.len())];
            for _ in 0..(2 + rng.range(5)) {
                if x > 0 && y > 0 && x + 1 < self.width && y + 1 < self.height {
                    self.set_wall(x, y);
                }
                x = x.saturating_add_signed(dx);
                y = y.saturating_add_signed(dy);
            }
        }
        let mut free: Vec<_> = (1..self.height - 1)
            .flat_map(|y| (1..self.width - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| !self.is_wall(x, y))
            .collect();
        assert!(
            free.len() >= self.player_count,
            "map is too small for the requested player count"
        );
        for id in 1..=self.player_count {
            let pick = rng.range(free.len());
            let (x, y) = free.swap_remove(pick);
            self.players.push(Player {
                id,
                x,
                y,
                direction: rng.range(DIRECTIONS.len()),
                alive: true,
                kills: 0,
                tiles_walked: 1,
                born_turn: 0,
                died_turn: None,
            });
        }
    }

    pub fn score(&self, player: &Player) -> u64 {
        (player.died_turn.unwrap_or(self.turn) - player.born_turn)
            + player.kills * 10
            + player.tiles_walked
    }

    pub fn record_step(&mut self, player_index: usize) {
        if let Some(player) = self.players.get_mut(player_index) {
            player.tiles_walked += 1;
        }
    }

    pub fn alive_agent_count(&self) -> usize {
        self.players.iter().filter(|p| p.alive && p.id > 0).count()
    }
}
