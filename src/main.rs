mod app;
mod game;
mod launch;
mod test;

use bevy::prelude::*;
use launch::Launch;

fn main() {
    let launch = Launch::parse();
    launch.validate();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AI NEAT Battle Royal".into(),
                resolution: (1200, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(app::BattlePlugin::new(launch))
        .run();
}
