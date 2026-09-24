use vivalaakam_neuro_neat::{Config, Genome, Organism};

use super::constants::{ACTIONS, OBSERVATION_SIZE};
use super::lineage::assign_serial;
use super::rng::Rng;

pub fn new_brains(
    seed: &str,
    generation: u64,
    player_count: usize,
    lineage_serial: Option<&mut u64>,
) -> Vec<Organism> {
    let config = Config::default();
    let mut brains = Vec::with_capacity(player_count);
    let build = |id: usize, counter: Option<&mut u64>| {
        let genome =
            Genome::generate_genome(OBSERVATION_SIZE, ACTIONS.len(), Vec::new(), None, &config)
                .expect("NEAT genome generation failed");
        let mut weights = genome.to_weights();
        let mut rng = Rng::from_phrase(&format!("{seed}:agent:{id}"), generation);
        let node_count = OBSERVATION_SIZE + ACTIONS.len();
        for node in 0..node_count {
            weights[6 + node * 4 + 1] = rng.next() as f32 / u64::MAX as f32 * 2.0 - 1.0;
        }
        for connection in 0..OBSERVATION_SIZE * ACTIONS.len() {
            weights[6 + node_count * 4 + connection * 4 + 2] =
                rng.next() as f32 / u64::MAX as f32 * 2.0 - 1.0;
        }
        let mut organism = Organism::new(Genome::from_weights(weights));
        if let Some(next) = counter {
            assign_serial(&mut organism, next);
        }
        organism
    };
    if let Some(counter) = lineage_serial {
        for id in 0..player_count {
            brains.push(build(id, Some(counter)));
        }
    } else {
        for id in 0..player_count {
            brains.push(build(id, None));
        }
    }
    brains
}

pub fn breed_next_generation(
    seed: &str,
    generation: u64,
    player_count: usize,
    parents: &[Organism],
    lineage_serial: &mut u64,
) -> Vec<Organism> {
    if parents.is_empty() {
        return new_brains(
            seed,
            generation,
            player_count,
            Some(lineage_serial),
        );
    }
    let config = Config::default();
    let mut rng = Rng::from_phrase(&format!("{seed}:breed"), generation);
    let mut brains = Vec::with_capacity(player_count);
    for _ in 0..player_count {
        let parent_a = &parents[rng.range(parents.len())];
        let parent_b = &parents[rng.range(parents.len())];
        let genome = parent_a
            .genome
            .mutate(Some(&parent_b.genome), &config)
            .unwrap_or_else(|_| parent_a.genome.clone());
        let mut organism = Organism::new(genome);
        assign_serial(&mut organism, lineage_serial);
        brains.push(organism);
    }
    brains
}
