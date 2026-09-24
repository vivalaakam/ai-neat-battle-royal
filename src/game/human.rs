use super::constants::{DIRECTIONS, VISION_DEPTH};
use super::state::Game;

impl Game {
    pub fn add_human(&mut self) -> bool {
        if self.players.iter().any(|p| p.id == 0) {
            return false;
        }
        let occupied: std::collections::HashSet<_> =
            self.players.iter().map(|p| (p.x, p.y)).collect();
        let (x, y) = (1..self.height - 1)
            .flat_map(|y| (1..self.width - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| !self.is_wall(x, y) && !occupied.contains(&(x, y)))
            .min_by_key(|&(x, y)| x.abs_diff(self.width / 2) + y.abs_diff(self.height / 2))
            .expect("map has no free cell");
        self.players.push(super::player::Player {
            id: 0,
            x,
            y,
            direction: 0,
            alive: true,
            kills: 0,
            tiles_walked: 1,
            born_turn: self.turn,
            died_turn: None,
        });
        true
    }

    pub fn move_human(&mut self, forward: isize, side: isize) {
        let Some(index) = self.players.iter().position(|p| p.id == 0 && p.alive) else {
            return;
        };
        let (dx, dy) = DIRECTIONS[self.players[index].direction];
        let (side_x, side_y) = (-dy, dx);
        let target = (
            self.players[index]
                .x
                .saturating_add_signed(dx * forward + side_x * side),
            self.players[index]
                .y
                .saturating_add_signed(dy * forward + side_y * side),
        );
        if !self.is_wall(target.0, target.1)
            && !self
                .players
                .iter()
                .any(|p| p.alive && (p.x, p.y) == target)
        {
            self.players[index].x = target.0;
            self.players[index].y = target.1;
            self.record_step(index);
        }
        self.turn += 1;
    }

    pub fn turn_human(&mut self, clockwise: bool) {
        if let Some(human) = self.players.iter_mut().find(|p| p.id == 0 && p.alive) {
            human.direction = (human.direction + if clockwise { 1 } else { DIRECTIONS.len() - 1 })
                % DIRECTIONS.len();
            self.turn += 1;
        }
    }

    pub fn shoot_human(&mut self) {
        let Some(shooter) = self.players.iter().find(|p| p.id == 0 && p.alive).cloned() else {
            return;
        };
        let visible = self.visible_cells(&shooter, VISION_DEPTH);
        let victims: Vec<_> = self
            .players
            .iter()
            .filter(|p| p.alive && p.id > 0 && visible.contains(&(p.x, p.y)))
            .map(|p| p.id)
            .collect();
        for id in &victims {
            self.players[*id - 1].alive = false;
            self.players[*id - 1].died_turn = Some(self.turn + 1);
        }
        if let Some(human) = self.players.iter_mut().find(|p| p.id == 0) {
            human.kills += victims.len() as u64;
        }
        self.turn += 1;
    }
}
