use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::components::{BoardCamera, LeaderboardList, StatusText};
use super::emoji_font::EmojiFont;
use super::resources::LEADERBOARD_PANEL_PX;

pub fn setup(mut commands: Commands, emoji: Res<EmojiFont>) {
    commands.spawn((
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
    commands.spawn((
        Camera2d,
        BoardCamera,
        Camera {
            order: 0,
            ..default()
        },
        RenderLayers::layer(1),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));

    let emoji_face = TextFont {
        font: emoji.0.clone().into(),
        font_size: FontSize::Px(10.0),
        ..default()
    };

    // Default FiraMono for digits/ASCII (narrow). Emoji glyphs fall back to Noto Emoji
    // via FontCx — using Noto Emoji as the primary face blows up advance width + line height.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: px(0),
            width: px(LEADERBOARD_PANEL_PX),
            height: percent(100),
            padding: UiRect::axes(px(10), px(8)),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.04, 0.07, 0.94)),
        children![
            (
                Text::new("Leaderboard"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.45)),
            ),
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.58, 0.62)),
                children![
                    (
                        TextSpan::new("score = "),
                        TextColor(Color::srgb(0.55, 0.58, 0.62)),
                    ),
                    (
                        TextSpan::new("⚫"),
                        emoji_face.clone(),
                        TextColor(Color::srgb(0.2, 0.9, 0.35)),
                    ),
                    (
                        TextSpan::new("/"),
                        TextColor(Color::srgb(0.55, 0.58, 0.62)),
                    ),
                    (
                        TextSpan::new("⚫"),
                        emoji_face,
                        TextColor(Color::srgb(0.95, 0.25, 0.25)),
                    ),
                    (
                        TextSpan::new(" + 👟 + 🍬×10 + ⚔100×2ⁿ"),
                        TextColor(Color::srgb(0.55, 0.58, 0.62)),
                    ),
                ],
            ),
            (
                LeaderboardList,
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ),
        ],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(10),
            right: px(LEADERBOARD_PANEL_PX + 12.0),
            ..default()
        },
        children![(
            StatusText,
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));
}
