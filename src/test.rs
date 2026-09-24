use crate::game::{Game, DEFAULT_PLAYER_COUNT, OBSERVATION_SIZE};

pub fn self_test() {
    let mut a = Game::new("replay phrase".into(), 3, 50, 24, DEFAULT_PLAYER_COUNT);
    let b = Game::new("replay phrase".into(), 3, 50, 24, DEFAULT_PLAYER_COUNT);
    assert_eq!(a.walls, b.walls);
    assert_eq!(a.players, b.players);
    assert_eq!(a.brains[0].as_json(), b.brains[0].as_json());
    assert_eq!(a.observation(&a.players[0]).len(), OBSERVATION_SIZE);
    assert!(a.apply_neat_turn().len() <= DEFAULT_PLAYER_COUNT);
    assert!(a.add_human());
    a.turn_human(true);
    assert_eq!(a.players.iter().find(|p| p.id == 0).unwrap().direction, 1);
    println!("self-test passed");
}
