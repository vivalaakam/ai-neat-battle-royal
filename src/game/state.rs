use vivalaakam_neuro_neat::Organism;

use super::brain::new_brains;
use super::constants::{DIRECTIONS, KILL_BASE_SCORE, SPAWN_CELL, TREAT_SCORE, TREAT_TILE_DIVISOR};
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
    pub treats: Vec<bool>,
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
        let width = width.max(SPAWN_CELL);
        let height = height.max(SPAWN_CELL);
        let mut game = Self {
            seed: seed.clone(),
            generation,
            width,
            height,
            player_count,
            turn: 0,
            walls: vec![false; width * height],
            treats: vec![false; width * height],
            players: Vec::with_capacity(player_count + 1),
            brains,
        };
        // Map layout is fixed by seed only — generation only reshuffles spawns.
        let mut map_rng = Rng::from_phrase(&seed, 0);
        game.generate_walls(&mut map_rng);
        game.place_treats(&mut map_rng);
        let mut spawn_rng = Rng::from_phrase(&format!("{seed}:spawn"), generation);
        game.spawn_players(&mut spawn_rng);
        game
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn is_wall(&self, x: usize, y: usize) -> bool {
        self.walls[self.index(x, y)]
    }

    pub fn has_treat(&self, x: usize, y: usize) -> bool {
        self.treats[self.index(x, y)]
    }

    fn set_wall(&mut self, x: usize, y: usize) {
        let index = self.index(x, y);
        self.walls[index] = true;
        self.treats[index] = false;
    }

    pub fn spawn_cell_count(width: usize, height: usize) -> usize {
        width.div_ceil(SPAWN_CELL) * height.div_ceil(SPAWN_CELL)
    }

    fn spawn_cells(width: usize, height: usize) -> Vec<(usize, usize, usize, usize)> {
        let mut cells = Vec::new();
        let mut y0 = 0;
        while y0 < height {
            let y1 = (y0 + SPAWN_CELL).min(height);
            let mut x0 = 0;
            while x0 < width {
                let x1 = (x0 + SPAWN_CELL).min(width);
                cells.push((x0, y0, x1, y1));
                x0 += SPAWN_CELL;
            }
            y0 += SPAWN_CELL;
        }
        cells
    }

    fn generate_walls(&mut self, rng: &mut Rng) {
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
    }

    fn place_treats(&mut self, rng: &mut Rng) {
        let free: Vec<_> = (1..self.height - 1)
            .flat_map(|y| (1..self.width - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| !self.is_wall(x, y))
            .collect();
        let treat_count = (free.len() / TREAT_TILE_DIVISOR).max(1);
        let mut pool = free;
        for _ in 0..treat_count.min(pool.len()) {
            let pick = rng.range(pool.len());
            let (x, y) = pool.swap_remove(pick);
            let index = self.index(x, y);
            self.treats[index] = true;
        }
    }

    fn spawn_players(&mut self, rng: &mut Rng) {
        let mut cells = Self::spawn_cells(self.width, self.height);
        assert!(
            cells.len() >= self.player_count,
            "map needs at least {} spawn cells of {}×{}, got {}",
            self.player_count,
            SPAWN_CELL,
            SPAWN_CELL,
            cells.len()
        );
        for id in 1..=self.player_count {
            let cell_pick = rng.range(cells.len());
            let (x0, y0, x1, y1) = cells.swap_remove(cell_pick);
            let mut free: Vec<_> = (y0.max(1)..y1.min(self.height - 1))
                .flat_map(|y| (x0.max(1)..x1.min(self.width - 1)).map(move |x| (x, y)))
                .filter(|&(x, y)| !self.is_wall(x, y))
                .collect();
            assert!(
                !free.is_empty(),
                "spawn cell ({x0},{y0})-({x1},{y1}) has no free tiles"
            );
            let pick = rng.range(free.len());
            let (x, y) = free.swap_remove(pick);
            // Don't spawn on a treat — leave it for exploration.
            let index = self.index(x, y);
            self.treats[index] = false;
            self.players.push(Player {
                id,
                x,
                y,
                direction: rng.range(DIRECTIONS.len()),
                alive: true,
                kills: 0,
                treats: 0,
                tiles_walked: 1,
                born_turn: 0,
                died_turn: None,
            });
        }
    }

    pub fn kill_score(kills: u64) -> u64 {
        // 100 + 200 + 400 + ... = KILL_BASE * (2^kills - 1)
        if kills == 0 {
            return 0;
        }
        let shift = kills.min(63);
        KILL_BASE_SCORE.saturating_mul((1u64 << shift).saturating_sub(1))
    }

    pub fn score(&self, player: &Player) -> u64 {
        let alive_turns = player.died_turn.unwrap_or(self.turn) - player.born_turn;
        alive_turns
            + player.tiles_walked
            + player.treats * TREAT_SCORE
            + Self::kill_score(player.kills)
    }

    pub fn try_collect_treat(&mut self, player_index: usize) {
        let Some(player) = self.players.get(player_index) else {
            return;
        };
        let index = self.index(player.x, player.y);
        if self.treats[index] {
            self.treats[index] = false;
            if let Some(player) = self.players.get_mut(player_index) {
                player.treats += 1;
            }
        }
    }

    pub fn record_step(&mut self, player_index: usize) {
        if let Some(player) = self.players.get_mut(player_index) {
            player.tiles_walked += 1;
        }
        self.try_collect_treat(player_index);
    }

    pub fn alive_agent_count(&self) -> usize {
        self.players.iter().filter(|p| p.alive && p.id > 0).count()
    }
}
