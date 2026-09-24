use bevy::prelude::*;

use crate::game::organism_serial;

use super::components::{LeaderboardText, StatusText};
use super::resources::{AutoRun, Battle};

pub fn update_hud(
    battle: Res<Battle>,
    auto: Res<AutoRun>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LeaderboardText>>,
        Query<&mut Text, With<StatusText>>,
    )>,
) {
    if !battle.dirty {
        return;
    }
    let game = &battle.game;
    let mut players: Vec<_> = game.players.iter().collect();
    players.sort_by_key(|p| (std::cmp::Reverse(game.score(p)), p.id));
    let board = players
        .into_iter()
        .map(|p| {
            let tag = if p.id > 0 {
                organism_serial(&game.brains[p.id - 1])
                    .map(|serial| format!("#{serial}"))
                    .unwrap_or_else(|| format!("#{:02}", p.id))
            } else {
                "#00".into()
            };
            format!(
                "{tag}  score {:>5}  turns {:>4}  kills {:>2}  walk {:>4}  {}",
                game.score(p),
                p.died_turn.unwrap_or(game.turn) - p.born_turn,
                p.kills,
                p.tiles_walked,
                if p.alive { "alive" } else { "dead" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if let Ok(mut text) = texts.p0().single_mut() {
        **text = board;
    }

    let pause = if battle.paused { "  PAUSED" } else { "" };
    let auto_hint = if auto.configured() {
        if !auto.armed {
            "  [Enter → start auto epochs]"
        } else if auto.running {
            "  [auto epochs]"
        } else {
            "  [Enter → resume auto]"
        }
    } else {
        ""
    };
    let gen_limit = battle
        .max_generation
        .map(|limit| format!(" / {limit}"))
        .unwrap_or_default();
    let status_line = format!(
        "seed {:?}   gen {}{gen_limit}   turn #{}   map {}×{}   agents {}/{}{pause}{auto_hint}\n\
Enter AI   ` 00   Q/E turn   WASD move   Space shoot   Esc pause   −/= zoom   arrows pan   Ctrl+W quit",
        game.seed,
        game.generation,
        game.turn,
        game.width,
        game.height,
        game.alive_agent_count(),
        game.player_count,
    );
    if let Ok(mut text) = texts.p1().single_mut() {
        **text = status_line;
    }
}
