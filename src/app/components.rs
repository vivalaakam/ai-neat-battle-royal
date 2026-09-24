use bevy::prelude::*;

#[derive(Component)]
pub struct BoardVisual;

#[derive(Component)]
pub struct BoardCamera;

/// Scroll body that owns per-player leaderboard rows.
#[derive(Component)]
pub struct LeaderboardList;

#[derive(Component)]
pub struct LeaderboardRow;

#[derive(Component)]
pub struct StatusText;
