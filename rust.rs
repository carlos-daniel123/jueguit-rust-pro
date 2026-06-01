use std::io::{stdout, Stdout, Write};
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use rand::Rng;

#[derive(Clone, Copy, PartialEq)]
enum Tile {
    Empty,
    Wall,
    Food,
    Poison,
    Exit,
}

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

struct Game {
    w: usize,
    h: usize,
    map: Vec<Vec<Tile>>,
    snake: Vec<(usize, usize)>,
    dir: Dir,
    next_dir: Dir,
    food_eaten: u32,
    poison_hits: u32,
    score: i32,
    lives: i32,
    level: u32,
    game_over: bool,
    win: bool,
}

impl Game {
    fn new(w: usize, h: usize) -> Self {
        let mut g = Self {
            w,
            h,
            map: vec![vec![Tile::Empty; w]; h],
            snake: vec![(w / 2, h / 2), (w / 2 - 1, h / 2), (w / 2 - 2, h / 2)],
            dir: Dir::Right,
            next_dir: Dir::Right,
            food_eaten: 0,
            poison_hits: 0,
            score: 0,
            lives: 3,
            level: 1,
            game_over: false,
            win: false,
        };
        g.build_level();
        g
    }

    fn build_level(&mut self) {
        self.map = vec![vec![Tile::Empty; self.w]; self.h];

        for x in 0..self.w {
            self.map[0][x] = Tile::Wall;
            self.map[self.h - 1][x] = Tile::Wall;
        }
        for y in 0..self.h {
            self.map[y][0] = Tile::Wall;
            self.map[y][self.w - 1] = Tile::Wall;
        }

        let mut rng = rand::thread_rng();
        let obstacle_count = 8 + self.level as usize * 3;
        for _ in 0..obstacle_count {
            let x = rng.gen_range(2..self.w - 2);
            let y = rng.gen_range(2..self.h - 2);
            if !self.snake.contains(&(x, y)) {
                self.map[y][x] = Tile::Wall;
            }
        }

        self.place_tile(Tile::Food);
        self.place_tile(Tile::Poison);

        if self.level >= 3 {
            self.place_tile(Tile::Exit);
        }
    }

    fn place_tile(&mut self, tile: Tile) {
        let mut rng = rand::thread_rng();
        loop {
            let x = rng.gen_range(1..self.w - 1);
            let y = rng.gen_range(1..self.h - 1);
            if self.map[y][x] == Tile::Empty && !self.snake.contains(&(x, y)) {
                self.map[y][x] = tile;
                break;
            }
        }
    }

    fn set_dir(&mut self, d: Dir) {
        let invalid = matches!(
            (self.dir, d),
            (Dir::Up, Dir::Down)
                | (Dir::Down, Dir::Up)
                | (Dir::Left, Dir::Right)
                | (Dir::Right, Dir::Left)
        );
        if !invalid {
            self.next_dir = d;
        }
    }

    fn step(&mut self) {
        if self.game_over || self.win {
            return;
        }

        self.dir = self.next_dir;
        let (hx, hy) = self.snake[0];
        let (nx, ny) = match self.dir {
            Dir::Up => (hx, hy.saturating_sub(1)),
            Dir::Down => (hx, hy + 1),
            Dir::Left => (hx.saturating_sub(1), hy),
            Dir::Right => (hx + 1, hy),
        };

        if nx >= self.w || ny >= self.h {
            self.lose_life();
            return;
        }

        if self.map[ny][nx] == Tile::Wall {
            self.lose_life();
            return;
        }

        if self.snake.contains(&(nx, ny)) {
            self.lose_life();
            return;
        }

        self.snake.insert(0, (nx, ny));

        match self.map[ny][nx] {
            Tile::Food => {
                self.score += 10 * self.level as i32;
                self.food_eaten += 1;
                self.map[ny][nx] = Tile::Empty;
                self.place_tile(Tile::Food);

                if self.food_eaten % 3 == 0 {
                    self.level += 1;
                    self.score += 25;
                    self.build_level();
                    self.snake.truncate(3);
                    self.snake[0] = (self.w / 2, self.h / 2);
                    self.snake[1] = (self.w / 2 - 1, self.h / 2);
                    self.snake[2] = (self.w / 2 - 2, self.h / 2);
                }
            }
            Tile::Poison => {
                self.score -= 15;
                self.poison_hits += 1;
                self.map[ny][nx] = Tile::Empty;
                if self.snake.len() > 2 {
                    self.snake.pop();
                }
                if self.poison_hits >= 4 {
                    self.lose_life();
                } else {
                    self.place_tile(Tile::Poison);
                }
            }
            Tile::Exit => {
                if self.level >= 3 {
                    self.win = true;
                }
            }
            _ => {
                self.snake.pop();
            }
        }
    }

    fn lose_life(&mut self) {
        self.lives -= 1;
        self.score -= 20;
        if self.lives <= 0 {
            self.game_over = true;
            return;
        }
        self.snake = vec![(self.w / 2, self.h / 2), (self.w / 2 - 1, self.h / 2), (self.w / 2 - 2, self.h / 2)];
        self.dir = Dir::Right;
        self.next_dir = Dir::Right;
    }

    fn draw(&self, out: &mut Stdout) -> std::io::Result<()> {
        queue!(out, MoveTo(0, 0), Clear(ClearType::All))?;
        queue!(
            out,
            SetForegroundColor(Color::Yellow),
            Print("Rusty Dungeon Snake\n"),
            ResetColor
        )?;
        queue!(
            out,
            Print(format!(
                "Score: {}   Lives: {}   Level: {}   Food: {}   Poison: {}\n",
                self.score, self.lives, self.level, self.food_eaten, self.poison_hits
            ))
        )?;
        queue!(out, Print("Controls: arrows/WASD move, q quit\n\n"))?;

        for y in 0..self.h {
            for x in 0..self.w {
                let mut printed = false;

                if self.snake[0] == (x, y) {
                    queue!(out, SetForegroundColor(Color::Green), Print("@"), ResetColor)?;
                    printed = true;
                } else if self.snake.contains(&(x, y)) {
                    queue!(out, SetForegroundColor(Color::DarkGreen), Print("o"), ResetColor)?;
                    printed = true;
                }

                if !printed {
                    match self.map[y][x] {
                        Tile::Empty => queue!(out, Print(" "))?,
                        Tile::Wall => queue!(out, SetForegroundColor(Color::Blue), Print("#"), ResetColor)?,
                        Tile::Food => queue!(out, SetForegroundColor(Color::Red), Print("*"), ResetColor)?,
                        Tile::Poison => queue!(out, SetForegroundColor(Color::Magenta), Print("!"), ResetColor)?,
                        Tile::Exit => queue!(out, SetForegroundColor(Color::Cyan), Print("X"), ResetColor)?,
                    }
                }
            }
            queue!(out, Print("\n"))?;
        }

        if self.game_over {
            queue!(
                out,
                Print("\nGame Over. Presiona q para salir.\n")
            )?;
        }
        if self.win {
            queue!(
                out,
                Print("\n¡Ganaste! Presiona q para salir.\n")
            )?;
        }

        out.flush()?;
        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, Hide)?;

    let mut game = Game::new(28, 18);
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(140);

    loop {
        game.draw(&mut out)?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Up | KeyCode::Char('w') => game.set_dir(Dir::Up),
                    KeyCode::Down | KeyCode::Char('s') => game.set_dir(Dir::Down),
                    KeyCode::Left | KeyCode::Char('a') => game.set_dir(Dir::Left),
                    KeyCode::Right | KeyCode::Char('d') => game.set_dir(Dir::Right),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            game.step();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(out, Show, LeaveAlternateScreen)?;
    Ok(())
}
