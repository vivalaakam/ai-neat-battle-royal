use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use vivalaakam_neuro_neat::{Genome, Organism};

use super::lineage::{organism_serial, parse_serial};
use super::state::Game;

const MODELS_DIR: &str = "models";

pub fn weights_hash(weights: &[f32]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for weight in weights {
        weight.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn model_path(hash: u64) -> PathBuf {
    PathBuf::from(MODELS_DIR).join(format!("{hash:016x}.json"))
}

#[derive(Deserialize)]
struct SavedModel {
    score: Option<u64>,
    lineage: Option<u64>,
    genome: Genome,
}

pub fn max_lineage_serial() -> u64 {
    let Some(entries) = fs::read_dir(MODELS_DIR).ok() else {
        return 0;
    };
    let mut max = 0u64;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if let Some(lineage) = value.get("lineage").and_then(|v| v.as_u64()) {
            max = max.max(lineage);
        }
    }
    max
}

fn organism_from_file(path: &Path) -> Option<(u64, u64, Organism)> {
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let (score, genome) = if let Ok(saved) = serde_json::from_str::<SavedModel>(&text) {
        (saved.score.unwrap_or(0), saved.genome)
    } else if let Some(genome_value) = value.get("genome") {
        let genome: Genome = serde_json::from_value(genome_value.clone()).ok()?;
        let score = value.get("score").and_then(|s| s.as_u64()).unwrap_or(0);
        (score, genome)
    } else {
        let genome: Genome = serde_json::from_str(&text).ok()?;
        (0, genome)
    };
    let mut organism = Organism::new(genome);
    if let Some(lineage) = value
        .get("lineage")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            value
                .get("lineage")
                .and_then(|v| v.as_str())
                .and_then(parse_serial)
        })
    {
        organism.set_id(lineage.to_string());
    }
    let hash = weights_hash(&organism.genome.to_weights());
    Some((score, hash, organism))
}

/// Loads unique saved genomes from `models/`, highest score first.
pub fn load_all_models() -> Vec<Organism> {
    let entries = fs::read_dir(MODELS_DIR).ok();
    let Some(entries) = entries else {
        return Vec::new();
    };

    let mut loaded = Vec::new();
    let mut seen_hashes = std::collections::HashSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Some((score, hash, organism)) = organism_from_file(&path) else {
            continue;
        };
        if seen_hashes.insert(hash) {
            loaded.push((score, organism));
        }
    }
    loaded.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    loaded.into_iter().map(|(_, org)| org).collect()
}

fn generation_best(game: &Game) -> Option<(&super::player::Player, &Organism)> {
    let best = game
        .players
        .iter()
        .filter(|p| p.id > 0)
        .max_by_key(|p| (game.score(p), std::cmp::Reverse(p.id)))?;
    Some((best, &game.brains[best.id - 1]))
}

pub fn log_generation_best(game: &Game) {
    let Some((best, brain)) = generation_best(game) else {
        return;
    };
    let lineage = organism_serial(brain)
        .map(|serial| serial.to_string())
        .unwrap_or_else(|| "?".into());
    eprintln!(
        "gen {} best: lineage #{lineage}  slot {:02}  score {:>5}  ⚔{:>2} 🍬{:>3} 👟{:>4} {}",
        game.generation,
        best.id,
        game.score(best),
        best.kills,
        best.treats,
        best.tiles_walked,
        if best.alive { "🟢" } else { "🔴" },
    );
}

/// Saves the top-scoring agent's genome if this weight fingerprint is not on disk yet.
pub fn save_best_if_new(game: &Game) {
    let Some((best, brain)) = generation_best(game) else {
        return;
    };

    let weights = brain.genome.to_weights();
    let hash = weights_hash(&weights);
    let path = model_path(hash);
    if path.exists() {
        return;
    }

    if fs::create_dir_all(MODELS_DIR).is_err() {
        return;
    }

    let lineage = organism_serial(brain).unwrap_or(0);
    let payload = format!(
        "{{\n  \"seed\": {:?},\n  \"generation\": {},\n  \"lineage\": {lineage},\n  \"player_id\": {},\n  \"score\": {},\n  \"treats\": {},\n  \"tiles_walked\": {},\n  \"kills\": {},\n  \"weights_hash\": \"{hash:016x}\",\n  \"genome\": {}\n}}",
        game.seed,
        game.generation,
        best.id,
        game.score(best),
        best.treats,
        best.tiles_walked,
        best.kills,
        brain.as_json(),
    );
    let _ = fs::write(path, payload);
}
