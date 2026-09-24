use bevy::prelude::*;

use crate::launch::Launch;

use super::auto_run::auto_run_epochs;
use super::emoji_font::{configure_emoji_font, load_emoji_font};
use super::hud::update_hud;
use super::input::{handle_input, handle_window_close};
use super::map_view::{apply_map_camera, handle_map_view};
use super::render::redraw;
use super::resources::{AutoRun, Battle, MapView};
use super::setup::setup;

pub struct BattlePlugin {
    launch: Launch,
}

impl BattlePlugin {
    pub fn new(launch: Launch) -> Self {
        Self { launch }
    }
}

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        let mut lineage_serial = crate::game::max_lineage_serial().saturating_add(1);
        if lineage_serial == 0 {
            lineage_serial = 1;
        }
        let (game, breed_lineage) = self.launch.initial_state(&mut lineage_serial);
        app.insert_resource(ClearColor(Color::srgb(0.015, 0.02, 0.03)))
            .insert_resource(Battle {
                game,
                seed: self.launch.seed.clone(),
                generation: 0,
                paused: false,
                dirty: true,
                max_generation: self.launch.max_generation,
                breed_lineage,
                lineage_serial,
            })
            .insert_resource(MapView {
                pan: Vec2::ZERO,
                zoom: 1.0,
            })
            .insert_resource(AutoRun::new(self.launch.max_step))
            .add_systems(Startup, (load_emoji_font, setup).chain())
            .add_systems(
                Update,
                (
                    configure_emoji_font,
                    handle_window_close,
                    handle_map_view,
                    apply_map_camera,
                    handle_input,
                    auto_run_epochs,
                    update_hud,
                    redraw,
                )
                    .chain(),
            );
    }
}
