use bevy::prelude::*;
use bevy::window::WindowCloseRequested;

use crate::game::{breed_next_generation, log_generation_best, save_best_if_new, Game};

use super::resources::{AutoRun, Battle};

fn modifier_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
}

pub fn handle_window_close(
    mut close: MessageReader<WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    for _ in close.read() {
        exit.write(AppExit::Success);
    }
}

pub fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut battle: ResMut<Battle>,
    mut auto: ResMut<AutoRun>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::KeyW) && modifier_held(&keys) {
        exit.write(AppExit::Success);
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        battle.paused = !battle.paused;
        battle.dirty = true;
        return;
    }
    if battle.paused || modifier_held(&keys) {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) {
        if auto.configured() {
            if !auto.armed {
                auto.armed = true;
                auto.running = true;
                auto.turns_this_generation = 0;
                battle.dirty = true;
                return;
            }
            if !auto.running {
                auto.running = true;
                battle.dirty = true;
                return;
            }
            auto.running = false;
            battle.dirty = true;
            return;
        }
        battle.game.apply_neat_turn();
    }

    if keys.just_pressed(KeyCode::Backquote) {
        battle.game.add_human();
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        battle.game.turn_human(false);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        battle.game.turn_human(true);
    }
    if keys.just_pressed(KeyCode::KeyW) {
        battle.game.move_human(1, 0);
    }
    if keys.just_pressed(KeyCode::KeyS) {
        battle.game.move_human(-1, 0);
    }
    if keys.just_pressed(KeyCode::KeyA) {
        battle.game.move_human(0, -1);
    }
    if keys.just_pressed(KeyCode::KeyD) {
        battle.game.move_human(0, 1);
    }
    if keys.just_pressed(KeyCode::Space) {
        battle.game.shoot_human();
    }
    if keys.just_pressed(KeyCode::KeyR) {
        advance_generation(&mut battle, &mut exit);
        auto.turns_this_generation = 0;
    }
    battle.dirty = true;
}

pub fn advance_generation(battle: &mut Battle, exit: &mut MessageWriter<AppExit>) {
    log_generation_best(&battle.game);
    save_best_if_new(&battle.game);
    battle.generation += 1;
    if battle
        .max_generation
        .is_some_and(|limit| battle.generation >= limit)
    {
        exit.write(AppExit::Success);
        return;
    }
    let width = battle.game.width;
    let height = battle.game.height;
    let player_count = battle.game.player_count;
    battle.game = if battle.breed_lineage {
        let parents = battle.game.brains.clone();
        let brains = breed_next_generation(
            &battle.seed,
            battle.generation,
            player_count,
            &parents,
            &mut battle.lineage_serial,
        );
        Game::with_brains(
            battle.seed.clone(),
            battle.generation,
            width,
            height,
            player_count,
            brains,
        )
    } else {
        Game::new(
            battle.seed.clone(),
            battle.generation,
            width,
            height,
            player_count,
        )
    };
}
