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
const DIRECTIONS: [(isize, isize); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

#[derive(Clone, Debug, PartialEq, Eq)]
struct Player {
    id: usize,
    x: usize,
    y: usize,
    direction: usize,
    alive: bool,
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
                direction: rng.range(4),
                alive: true,
            });
        }
    }

    fn visible_cells(
        &self,
        player: &Player,
        depth: usize,
        half_width: isize,
    ) -> Vec<(usize, usize)> {
        let (dx, dy) = DIRECTIONS[player.direction];
        let (sx, sy) = (-dy, dx);
        let mut cells = Vec::new();
        for forward in 1..=depth as isize {
            for side in -half_width.min(forward / 2)..=half_width.min(forward / 2) {
                let x = player.x as isize + dx * forward + sx * side;
                let y = player.y as isize + dy * forward + sy * side;
                if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
                    continue;
                }
                let point = (x as usize, y as usize);
                cells.push(point);
                if self.is_wall(point.0, point.1) {
                    continue;
                }
            }
        }
        cells
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
            if !player.alive {
                continue;
            }
            match actions.get(player.id - 1).copied().unwrap_or("wait") {
                "left" => player.direction = (player.direction + 3) % 4,
                "right" => player.direction = (player.direction + 1) % 4,
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
        for player in self.players.iter().filter(|p| p.alive) {
            if actions.get(player.id - 1).copied() == Some("shoot") {
                let visible = self.visible_cells(player, 6, 2);
                for other in self.players.iter().filter(|p| p.alive && p.id != player.id) {
                    if visible.contains(&(other.x, other.y)) {
                        hit[other.id - 1] = true;
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
        }
        self.turn += 1;
        hit_ids
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
    let mut screen = format!(
        "\x1b[H seed: {:?}  generation: {}  turn: {}  players: {}/{}\r\n r regenerate   Ctrl-W quit\r\n",
        game.seed,
        game.generation,
        game.turn,
        game.players.iter().filter(|p| p.alive).count(),
        PLAYER_COUNT
    );
    for y in 0..game.height {
        for x in 0..game.width {
            if let Some(player) = game
                .players
                .iter()
                .find(|p| p.alive && p.x == x && p.y == y)
            {
                screen.push_str(&format!(
                    "\x1b[38;5;{}m{:02}\x1b[0m",
                    COLORS[player.id - 1],
                    player.id
                ));
            } else {
                screen.push_str(if game.is_wall(x, y) { "##" } else { "  " });
            }
        }
        screen.push_str("\r\n");
    }
    screen.push_str("\x1b[J");
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
    assert!(!a.visible_cells(&a.players[0], 6, 2).is_empty());
    assert!(a.apply_turn(&["wait"; PLAYER_COUNT]).is_empty());
    assert_eq!(a.turn, 1);
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
        let game = Game::new(
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
                _ => {}
            }
        }
    }
}
