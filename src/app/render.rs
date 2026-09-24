use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::game::{Game, VISION_DEPTH};

use super::components::BoardVisual;
use super::resources::{Battle, TILE};

pub fn point(game: &Game, x: usize, y: usize) -> Vec3 {
    Vec3::new(
        (x as f32 - game.width as f32 / 2.0 + 0.5) * TILE,
        (game.height as f32 / 2.0 - y as f32 - 0.5) * TILE,
        0.0,
    )
}

fn player_color(id: usize) -> Color {
    if id == 0 {
        Color::WHITE
    } else {
        Color::hsl((id * 47 % 360) as f32, 0.8, 0.58)
    }
}

pub fn redraw(
    mut commands: Commands,
    mut battle: ResMut<Battle>,
    visuals: Query<Entity, With<BoardVisual>>,
) {
    if !battle.dirty {
        return;
    }
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let game = &battle.game;
    let mut visible = vec![false; game.width * game.height];
    for player in game.players.iter().filter(|p| p.alive) {
        for (x, y) in game.visible_cells(player, VISION_DEPTH) {
            visible[game.index(x, y)] = true;
        }
    }
    for y in 0..game.height {
        for x in 0..game.width {
            let color = if game.is_wall(x, y) {
                Some(Color::srgb(0.23, 0.25, 0.29))
            } else if game.has_treat(x, y) {
                Some(Color::srgb(0.95, 0.72, 0.18))
            } else if visible[game.index(x, y)] {
                Some(Color::srgb(0.09, 0.12, 0.16))
            } else {
                None
            };
            if let Some(color) = color {
                commands.spawn((
                    Sprite::from_color(color, Vec2::splat(TILE - 1.0)),
                    Transform::from_translation(point(game, x, y)),
                    RenderLayers::layer(1),
                    BoardVisual,
                ));
            }
        }
    }
    for player in game.players.iter().filter(|p| p.alive) {
        let position = point(game, player.x, player.y);
        commands.spawn((
            Sprite::from_color(player_color(player.id), Vec2::splat(TILE - 3.0)),
            Transform::from_translation(position + Vec3::Z),
            RenderLayers::layer(1),
            BoardVisual,
        ));
        commands.spawn((
            Text2d::new(format!("{:02}", player.id)),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::BLACK),
            Transform::from_translation(position + Vec3::Z * 2.0),
            RenderLayers::layer(1),
            BoardVisual,
        ));
    }
    battle.dirty = false;
}
