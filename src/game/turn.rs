use super::constants::{DIRECTIONS, VISION_DEPTH};
use super::state::Game;

impl Game {
    // Actions are collected first, then resolved together: forward, left, right, shoot, wait.
    pub fn apply_turn(&mut self, actions: &[&str]) -> Vec<usize> {
        let mut occupied = vec![false; self.width * self.height];
        for p in self.players.iter().filter(|p| p.alive) {
            occupied[self.index(p.x, p.y)] = true;
        }
        let mut proposed = vec![None; self.player_count];
        let width = self.width;
        let walls = &self.walls;
        for player in &mut self.players {
            if !player.alive || player.id == 0 {
                continue;
            }
            match actions.get(player.id - 1).copied().unwrap_or("wait") {
                "left" => {
                    player.direction = (player.direction + DIRECTIONS.len() - 1) % DIRECTIONS.len()
                }
                "right" => player.direction = (player.direction + 1) % DIRECTIONS.len(),
                "forward" => {
                    let (dx, dy) = DIRECTIONS[player.direction];
                    let x = player.x.saturating_add_signed(dx);
                    let y = player.y.saturating_add_signed(dy);
                    if !walls[y * width + x] {
                        proposed[player.id - 1] = Some((x, y));
                    }
                }
                _ => {}
            }
        }
        for id in 0..self.player_count {
            if let Some(target) = proposed[id]
                && proposed
                    .iter()
                    .filter(|&&other| other == Some(target))
                    .count()
                    == 1
                && !occupied[self.index(target.0, target.1)]
            {
                self.players[id].x = target.0;
                self.players[id].y = target.1;
                self.record_step(id);
            }
        }
        let mut hit = vec![false; self.player_count];
        let mut kills = vec![0; self.player_count];
        for player in self.players.iter().filter(|p| p.alive && p.id > 0) {
            if actions.get(player.id - 1).copied() == Some("shoot") {
                let visible = self.visible_cells(player, VISION_DEPTH);
                for other in self
                    .players
                    .iter()
                    .filter(|p| p.alive && p.id > 0 && p.id != player.id)
                {
                    if visible.contains(&(other.x, other.y)) {
                        hit[other.id - 1] = true;
                        kills[player.id - 1] += 1;
                    }
                }
            }
        }
        let hit_ids: Vec<_> = hit
            .iter()
            .enumerate()
            .filter_map(|(i, &was_hit)| was_hit.then_some(i + 1))
            .collect();
        for id in &hit_ids {
            self.players[*id - 1].alive = false;
            self.players[*id - 1].died_turn = Some(self.turn + 1);
        }
        for (index, count) in kills.into_iter().enumerate() {
            self.players[index].kills += count;
        }
        self.turn += 1;
        hit_ids
    }
}
