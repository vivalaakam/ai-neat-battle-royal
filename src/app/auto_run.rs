use bevy::prelude::*;

use super::input::advance_generation;
use super::resources::{AutoRun, Battle, AUTO_TURN_INTERVAL_SECS};

pub fn auto_run_epochs(
    time: Res<Time>,
    mut battle: ResMut<Battle>,
    mut auto: ResMut<AutoRun>,
    mut exit: MessageWriter<AppExit>,
    mut timer: Local<Option<Timer>>,
) {
    let Some(max_step) = auto.max_step else {
        return;
    };
    if !auto.armed || !auto.running || battle.paused {
        return;
    }

    if timer.is_none() {
        *timer = Some(Timer::from_seconds(AUTO_TURN_INTERVAL_SECS, TimerMode::Repeating));
    }
    let tick = timer.as_mut().unwrap();
    tick.tick(time.delta());
    if !tick.is_finished() {
        return;
    }

    battle.game.apply_neat_turn();
    auto.turns_this_generation += 1;
    battle.dirty = true;

    let generation_done =
        auto.turns_this_generation >= max_step || battle.game.alive_agent_count() <= 1;
    if generation_done {
        auto.turns_this_generation = 0;
        advance_generation(&mut battle, &mut exit);
    }
}
