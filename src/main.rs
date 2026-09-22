use std::{
    env,
    io::{self, IsTerminal, Read, Write},
    process::{Command, Stdio},
};

const PLAYER_COUNT: usize = 32;
const COLORS: [u8; PLAYER_COUNT] = [
    196, 202, 208, 214, 220, 118, 46, 48, 51, 39, 33, 69, 93, 129, 135, 171, 201, 199, 207, 177,
    141, 105, 75, 81, 87, 123, 159, 183, 219, 227, 155, 49,
];
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

struct Game {
    seed: String,
    generation: u64,
    width: usize,
    height: usize,
    turn: u64,
    walls: Vec<bool>,
    players: Vec<Player>,
}

impl Game {
    fn new(seed: String, generation: u64, width: usize, height: usize) -> Self {
        let width = width.max(20);
        let height = height.max(12);
        let mut game = Self {
            seed: seed.clone(),
            generation,
            width,
            height,
            turn: 0,
            walls: vec![false; width * height],
            players: Vec::with_capacity(PLAYER_COUNT),
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
            free.len() >= PLAYER_COUNT,
            "map is too small for 32 players"
        );
        for id in 1..=PLAYER_COUNT {
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
        let mut proposed = vec![None; PLAYER_COUNT];
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
        for id in 0..PLAYER_COUNT {
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
        let mut hit = [false; PLAYER_COUNT];
        let mut kills = [0; PLAYER_COUNT];
        for player in self.players.iter().filter(|p| p.alive && p.id > 0) {
            if actions.get(player.id - 1).copied() == Some("shoot") {
                let visible = self.visible_cells(player, 6);
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
        let visible = self.visible_cells(&shooter, 6);
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
}

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
        "\x1b[H seed: {:?}  generation: {}  turn: {}  players: {}/{}  {}\r\n ` add player 00   Q/E turn   WASD move   Space shoot   Esc leaderboard   r regenerate   Ctrl-W quit\r\n",
        game.seed,
        game.generation,
        game.turn,
        game.players.iter().filter(|p| p.alive).count(),
        PLAYER_COUNT,
        human_status
    );
    let mut visible = vec![false; game.width * game.height];
    for player in game.players.iter().filter(|p| p.alive) {
        for (x, y) in game.visible_cells(player, 6) {
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
                        COLORS[player.id - 1]
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
        screen.push_str("\r\n");
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
    let mut a = Game::new("replay phrase".into(), 3, 50, 24);
    let b = Game::new("replay phrase".into(), 3, 50, 24);
    assert_eq!(a.walls, b.walls);
    assert_eq!(a.players, b.players);
    assert_eq!(a.players.len(), PLAYER_COUNT);
    assert!(a.players.iter().all(|p| !a.is_wall(p.x, p.y)));
    assert_eq!(
        a.players
            .iter()
            .map(|p| (p.x, p.y))
            .collect::<std::collections::HashSet<_>>()
            .len(),
        PLAYER_COUNT
    );
    assert!(!a.visible_cells(&a.players[0], 6).is_empty());
    assert!(a.apply_turn(&["wait"; PLAYER_COUNT]).is_empty());
    assert_eq!(a.turn, 1);
    assert!(a.add_human());
    a.move_human(1, 0);
    let human = a.players.iter().find(|p| p.id == 0).unwrap();
    assert_eq!(human.direction, 0);
    assert_eq!(a.score(human), 1);
    println!("self-test passed");
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--self-test") {
        self_test();
        return Ok(());
    }
    let seed = args
        .first()
        .cloned()
        .unwrap_or_else(|| "battle-royal".into());
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other("Run this in an interactive terminal."));
    }
    let _terminal = Terminal::enter()?;
    let mut generation = 0;
    loop {
        let (columns, rows) = terminal_size();
        let mut game = Game::new(
            seed.clone(),
            generation,
            columns / 2,
            rows.saturating_sub(2),
        );
        print!("{}", render(&game));
        io::stdout().flush()?;
        loop {
            let mut key = [0];
            io::stdin().read_exact(&mut key)?;
            match key[0] {
                0x17 => return Ok(()),
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
