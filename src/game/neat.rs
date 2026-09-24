use super::constants::ACTIONS;
use super::state::Game;

impl Game {
    pub fn neat_actions(&self) -> Vec<&'static str> {
        let sensed: Vec<_> = self
            .players
            .iter()
            .filter(|p| p.alive && p.id > 0)
            .map(|p| (p.id, self.observation(p)))
            .collect();
        let mut actions = vec!["wait"; self.player_count];
        for (id, input) in sensed {
            let output = self.brains[id - 1].network.activate(input);
            let action = output
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(index, _)| index)
                .unwrap_or(ACTIONS.len() - 1);
            actions[id - 1] = ACTIONS[action];
        }
        actions
    }

    pub fn apply_neat_turn(&mut self) -> Vec<usize> {
        let actions = self.neat_actions();
        self.apply_turn(&actions)
    }
}
