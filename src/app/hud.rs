use bevy::prelude::*;
use bevy::text::LineHeight;

use crate::game::organism_serial;

use super::components::{LeaderboardList, LeaderboardRow, StatusText};
use super::emoji_font::EmojiFont;
use super::resources::{AutoRun, Battle};

const ALIVE_DOT: Color = Color::srgb(0.2, 0.9, 0.35);
const DEAD_DOT: Color = Color::srgb(0.95, 0.25, 0.25);
const ROW_COLOR: Color = Color::srgb(0.86, 0.89, 0.92);

pub fn update_hud(
    mut commands: Commands,
    battle: Res<Battle>,
    auto: Res<AutoRun>,
    emoji: Res<EmojiFont>,
    list: Query<Entity, With<LeaderboardList>>,
    rows: Query<Entity, With<LeaderboardRow>>,
    mut status: Query<&mut Text, With<StatusText>>,
) {
    if !battle.dirty {
        return;
    }

    for entity in &rows {
        commands.entity(entity).despawn();
    }

    let Ok(list_entity) = list.single() else {
        return;
    };

    let game = &battle.game;
    let mut players: Vec<_> = game.players.iter().collect();
    players.sort_by_key(|p| (std::cmp::Reverse(game.score(p)), p.id));

    let emoji_font = emoji.0.clone();
    for p in players {
        let tag = if p.id > 0 {
            organism_serial(&game.brains[p.id - 1])
                .map(|serial| format!("#{serial}"))
                .unwrap_or_else(|| format!("#{:02}", p.id))
        } else {
            "#00".into()
        };
        let tag = if tag.chars().count() > 6 {
            format!("{}…", tag.chars().take(5).collect::<String>())
        } else {
            format!("{tag:<6}")
        };
        let body = format!(
            "{tag}{:>5} ⚔{:>2} 🍬{:>3} 👟{:>4} ",
            game.score(p),
            p.kills,
            p.treats,
            p.tiles_walked,
        );
        let dot_color = if p.alive { ALIVE_DOT } else { DEAD_DOT };

        commands.entity(list_entity).with_children(|parent| {
            parent.spawn((
                LeaderboardRow,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                LineHeight::Px(14.0),
                TextColor(ROW_COLOR),
                TextLayout::linebreak(LineBreak::NoWrap),
                children![
                    (TextSpan::new(body), TextColor(ROW_COLOR),),
                    (
                        // Monochrome ⚫ tinted green/red — color emoji fonts panic swash.
                        TextSpan::new("⚫"),
                        TextFont {
                            font: emoji_font.clone().into(),
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(dot_color),
                    ),
                ],
            ));
        });
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
    if let Ok(mut text) = status.single_mut() {
        **text = status_line;
    }
}
