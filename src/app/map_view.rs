use bevy::camera::Viewport;
use bevy::prelude::*;

use crate::game::Game;

use super::components::BoardCamera;
use super::resources::{
    Battle, MapView, LEADERBOARD_PANEL_PX, MAP_PAN_SPEED, MAP_ZOOM_FACTOR, MAP_ZOOM_MAX,
    MAP_ZOOM_MIN, TILE,
};

pub fn map_half_extents(game: &Game) -> Vec2 {
    Vec2::new(
        game.width as f32 * TILE * 0.5,
        game.height as f32 * TILE * 0.5,
    )
}

fn map_view_logical_size(window: &Window) -> Vec2 {
    Vec2::new(
        (window.width() - LEADERBOARD_PANEL_PX).max(1.0),
        window.height(),
    )
}

fn view_half_extents(window: &Window, zoom: f32) -> Vec2 {
    let size = map_view_logical_size(window);
    Vec2::new(size.x * 0.5 * zoom, size.y * 0.5 * zoom)
}

fn board_viewport(window: &Window) -> Viewport {
    let scale = window.scale_factor();
    let full = window.physical_size();
    let panel = (LEADERBOARD_PANEL_PX * scale).round() as u32;
    Viewport {
        physical_position: UVec2::ZERO,
        physical_size: UVec2::new(full.x.saturating_sub(panel).max(1), full.y),
        ..default()
    }
}

fn clamp_map_pan(pan: Vec2, game: &Game, window: &Window, zoom: f32) -> Vec2 {
    let map_half = map_half_extents(game);
    let view_half = view_half_extents(window, zoom);
    let mut pan = pan;
    if map_half.x * 2.0 <= view_half.x * 2.0 {
        pan.x = 0.0;
    } else {
        pan.x = pan.x.clamp(-map_half.x + view_half.x, map_half.x - view_half.x);
    }
    if map_half.y * 2.0 <= view_half.y * 2.0 {
        pan.y = 0.0;
    } else {
        pan.y = pan.y.clamp(-map_half.y + view_half.y, map_half.y - view_half.y);
    }
    pan
}

pub fn handle_map_view(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut view: ResMut<MapView>) {
    let dt = time.delta_secs();
    let step = MAP_PAN_SPEED * view.zoom * dt;
    if keys.pressed(KeyCode::ArrowLeft) {
        view.pan.x -= step;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        view.pan.x += step;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        view.pan.y += step;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        view.pan.y -= step;
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        view.zoom = (view.zoom * MAP_ZOOM_FACTOR).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        view.zoom = (view.zoom / MAP_ZOOM_FACTOR).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }
}

pub fn apply_map_camera(
    battle: Res<Battle>,
    window: Single<&Window>,
    mut view: ResMut<MapView>,
    mut cameras: Query<(&mut Transform, &mut Projection, &mut Camera), With<BoardCamera>>,
) {
    view.pan = clamp_map_pan(view.pan, &battle.game, &window, view.zoom);
    let Ok((mut transform, mut projection, mut camera)) = cameras.single_mut() else {
        return;
    };
    camera.viewport = Some(board_viewport(&window));
    transform.translation = view.pan.extend(999.0);
    if let Projection::Orthographic(ortho) = &mut *projection {
        ortho.scale = view.zoom;
    }
}
