use crate::game::{
    Game, DEFAULT_MAP_HEIGHT, DEFAULT_MAP_WIDTH, DEFAULT_PLAYER_COUNT, OBSERVATION_SIZE,
};

pub fn self_test() {
    let mut a = Game::new(
        "replay phrase".into(),
        3,
        DEFAULT_MAP_WIDTH,
        DEFAULT_MAP_HEIGHT,
        DEFAULT_PLAYER_COUNT,
    );
    let b = Game::new(
        "replay phrase".into(),
        3,
        DEFAULT_MAP_WIDTH,
        DEFAULT_MAP_HEIGHT,
        DEFAULT_PLAYER_COUNT,
    );
    assert_eq!(a.walls, b.walls);
    assert_eq!(a.treats, b.treats);
    assert_eq!(a.players, b.players);
    assert_eq!(a.brains[0].as_json(), b.brains[0].as_json());

    // Same seed → same walls across generations; spawns differ.
    let c = Game::new(
        "replay phrase".into(),
        0,
        DEFAULT_MAP_WIDTH,
        DEFAULT_MAP_HEIGHT,
        DEFAULT_PLAYER_COUNT,
    );
    let d = Game::new(
        "replay phrase".into(),
        7,
        DEFAULT_MAP_WIDTH,
        DEFAULT_MAP_HEIGHT,
        DEFAULT_PLAYER_COUNT,
    );
    assert_eq!(c.walls, d.walls);
    assert_ne!(
        c.players
            .iter()
            .map(|p| (p.x, p.y))
            .collect::<Vec<_>>(),
        d.players
            .iter()
            .map(|p| (p.x, p.y))
            .collect::<Vec<_>>()
    );

    // One agent per 16×16 spawn cell.
    let mut occupied_cells = std::collections::HashSet::new();
    for p in &a.players {
        let cell = (p.x / 16, p.y / 16);
        assert!(
            occupied_cells.insert(cell),
            "two players spawned in the same 16×16 cell"
        );
    }

    assert_eq!(Game::kill_score(0), 0);
    assert_eq!(Game::kill_score(1), 100);
    assert_eq!(Game::kill_score(2), 300);
    assert_eq!(Game::kill_score(3), 700);

    // score = alive turns + walk + treats×10 + kill curve
    a.players[0].born_turn = 0;
    a.players[0].died_turn = Some(10);
    a.players[0].tiles_walked = 5;
    a.players[0].treats = 2;
    a.players[0].kills = 1;
    assert_eq!(a.score(&a.players[0]), 10 + 5 + 20 + 100);

    assert_eq!(a.observation(&a.players[0]).len(), OBSERVATION_SIZE);
    assert!(a.apply_neat_turn().len() <= DEFAULT_PLAYER_COUNT);
    assert!(a.add_human());
    a.turn_human(true);
    assert_eq!(a.players.iter().find(|p| p.id == 0).unwrap().direction, 1);
    println!("self-test passed");
}
