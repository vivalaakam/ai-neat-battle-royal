use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::components::{BoardCamera, LeaderboardText, StatusText};
use super::resources::LEADERBOARD_PANEL_PX;

pub fn setup(mut commands: Commands) {
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

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: px(0),
            width: px(LEADERBOARD_PANEL_PX),
            height: percent(100),
            padding: UiRect::all(px(12)),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.04, 0.07, 0.94)),
        children![
            (
                Text::new("Leaderboard"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.45)),
            ),
            (
                Text::new("score = survival + kills×10 + tiles walked"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.58, 0.62)),
            ),
            (
                LeaderboardText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.89, 0.92)),
                TextLayout::linebreak(LineBreak::NoWrap),
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
