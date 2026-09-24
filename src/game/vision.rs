use super::constants::VISION_DEPTH;
use super::player::Player;
use super::state::Game;

impl Game {
    pub fn visible_cells(&self, player: &Player, depth: usize) -> Vec<(usize, usize)> {
        let (dx, dy) = super::constants::DIRECTIONS[player.direction];
        let mut cells = Vec::new();
        for y in player.y.saturating_sub(depth)..=(player.y + depth).min(self.height - 1) {
            for x in player.x.saturating_sub(depth)..=(player.x + depth).min(self.width - 1) {
                let (relative_x, relative_y) = (
                    x as isize - player.x as isize,
                    y as isize - player.y as isize,
                );
                let dot = relative_x * dx + relative_y * dy;
                let cross = relative_x * dy - relative_y * dx;
                if dot > 0
                    && cross.abs() <= dot
                    && self.has_line_of_sight((player.x, player.y), (x, y))
                {
                    cells.push((x, y));
                }
            }
        }
        cells
    }

    fn has_line_of_sight(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        let (mut x, mut y) = (from.0 as isize, from.1 as isize);
        let (end_x, end_y) = (to.0 as isize, to.1 as isize);
        let (dx, dy) = ((end_x - x).abs(), -(end_y - y).abs());
        let (step_x, step_y) = ((end_x - x).signum(), (end_y - y).signum());
        let mut error = dx + dy;
        while (x, y) != (end_x, end_y) {
            let twice_error = error * 2;
            if twice_error >= dy {
                error += dy;
                x += step_x;
            }
            if twice_error <= dx {
                error += dx;
                y += step_y;
            }
            if (x, y) != (end_x, end_y) && self.is_wall(x as usize, y as usize) {
                return false;
            }
        }
        true
    }

    pub fn observation(&self, player: &Player) -> Vec<f32> {
        use super::constants::{DIRECTIONS, OBSERVATION_SIZE};

        let mut input = Vec::with_capacity(OBSERVATION_SIZE);
        input.push(player.x as f32 / (self.width - 1) as f32 * 2.0 - 1.0);
        input.push(player.y as f32 / (self.height - 1) as f32 * 2.0 - 1.0);
        input.extend(
            (0..DIRECTIONS.len()).map(|direction| (direction == player.direction) as u8 as f32),
        );
        let mut visible = vec![false; self.width * self.height];
        for (x, y) in self.visible_cells(player, VISION_DEPTH) {
            visible[self.index(x, y)] = true;
        }
        for relative_y in -(VISION_DEPTH as isize)..=VISION_DEPTH as isize {
            for relative_x in -(VISION_DEPTH as isize)..=VISION_DEPTH as isize {
                let x = player.x as isize + relative_x;
                let y = player.y as isize + relative_y;
                input.push(
                    if x < 0
                        || y < 0
                        || x >= self.width as isize
                        || y >= self.height as isize
                        || !visible[self.index(x as usize, y as usize)]
                    {
                        -1.0
                    } else if self.is_wall(x as usize, y as usize) {
                        -0.5
                    } else if self.players.iter().any(|p| {
                        p.alive && p.id != player.id && p.x == x as usize && p.y == y as usize
                    }) {
                        1.0
                    } else if self.has_treat(x as usize, y as usize) {
                        0.5
                    } else {
                        0.0
                    },
                );
            }
        }
        input
    }
}
