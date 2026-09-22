use std::env;

use bevy::camera::Viewport;
use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    window::WindowCloseRequested,
};
use vivalaakam_neuro_neat::{Config, Genome, Organism};

const DEFAULT_PLAYER_COUNT: usize = 32;
const VISION_DEPTH: usize = 6;
const OBSERVATION_SIZE: usize = 2 + 8 + (VISION_DEPTH * 2 + 1) * (VISION_DEPTH * 2 + 1);
const ACTIONS: [&str; 5] = ["left", "right", "forward", "shoot", "wait"];
const DIRECTIONS: [(isize, isize); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Player {
    id: usize,
    x: usize,
    y: usize,
    direction: usize,
    alive: bool,
    kills: u64,
    born_turn: u64,
    died_turn: Option<u64>,
}

struct Rng(u64);

impl Rng {
    fn from_phrase(phrase: &str, generation: u64) -> Self {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in phrase.bytes().chain(generation.to_le_bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self(hash.max(1))
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, end: usize) -> usize {
        (self.next() % end as u64) as usize
    }
}

fn new_brains(seed: &str, generation: u64, player_count: usize) -> Vec<Organism> {
    let config = Config::default();
    (0..player_count)
        .map(|id| {
            let genome =
                Genome::generate_genome(OBSERVATION_SIZE, ACTIONS.len(), Vec::new(), None, &config)
                    .expect("NEAT genome generation failed");
            let mut weights = genome.to_weights();
            let mut rng = Rng::from_phrase(&format!("{seed}:agent:{id}"), generation);
            let node_count = OBSERVATION_SIZE + ACTIONS.len();
            for node in 0..node_count {
                weights[6 + node * 4 + 1] = rng.next() as f32 / u64::MAX as f32 * 2.0 - 1.0;
            }
            for connection in 0..OBSERVATION_SIZE * ACTIONS.len() {
                weights[6 + node_count * 4 + connection * 4 + 2] =
                    rng.next() as f32 / u64::MAX as f32 * 2.0 - 1.0;
            }
            Organism::new(Genome::from_weights(weights))
        })
        .collect()
}

struct Game {
    seed: String,
    generation: u64,
    width: usize,
    height: usize,
    player_count: usize,
    turn: u64,
    walls: Vec<bool>,
    players: Vec<Player>,
    brains: Vec<Organism>,
}

impl Game {
    fn new(
        seed: String,
        generation: u64,
        width: usize,
        height: usize,
        player_count: usize,
    ) -> Self {
        let width = width.max(20);
        let height = height.max(12);
        let mut game = Self {
            seed: seed.clone(),
            generation,
            width,
            height,
            player_count,
            turn: 0,
            walls: vec![false; width * height],
            players: Vec::with_capacity(player_count + 1),
            brains: new_brains(&seed, generation, player_count),
        };
        let mut rng = Rng::from_phrase(&seed, generation);
        game.generate(&mut rng);
        game
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn is_wall(&self, x: usize, y: usize) -> bool {
        self.walls[self.index(x, y)]
    }

    fn set_wall(&mut self, x: usize, y: usize) {
        let index = self.index(x, y);
        self.walls[index] = true;
    }

    fn generate(&mut self, rng: &mut Rng) {
        for y in 0..self.height {
            for x in 0..self.width {
                if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
                    self.set_wall(x, y);
                }
            }
        }
        for _ in 0..(self.width * self.height / 55) {
            let (mut x, mut y) = (
                2 + rng.range(self.width - 5),
                2 + rng.range(self.height - 5),
            );
            let (dx, dy) = DIRECTIONS[rng.range(DIRECTIONS.len())];
            for _ in 0..(2 + rng.range(5)) {
                if x > 0 && y > 0 && x + 1 < self.width && y + 1 < self.height {
                    self.set_wall(x, y);
                }
                x = x.saturating_add_signed(dx);
                y = y.saturating_add_signed(dy);
            }
        }
        let mut free: Vec<_> = (1..self.height - 1)
            .flat_map(|y| (1..self.width - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| !self.is_wall(x, y))
            .collect();
        assert!(
            free.len() >= self.player_count,
            "map is too small for the requested player count"
        );
        for id in 1..=self.player_count {
            let pick = rng.range(free.len());
            let (x, y) = free.swap_remove(pick);
            self.players.push(Player {
                id,
                x,
                y,
                direction: rng.range(DIRECTIONS.len()),
                alive: true,
                kills: 0,
                born_turn: 0,
                died_turn: None,
            });
        }
    }

    fn visible_cells(&self, player: &Player, depth: usize) -> Vec<(usize, usize)> {
        let (dx, dy) = DIRECTIONS[player.direction];
        let mut cells = Vec::new();
        for y in player.y.saturating_sub(depth)..=(player.y + depth).min(self.height - 1) {
            for x in player.x.saturating_sub(depth)..=(player.x + depth).min(self.width - 1) {
                let (relative_x, relative_y) = (
                    x as isize - player.x as isize,
                    y as isize - player.y as isize,
                );
                let dot = relative_x * dx + relative_y * dy;
                let cross = relative_x * dy - relative_y * dx;
                if dot > 0
                    && cross.abs() <= dot
                    && self.has_line_of_sight((player.x, player.y), (x, y))
                {
                    cells.push((x, y));
                }
            }
        }
        cells
    }

    fn has_line_of_sight(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        let (mut x, mut y) = (from.0 as isize, from.1 as isize);
        let (end_x, end_y) = (to.0 as isize, to.1 as isize);
        let (dx, dy) = ((end_x - x).abs(), -(end_y - y).abs());
        let (step_x, step_y) = ((end_x - x).signum(), (end_y - y).signum());
        let mut error = dx + dy;
        while (x, y) != (end_x, end_y) {
            let twice_error = error * 2;
            if twice_error >= dy {
                error += dy;
                x += step_x;
            }
            if twice_error <= dx {
                error += dx;
                y += step_y;
            }
            if (x, y) != (end_x, end_y) && self.is_wall(x as usize, y as usize) {
                return false;
            }
        }
        true
    }

    // Actions are collected first, then resolved together: forward, left, right, shoot, wait.
    fn apply_turn(&mut self, actions: &[&str]) -> Vec<usize> {
        let mut occupied = vec![false; self.width * self.height];
        for p in self.players.iter().filter(|p| p.alive) {
            occupied[self.index(p.x, p.y)] = true;
        }
        let mut proposed = vec![None; self.player_count];
        let width = self.width;
        let walls = &self.walls;
        for player in &mut self.players {
            if !player.alive || player.id == 0 {
                continue;
            }
            match actions.get(player.id - 1).copied().unwrap_or("wait") {
                "left" => {
                    player.direction = (player.direction + DIRECTIONS.len() - 1) % DIRECTIONS.len()
                }
                "right" => player.direction = (player.direction + 1) % DIRECTIONS.len(),
                "forward" => {
                    let (dx, dy) = DIRECTIONS[player.direction];
                    let x = player.x.saturating_add_signed(dx);
                    let y = player.y.saturating_add_signed(dy);
                    if !walls[y * width + x] {
                        proposed[player.id - 1] = Some((x, y));
                    }
                }
                _ => {}
            }
        }
        for id in 0..self.player_count {
            if let Some(target) = proposed[id]
                && proposed
                    .iter()
                    .filter(|&&other| other == Some(target))
                    .count()
                    == 1
                && !occupied[self.index(target.0, target.1)]
            {
                self.players[id].x = target.0;
                self.players[id].y = target.1;
            }
        }
        let mut hit = vec![false; self.player_count];
        let mut kills = vec![0; self.player_count];
        for player in self.players.iter().filter(|p| p.alive && p.id > 0) {
            if actions.get(player.id - 1).copied() == Some("shoot") {
                let visible = self.visible_cells(player, VISION_DEPTH);
                for other in self
                    .players
                    .iter()
                    .filter(|p| p.alive && p.id > 0 && p.id != player.id)
                {
                    if visible.contains(&(other.x, other.y)) {
                        hit[other.id - 1] = true;
                        kills[player.id - 1] += 1;
                    }
                }
            }
        }
        let hit_ids: Vec<_> = hit
            .iter()
            .enumerate()
            .filter_map(|(i, &was_hit)| was_hit.then_some(i + 1))
            .collect();
        for id in &hit_ids {
            self.players[*id - 1].alive = false;
            self.players[*id - 1].died_turn = Some(self.turn + 1);
        }
        for (index, count) in kills.into_iter().enumerate() {
            self.players[index].kills += count;
        }
        self.turn += 1;
        hit_ids
    }

    fn add_human(&mut self) -> bool {
        if self.players.iter().any(|p| p.id == 0) {
            return false;
        }
        let occupied: std::collections::HashSet<_> =
            self.players.iter().map(|p| (p.x, p.y)).collect();
        let (x, y) = (1..self.height - 1)
            .flat_map(|y| (1..self.width - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| !self.is_wall(x, y) && !occupied.contains(&(x, y)))
            .min_by_key(|&(x, y)| x.abs_diff(self.width / 2) + y.abs_diff(self.height / 2))
            .expect("map has no free cell");
        self.players.push(Player {
            id: 0,
            x,
            y,
            direction: 0,
            alive: true,
            kills: 0,
            born_turn: self.turn,
            died_turn: None,
        });
        true
    }

    fn move_human(&mut self, forward: isize, side: isize) {
        let Some(index) = self.players.iter().position(|p| p.id == 0 && p.alive) else {
            return;
        };
        let (dx, dy) = DIRECTIONS[self.players[index].direction];
        let (side_x, side_y) = (-dy, dx);
        let target = (
            self.players[index]
                .x
                .saturating_add_signed(dx * forward + side_x * side),
            self.players[index]
                .y
                .saturating_add_signed(dy * forward + side_y * side),
        );
        if !self.is_wall(target.0, target.1)
            && !self.players.iter().any(|p| p.alive && (p.x, p.y) == target)
        {
            self.players[index].x = target.0;
            self.players[index].y = target.1;
        }
        self.turn += 1;
    }

    fn turn_human(&mut self, clockwise: bool) {
        if let Some(human) = self.players.iter_mut().find(|p| p.id == 0 && p.alive) {
            human.direction = (human.direction + if clockwise { 1 } else { DIRECTIONS.len() - 1 })
                % DIRECTIONS.len();
            self.turn += 1;
        }
    }

    fn shoot_human(&mut self) {
        let Some(shooter) = self.players.iter().find(|p| p.id == 0 && p.alive).cloned() else {
            return;
        };
        let visible = self.visible_cells(&shooter, VISION_DEPTH);
        let victims: Vec<_> = self
            .players
            .iter()
            .filter(|p| p.alive && p.id > 0 && visible.contains(&(p.x, p.y)))
            .map(|p| p.id)
            .collect();
        for id in &victims {
            self.players[*id - 1].alive = false;
            self.players[*id - 1].died_turn = Some(self.turn + 1);
        }
        if let Some(human) = self.players.iter_mut().find(|p| p.id == 0) {
            human.kills += victims.len() as u64;
        }
        self.turn += 1;
    }

    fn score(&self, player: &Player) -> u64 {
        (player.died_turn.unwrap_or(self.turn) - player.born_turn) + player.kills * 10
    }

    fn observation(&self, player: &Player) -> Vec<f32> {
        let mut input = Vec::with_capacity(OBSERVATION_SIZE);
        input.push(player.x as f32 / (self.width - 1) as f32 * 2.0 - 1.0);
        input.push(player.y as f32 / (self.height - 1) as f32 * 2.0 - 1.0);
        input.extend(
            (0..DIRECTIONS.len()).map(|direction| (direction == player.direction) as u8 as f32),
        );
        let mut visible = vec![false; self.width * self.height];
        for (x, y) in self.visible_cells(player, VISION_DEPTH) {
            visible[self.index(x, y)] = true;
        }
        for relative_y in -(VISION_DEPTH as isize)..=VISION_DEPTH as isize {
            for relative_x in -(VISION_DEPTH as isize)..=VISION_DEPTH as isize {
                let x = player.x as isize + relative_x;
                let y = player.y as isize + relative_y;
                input.push(
                    if x < 0
                        || y < 0
                        || x >= self.width as isize
                        || y >= self.height as isize
                        || !visible[self.index(x as usize, y as usize)]
                    {
                        -1.0
                    } else if self.is_wall(x as usize, y as usize) {
                        -0.5
                    } else if self.players.iter().any(|p| {
                        p.alive && p.id != player.id && p.x == x as usize && p.y == y as usize
                    }) {
                        1.0
                    } else {
                        0.0
                    },
                );
            }
        }
        input
    }

    fn neat_actions(&self) -> Vec<&'static str> {
        let sensed: Vec<_> = self
            .players
            .iter()
            .filter(|p| p.alive && p.id > 0)
            .map(|p| (p.id, self.observation(p)))
            .collect();
        let mut actions = vec!["wait"; self.player_count];
        for (id, input) in sensed {
            let output = self.brains[id - 1].network.activate(input);
            let action = output
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(index, _)| index)
                .unwrap_or(ACTIONS.len() - 1);
            actions[id - 1] = ACTIONS[action];
        }
        actions
    }

    fn apply_neat_turn(&mut self) -> Vec<usize> {
        let actions = self.neat_actions();
        self.apply_turn(&actions)
    }
}

#[cfg(any())]
mod terminal {
    use super::*;

    fn terminal_size() -> (usize, usize) {
        let output = Command::new("stty")
            .arg("size")
            .stdin(Stdio::inherit())
            .output()
            .ok();
        let sizes: Vec<_> = output
            .as_ref()
            .and_then(|out| std::str::from_utf8(&out.stdout).ok())
            .into_iter()
            .flat_map(|text| {
                text.split_whitespace()
                    .filter_map(|n| n.parse::<usize>().ok())
            })
            .collect();
        match sizes.as_slice() {
            [rows, columns] => (*columns, *rows),
            _ => (120, 40),
        }
    }

    fn render(game: &Game) -> String {
        let human_status = if game.players.iter().any(|p| p.id == 0 && p.alive) {
            "00 active"
        } else {
            "` add 00"
        };
        let mut screen = format!(
            "\x1b[H seed: {:?}  generation: {}  turn #{}  map: {}×{}  players: {}/{}  {}\r\n Enter next AI turn   ` add player 00   Q/E turn   WASD move   Space shoot   Esc leaderboard   r regenerate   Ctrl-W quit\r\n",
            game.seed,
            game.generation,
            game.turn,
            game.width,
            game.height,
            game.players.iter().filter(|p| p.alive && p.id > 0).count(),
            game.player_count,
            human_status
        );
        let mut visible = vec![false; game.width * game.height];
        for player in game.players.iter().filter(|p| p.alive) {
            for (x, y) in game.visible_cells(player, VISION_DEPTH) {
                visible[game.index(x, y)] = true;
            }
        }
        for y in 0..game.height {
            for x in 0..game.width {
                if let Some(player) = game
                    .players
                    .iter()
                    .find(|p| p.alive && p.x == x && p.y == y)
                {
                    screen.push_str(&format!(
                        "{}\x1b[38;5;{}m{:02}\x1b[0m",
                        if visible[game.index(x, y)] {
                            "\x1b[48;5;236m"
                        } else {
                            ""
                        },
                        if player.id == 0 {
                            15
                        } else {
                            COLORS[(player.id - 1) % COLORS.len()]
                        },
                        player.id
                    ));
                } else {
                    let tile = if game.is_wall(x, y) { "##" } else { "  " };
                    if visible[game.index(x, y)] {
                        screen.push_str("\x1b[48;5;236m");
                        screen.push_str(tile);
                        screen.push_str("\x1b[0m");
                    } else {
                        screen.push_str(tile);
                    }
                }
            }
            if y + 1 < game.height {
                screen.push_str("\r\n");
            }
        }
        screen.push_str("\x1b[J");
        screen
    }

    fn render_pause(game: &Game) -> String {
        let mut players: Vec<_> = game.players.iter().collect();
        players.sort_by_key(|p| (std::cmp::Reverse(game.score(p)), p.id));
        let mut screen = String::from(
            "\x1b[H\x1b[2J PAUSED — leaderboard\r\n score = survival turns + kills × 10\r\n\r\n",
        );
        for player in players {
            screen.push_str(&format!(
                " {:02}  score {:>4}  survived {:>4}  kills {:>3}  {}\r\n",
                player.id,
                game.score(player),
                player.died_turn.unwrap_or(game.turn) - player.born_turn,
                player.kills,
                if player.alive { "alive" } else { "dead" },
            ));
        }
        screen.push_str("\r\n Esc resume   Ctrl-W quit\x1b[J");
        screen
    }

    struct Terminal {
        original: String,
    }

    impl Terminal {
        fn enter() -> io::Result<Self> {
            let original = String::from_utf8(
                Command::new("stty")
                    .arg("-g")
                    .stdin(Stdio::inherit())
                    .output()?
                    .stdout,
            )
            .unwrap_or_default()
            .trim()
            .to_owned();
            if original.is_empty() {
                return Err(io::Error::other("interactive terminal required"));
            }
            let status = Command::new("stty")
                .args(["raw", "-echo"])
                .stdin(Stdio::inherit())
                .status()?;
            if !status.success() {
                return Err(io::Error::other("could not enable raw terminal mode"));
            }
            print!("\x1b[?1049h\x1b[?25l");
            io::stdout().flush()?;
            Ok(Self { original })
        }
    }

    impl Drop for Terminal {
        fn drop(&mut self) {
            let _ = Command::new("stty")
                .arg(&self.original)
                .stdin(Stdio::inherit())
                .status();
            print!("\x1b[?25h\x1b[?1049l");
            let _ = io::stdout().flush();
        }
    }

    fn self_test() {
        let mut a = Game::new("replay phrase".into(), 3, 50, 24, DEFAULT_PLAYER_COUNT);
        let b = Game::new("replay phrase".into(), 3, 50, 24, DEFAULT_PLAYER_COUNT);
        assert_eq!(a.walls, b.walls);
        assert_eq!(a.players, b.players);
        assert_eq!(a.brains[0].as_json(), b.brains[0].as_json());
        assert_eq!(a.players.len(), DEFAULT_PLAYER_COUNT);
        assert!(a.players.iter().all(|p| !a.is_wall(p.x, p.y)));
        assert_eq!(
            a.players
                .iter()
                .map(|p| (p.x, p.y))
                .collect::<std::collections::HashSet<_>>()
                .len(),
            DEFAULT_PLAYER_COUNT
        );
        assert!(!a.visible_cells(&a.players[0], VISION_DEPTH).is_empty());
        assert_eq!(a.observation(&a.players[0]).len(), OBSERVATION_SIZE);
        assert_eq!(a.neat_actions().len(), DEFAULT_PLAYER_COUNT);
        assert!(a.apply_turn(&vec!["wait"; DEFAULT_PLAYER_COUNT]).is_empty());
        assert_eq!(a.turn, 1);
        assert!(a.add_human());
        a.move_human(1, 0);
        let human = a.players.iter().find(|p| p.id == 0).unwrap();
        assert_eq!(human.direction, 0);
        assert_eq!(a.score(human), 1);
        println!("self-test passed");
    }

    fn option_value(args: &[String], index: &mut usize, name: &str) -> io::Result<usize> {
        *index += 1;
        args.get(*index)
            .ok_or_else(|| io::Error::other(format!("{name} needs a value")))?
            .parse()
            .map_err(|_| io::Error::other(format!("{name} must be a positive integer")))
    }

    fn main() -> io::Result<()> {
        let args: Vec<_> = env::args().skip(1).collect();
        let mut seed = "battle-royal".to_owned();
        let mut width = None;
        let mut height = None;
        let mut player_count = DEFAULT_PLAYER_COUNT;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--self-test" => {
                    self_test();
                    return Ok(());
                }
                "--width" => width = Some(option_value(&args, &mut index, "--width")?),
                "--height" => height = Some(option_value(&args, &mut index, "--height")?),
                "--players" => player_count = option_value(&args, &mut index, "--players")?,
                "--help" | "-h" => {
                    println!(
                        "Usage: ai-neat-battle-royal [seed] [--width N] [--height N] [--players N]"
                    );
                    return Ok(());
                }
                value if !value.starts_with('-') => seed = value.to_owned(),
                value => return Err(io::Error::other(format!("unknown option: {value}"))),
            }
            index += 1;
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other("Run this in an interactive terminal."));
        }
        let _terminal = Terminal::enter()?;
        let mut generation = 0;
        loop {
            let (columns, rows) = terminal_size();
            let width = width.unwrap_or(columns / 2);
            let height = height.unwrap_or(rows.saturating_sub(2));
            if width < 8 || height < 8 {
                return Err(io::Error::other("map size must be at least 8×8"));
            }
            if player_count == 0 || player_count > (width - 2) * (height - 2) / 2 {
                return Err(io::Error::other("player count does not fit on this map"));
            }
            let mut game = Game::new(seed.clone(), generation, width, height, player_count);
            print!("{}", render(&game));
            io::stdout().flush()?;
            loop {
                let mut key = [0];
                io::stdin().read_exact(&mut key)?;
                match key[0] {
                    0x17 => return Ok(()),
                    b'\r' | b'\n' => {
                        game.apply_neat_turn();
                    }
                    b'r' | b'R' => {
                        generation += 1;
                        break;
                    }
                    b'`' | b'~' | 0xD1 => {
                        game.add_human();
                    }
                    b'q' | b'Q' => game.turn_human(false),
                    b'e' | b'E' => game.turn_human(true),
                    b'w' | b'W' => game.move_human(1, 0),
                    b's' | b'S' => game.move_human(-1, 0),
                    b'a' | b'A' => game.move_human(0, -1),
                    b'd' | b'D' => game.move_human(0, 1),
                    b' ' => game.shoot_human(),
                    0x1b => {
                        print!("{}", render_pause(&game));
                        io::stdout().flush()?;
                        loop {
                            let mut pause_key = [0];
                            io::stdin().read_exact(&mut pause_key)?;
                            if pause_key[0] == 0x17 {
                                return Ok(());
                            }
                            if pause_key[0] == 0x1b {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
                print!("{}", render(&game));
                io::stdout().flush()?;
            }
        }
    }
}

const TILE: f32 = 18.0;
const MAP_ZOOM_MIN: f32 = 0.2;
const MAP_ZOOM_MAX: f32 = 5.0;
const MAP_ZOOM_FACTOR: f32 = 1.12;
const MAP_PAN_SPEED: f32 = 480.0;
const LEADERBOARD_PANEL_PX: f32 = 420.0;

fn map_half_extents(game: &Game) -> Vec2 {
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

#[derive(Component)]
struct BoardVisual;

#[derive(Component)]
struct BoardCamera;

#[derive(Component)]
struct LeaderboardText;

#[derive(Component)]
struct StatusText;

#[derive(Resource)]
struct MapView {
    pan: Vec2,
    zoom: f32,
}

#[derive(Resource)]
struct Battle {
    game: Game,
    seed: String,
    generation: u64,
    paused: bool,
    dirty: bool,
}

#[derive(Default)]
struct Launch {
    seed: String,
    width: usize,
    height: usize,
    players: usize,
}

fn self_test() {
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

fn parse_launch() -> Launch {
    let mut launch = Launch {
        seed: "battle-royal".into(),
        width: 50,
        height: 30,
        players: DEFAULT_PLAYER_COUNT,
    };
    let args: Vec<_> = env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--self-test" => {
                self_test();
                std::process::exit(0);
            }
            "--width" => launch.width = cli_value(&args, &mut index, "--width"),
            "--height" => launch.height = cli_value(&args, &mut index, "--height"),
            "--players" => launch.players = cli_value(&args, &mut index, "--players"),
            seed if !seed.starts_with('-') => launch.seed = seed.into(),
            option => panic!("unknown option: {option}"),
        }
        index += 1;
    }
    launch
}

fn cli_value(args: &[String], index: &mut usize, name: &str) -> usize {
    *index += 1;
    args.get(*index)
        .unwrap_or_else(|| panic!("{name} needs a value"))
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be a positive integer"))
}

fn point(game: &Game, x: usize, y: usize) -> Vec3 {
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

fn modifier_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
}

fn setup(mut commands: Commands) {
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
                Text::new("score = survival + kills × 10"),
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

fn handle_map_view(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut view: ResMut<MapView>) {
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

fn apply_map_camera(
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

fn handle_window_close(
    mut close: MessageReader<WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    for _ in close.read() {
        exit.write(AppExit::Success);
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut battle: ResMut<Battle>,
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
        battle.generation += 1;
        battle.game = Game::new(
            battle.seed.clone(),
            battle.generation,
            battle.game.width,
            battle.game.height,
            battle.game.player_count,
        );
    }
    battle.dirty = true;
}

fn redraw(
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

fn update_hud(
    battle: Res<Battle>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LeaderboardText>>,
        Query<&mut Text, With<StatusText>>,
    )>,
) {
    if !battle.dirty {
        return;
    }
    let game = &battle.game;
    let mut players: Vec<_> = game.players.iter().collect();
    players.sort_by_key(|p| (std::cmp::Reverse(game.score(p)), p.id));
    let board = players
        .into_iter()
        .map(|p| {
            format!(
                "#{:02}  score {:>5}  turns {:>4}  kills {:>2}  {}",
                p.id,
                game.score(p),
                p.died_turn.unwrap_or(game.turn) - p.born_turn,
                p.kills,
                if p.alive { "alive" } else { "dead" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if let Ok(mut text) = texts.p0().single_mut() {
        **text = board;
    }
    let pause = if battle.paused { "  PAUSED" } else { "" };
    let status_line = format!(
        "seed {:?}   gen {}   turn #{}   map {}×{}   agents {}/{}{pause}\n\
Enter AI   ` 00   Q/E turn   WASD move   Space shoot   Esc pause   −/= zoom   arrows pan   Ctrl+W quit",
        game.seed,
        game.generation,
        game.turn,
        game.width,
        game.height,
        game.players.iter().filter(|p| p.alive && p.id > 0).count(),
        game.player_count,
    );
    if let Ok(mut text) = texts.p1().single_mut() {
        **text = status_line;
    }
}

fn main() {
    let launch = parse_launch();
    assert!(
        launch.width >= 8
            && launch.height >= 8
            && launch.players > 0
            && launch.players <= (launch.width - 2) * (launch.height - 2) / 2,
        "invalid map size or player count"
    );
    let game = Game::new(
        launch.seed.clone(),
        0,
        launch.width,
        launch.height,
        launch.players,
    );
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "AI NEAT Battle Royal".into(),
                resolution: (1200, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.015, 0.02, 0.03)))
        .insert_resource(Battle {
            game,
            seed: launch.seed,
            generation: 0,
            paused: false,
            dirty: true,
        })
        .insert_resource(MapView {
            pan: Vec2::ZERO,
            zoom: 1.0,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_window_close,
                handle_map_view,
                apply_map_camera,
                handle_input,
                update_hud,
                redraw,
            )
                .chain(),
        )
        .run();
}
