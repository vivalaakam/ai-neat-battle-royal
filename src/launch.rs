use std::env;

use crate::game::models::load_all_models;
use crate::game::{tournament_select, DEFAULT_PLAYER_COUNT, Game};
use crate::test::self_test;

#[derive(Clone)]
pub struct Launch {
    pub seed: String,
    pub width: usize,
    pub height: usize,
    pub players: usize,
    pub max_step: Option<u64>,
    pub max_generation: Option<u64>,
    pub load_best: bool,
}

impl Launch {
    pub fn parse() -> Self {
        let mut launch = Self {
            seed: "battle-royal".into(),
            width: 50,
            height: 30,
            players: DEFAULT_PLAYER_COUNT,
            max_step: None,
            max_generation: None,
            load_best: false,
        };
        let args: Vec<_> = env::args().skip(1).collect();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--self-test" => {
                    self_test();
                    std::process::exit(0);
                }
                "--width" => launch.width = cli_value(&args, &mut index, "--width"),
                "--height" => launch.height = cli_value(&args, &mut index, "--height"),
                "--players" => launch.players = cli_value(&args, &mut index, "--players"),
                "--max-step" => launch.max_step = Some(cli_value_u64(&args, &mut index, "--max-step")),
                "--max-gen" => {
                    launch.max_generation = Some(cli_value_u64(&args, &mut index, "--max-gen"));
                }
                "--best" => launch.load_best = true,
                seed if !seed.starts_with('-') => launch.seed = seed.into(),
                option => panic!("unknown option: {option}"),
            }
            index += 1;
        }
        launch
    }

    pub fn validate(&self) {
        assert!(
            self.width >= 8
                && self.height >= 8
                && self.players > 0
                && self.players <= (self.width - 2) * (self.height - 2) / 2,
            "invalid map size or player count"
        );
    }

    pub fn initial_state(&self, lineage_serial: &mut u64) -> (Game, bool) {
        if !self.load_best {
            return (
                Game::new(
                    self.seed.clone(),
                    0,
                    self.width,
                    self.height,
                    self.players,
                ),
                false,
            );
        }

        let contenders = load_all_models();
        if contenders.is_empty() {
            eprintln!("--best: no models in models/, using random population");
            return (
                Game::new(
                    self.seed.clone(),
                    0,
                    self.width,
                    self.height,
                    self.players,
                ),
                false,
            );
        }

        let max_turns = self.max_step.unwrap_or(20_000);
        eprintln!(
            "--best: {} contenders, selecting top {} (trial limit {max_turns} turns/group)",
            contenders.len(),
            self.players
        );
        let (winners, groups) = tournament_select(
            &self.seed,
            self.width,
            self.height,
            self.players,
            max_turns,
            contenders,
            lineage_serial,
        );
        eprintln!("--best: tournament done ({groups} groups) → {} survivors", winners.len());

        (
            Game::with_brains(
                self.seed.clone(),
                0,
                self.width,
                self.height,
                self.players,
                winners,
            ),
            true,
        )
    }
}

fn cli_value(args: &[String], index: &mut usize, name: &str) -> usize {
    *index += 1;
    args.get(*index)
        .unwrap_or_else(|| panic!("{name} needs a value"))
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be a positive integer"))
}

fn cli_value_u64(args: &[String], index: &mut usize, name: &str) -> u64 {
    *index += 1;
    args.get(*index)
        .unwrap_or_else(|| panic!("{name} needs a value"))
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be a positive integer"))
}
