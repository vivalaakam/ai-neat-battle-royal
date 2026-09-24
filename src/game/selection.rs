use vivalaakam_neuro_neat::Organism;

use super::brain::{breed_next_generation, new_brains};
use super::state::Game;

pub fn map_capacity(width: usize, height: usize) -> usize {
    Game::spawn_cell_count(width, height)
}

fn trial_map_seed(seed: &str) -> String {
    format!("{seed}:best-trial")
}

fn run_group_trial(
    map_seed: &str,
    width: usize,
    height: usize,
    group: Vec<Organism>,
    max_turns: u64,
) -> Vec<(Organism, u64)> {
    let count = group.len();
    if count == 0 {
        return Vec::new();
    }
    let mut game = Game::with_brains(
        map_seed.to_string(),
        0,
        width,
        height,
        count,
        group,
    );
    for _ in 0..max_turns {
        if game.alive_agent_count() <= 1 {
            break;
        }
        game.apply_neat_turn();
    }
    (1..=count)
        .filter_map(|id| {
            let player = game.players.iter().find(|p| p.id == id)?;
            Some((game.brains[id - 1].clone(), game.score(player)))
        })
        .collect()
}

fn pad_survivors(
    seed: &str,
    generation: u64,
    target: usize,
    mut selected: Vec<Organism>,
    lineage_serial: &mut u64,
) -> Vec<Organism> {
    if selected.is_empty() {
        return new_brains(seed, generation, target, Some(lineage_serial));
    }
    while selected.len() < target {
        let extra = breed_next_generation(seed, generation, 1, &selected, lineage_serial);
        selected.extend(extra);
    }
    selected.truncate(target);
    selected
}

/// Splits contenders into map-sized groups, runs each on the same trial map, returns top `survivors`.
pub fn tournament_select(
    seed: &str,
    width: usize,
    height: usize,
    survivors: usize,
    max_turns: u64,
    contenders: Vec<Organism>,
    lineage_serial: &mut u64,
) -> (Vec<Organism>, usize) {
    if contenders.is_empty() {
        return (
            new_brains(seed, 0, survivors, Some(lineage_serial)),
            0,
        );
    }

    let map_seed = trial_map_seed(seed);
    let group_size = map_capacity(width, height).max(1);
    let group_count = contenders.len().div_ceil(group_size);

    let mut scored = Vec::with_capacity(contenders.len());
    for (index, chunk) in contenders.chunks(group_size).enumerate() {
        let group = chunk.to_vec();
        eprintln!(
            "--best: group {}/{} — {} contenders on trial map",
            index + 1,
            group_count,
            group.len()
        );
        scored.extend(run_group_trial(
            &map_seed,
            width,
            height,
            group,
            max_turns,
        ));
    }

    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    let picked: Vec<Organism> = scored
        .into_iter()
        .take(survivors)
        .map(|(organism, _)| organism)
        .collect();

    (
        pad_survivors(seed, 0, survivors, picked, lineage_serial),
        group_count,
    )
}
