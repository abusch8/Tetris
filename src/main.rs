use std::{io::{stdout, Write}, time::{Instant, Duration}, env, thread::sleep};
use crossterm::{
    Result, QueueableCommand, execute,
    style::{PrintStyledContent, StyledContent, Color, Stylize, ContentStyle, Print},
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode},
    event::{Event, KeyCode, read, poll},
};
use rand::{thread_rng, seq::SliceRandom};
use num_derive::FromPrimitive;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::debug::*;

mod debug;

type Dimension = (i32, i32);
type Origin = (f32, f32);

const BOARD_DIMENSION: Dimension = (10, 20);

#[derive(Clone, Copy, FromPrimitive)]
enum Direction { Left, Right, Down }

#[derive(Clone, Copy, EnumIter, FromPrimitive, PartialEq)]
enum TetrominoVariant { I, J, L, O, S, T, Z }

#[derive(Clone)]
struct Tetromino {
    shape: Vec<Dimension>,
    origin: Origin,
    color: Color,
    variant: TetrominoVariant,
}

impl Tetromino {
    fn new(variant: TetrominoVariant) -> Self {
        match variant {
            TetrominoVariant::I => Tetromino {
                shape: vec![(3, 1), (4, 1), (5, 1), (6, 1)],
                origin: (4.5, 1.5),
                color: Color::Cyan,
                variant,
            },
            TetrominoVariant::J => Tetromino {
                shape: vec![(4, 1), (5, 1), (6, 1), (6, 0)],
                origin: (5.0, 1.0),
                color: Color::Blue,
                variant,
            },
            TetrominoVariant::L => Tetromino {
                shape: vec![(4, 0), (5, 0), (6, 0), (6, 1)],
                origin: (5.0, 0.0),
                color: Color::White,
                variant,
            },
            TetrominoVariant::O => Tetromino {
                shape: vec![(4, 0), (4, 1), (5, 0), (5, 1)],
                origin: (4.5, 0.5),
                color: Color::Yellow,
                variant,
            },
            TetrominoVariant::S => Tetromino {
                shape: vec![(4, 1), (5, 1), (5, 0), (6, 0)],
                origin: (5.0, 1.0),
                color: Color::Green,
                variant,
            },
            TetrominoVariant::T => Tetromino {
                shape: vec![(4, 1), (5, 1), (5, 0), (6, 1)],
                origin: (5.0, 1.0),
                color: Color::Magenta,
                variant,
            },
            TetrominoVariant::Z => Tetromino {
                shape: vec![(4, 0), (5, 0), (5, 1), (6, 1)],
                origin: (5.0, 1.0),
                color: Color::Red,
                variant,
            },
        }
    }
}

fn rand_bag_gen() -> Vec<Tetromino> {
    let mut bag = TetrominoVariant::iter().map(|variant| Tetromino::new(variant)).collect::<Vec<Tetromino>>();
    bag.shuffle(&mut thread_rng());
    bag
}

struct Game {
    falling: Tetromino,
    holding: Option<Tetromino>,
    ghost: Option<Tetromino>,
    next: Tetromino,
    bag: Vec<Tetromino>,
    stack: Vec<Vec<Option<Color>>>,
    start_level: u32,
    score: u32,
    level: u32,
    lines: u32,
    num_lock_resets: u32,
    can_hold: bool,
    placing: bool,
    end: bool,
}

impl Game {
    fn start(start_level: u32) -> Self {
        let mut bag = rand_bag_gen();
        let mut game = Game {
            falling: bag.pop().unwrap(),
            holding: None,
            ghost: None,
            next: bag.pop().unwrap(),
            bag,
            stack: vec![vec![None; BOARD_DIMENSION.0 as usize]; BOARD_DIMENSION.1 as usize],
            start_level,
            score: 0,
            level: start_level,
            lines: 0,
            num_lock_resets: 0,
            can_hold: true,
            placing: false,
            end: false,
        };
        game.update_ghost();
        game
    }

    fn tick(&mut self) {
        if self.placing { return }
        let mut num_cleared = 0;
        for (i, row) in self.stack.clone().iter().enumerate() {
            if row.iter().all(|block| block.is_some()) {
                self.stack.remove(i);
                self.stack.insert(0, vec![None; BOARD_DIMENSION.0 as usize]);
                num_cleared += 1;
            }
        }
        self.lines += num_cleared;
        if num_cleared > 0 {
            self.update_ghost();
            self.level = self.start_level + self.lines / 10;
        }
        self.score += if self.stack.iter().all(|row| row.iter().all(|block| block.is_none())) {
            match num_cleared {
                1 => self.level * 800,
                2 => self.level * 1200,
                3 => self.level * 1800,
                4 => self.level * 2000,
                _ => 0,
            }
        } else {
            match num_cleared {
                1 => self.level * 100,
                2 => self.level * 300,
                3 => self.level * 500,
                4 => self.level * 800,
                _ => 0,
            }
        };
        self.shift(Direction::Down);
    }

    fn get_next(&mut self) -> Tetromino {
        let next = self.next.clone();
        if self.bag.is_empty() { self.bag = rand_bag_gen() }
        self.next = self.bag.pop().unwrap();
        next
    }

    fn hitting_bottom(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.1 == BOARD_DIMENSION.1 - 1 || self.stack[(position.1 + 1) as usize][position.0 as usize].is_some()
        })
    }

    fn update_ghost(&mut self) {
        let mut ghost = self.falling.clone();
        while !self.hitting_bottom(&ghost) {
            for position in ghost.shape.iter_mut() {
                position.1 += 1;
            }
        }
        self.ghost = Some(ghost);
    }

    fn reset_lock(&mut self) {
        if self.placing {
            self.num_lock_resets += 1;
            self.placing = false;
        }
        debug_print!("num_locks_reset: {}", self.num_lock_resets);
    }

    fn shift(&mut self, direction: Direction) {
        match direction {
            Direction::Left => {
                if self.falling.shape.iter().all(|position| position.0 > 0
                && (position.1.is_negative() || self.stack[position.1 as usize][(position.0 - 1) as usize].is_none())) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 -= 1;
                    }
                    self.falling.origin.0 -= 1.0;
                    self.reset_lock();
                }
            },
            Direction::Right => {
                if self.falling.shape.iter().all(|position| position.0 < BOARD_DIMENSION.0 - 1
                && (position.1.is_negative() || self.stack[position.1 as usize][(position.0 + 1) as usize].is_none())) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 += 1;
                    }
                    self.falling.origin.0 += 1.0;
                    self.reset_lock();
                }
            },
            Direction::Down => {
                if !self.hitting_bottom(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.1 += 1;
                    }
                    self.falling.origin.1 += 1.0;
                } else {
                    self.placing = true;
                }
            },
        }
        self.update_ghost();
    }

    fn rotate(&mut self) {
        let mut rotated = Vec::new();
        let angle = f32::from(90.0).to_radians();
        for position in self.falling.shape.iter() {
            let x = position.0 as f32 - self.falling.origin.0;
            let y = position.1 as f32 - self.falling.origin.1;
            let xp = (x * angle.cos() - y * angle.sin()) + self.falling.origin.0;
            let yp = (x * angle.sin() + y * angle.cos()) + self.falling.origin.1;
            rotated.push((xp.round() as i32, yp.round() as i32));
        }
        while rotated.iter().any(|position| position.0 < 0) {
            for position in rotated.iter_mut() {
                position.0 += 1;
            }
        }
        while rotated.iter().any(|position| position.0 > BOARD_DIMENSION.0 - 1) {
            for position in rotated.iter_mut() {
                position.0 -= 1;
            }
        }
        self.falling.shape = rotated;
        if !self.hitting_bottom(&self.falling) {
            self.placing = false;
        }
        self.update_ghost();
    }

    fn place(&mut self) {
        for position in self.falling.shape.iter() {
            if position.1.is_negative() {
                self.end = true;
                return
            }
            self.stack[position.1 as usize][position.0 as usize] = Some(self.falling.color);
        }
        let mut falling = self.get_next();
        for i in 0..2 {
            if self.stack[i].iter().any(|block| block.is_some()) {
                for position in falling.shape.iter_mut() {
                    position.1 -= 1;
                }
            }
        }
        self.falling = falling;
        self.can_hold = true;
        self.update_ghost();
    }

    fn soft_drop(&mut self) {
        self.shift(Direction::Down);
        if !self.hitting_bottom(&self.falling) {
            self.score += 1;
        }
    }

    fn hard_drop(&mut self) {
        while !self.hitting_bottom(&self.falling) {
            for position in self.falling.shape.iter_mut() {
                position.1 += 1;
                self.score += 2;
            }
        }
        self.place();
    }

    fn hold(&mut self) {
        if self.can_hold {
            let swap = self.holding.clone().unwrap_or_else(|| self.get_next());
            self.holding = Some(Tetromino::new(self.falling.variant));
            self.falling = swap;
            self.can_hold = false;
            self.update_ghost();
        }
    }
}

const WIDTH: u16 = BOARD_DIMENSION.0 as u16 * 2 + 2;
const HEIGHT: u16 = BOARD_DIMENSION.1 as u16 + 2;

fn render(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    for x in 1..WIDTH - 1 {
        for y in 1..HEIGHT - 1 {
            stdout
                .queue(MoveTo(x, y))?
                .queue(PrintStyledContent((|| {
                    for position in game.falling.shape.iter() {
                        if !position.1.is_negative() && (position.1 + 1) as u16 == y
                        && ((position.0 + 1) as u16 * 2 == x || (position.0 + 1) as u16 * 2 - 1 == x) {
                            return if game.placing { "▓".with(game.falling.color) } else { " ".on(game.falling.color) }
                        }
                    }
                    for position in game.ghost.as_ref().unwrap().shape.iter() {
                        if !position.1.is_negative() && (position.1 + 1) as u16 == y
                        && ((position.0 + 1) as u16 * 2 == x || (position.0 + 1) as u16 * 2 - 1 == x) {
                            return "░".with(game.falling.color)
                        }
                    }
                    for (i, row) in game.stack.iter().enumerate() {
                        for (j, color) in row.iter().enumerate() {
                            if color.is_some() && (i + 1) as u16 == y && ((j + 1) as u16 * 2 == x || (j + 1) as u16 * 2 - 1 == x) {
                                return " ".on(color.unwrap())
                            }
                        }
                    }
                    StyledContent::new(ContentStyle::new(), if x % 2 == 0 { "." } else { " " })
                })()))?;
        }
    }
    stdout
        .queue(MoveTo(WIDTH + 1, 4))?
        .queue(Print("        "))?
        .queue(MoveTo(WIDTH + 1, 5))?
        .queue(Print("        "))?;
    for position in game.next.shape.iter() {
        stdout
            .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 2, position.1 as u16 + 4))?
            .queue(PrintStyledContent(" ".on(game.next.color)))?
            .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 1, position.1 as u16 + 4))?
            .queue(PrintStyledContent(" ".on(game.next.color)))?;
    }
    if game.holding.is_some() {
        stdout
            .queue(MoveTo(WIDTH + 1, 9))?
            .queue(Print("        "))?
            .queue(MoveTo(WIDTH + 1, 10))?
            .queue(Print("        "))?;
        for position in game.holding.as_ref().unwrap().shape.iter() {
            stdout
                .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 2, position.1 as u16 + 9))?
                .queue(PrintStyledContent(" ".on(game.holding.as_ref().unwrap().color)))?
                .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 1, position.1 as u16 + 9))?
                .queue(PrintStyledContent(" ".on(game.holding.as_ref().unwrap().color)))?;
        }
    }
    stdout
        .queue(MoveTo(WIDTH + 1, 12))?
        .queue(Print(format!("SCORE: {}", game.score)))?
        .queue(MoveTo(WIDTH + 1, 13))?
        .queue(Print(format!("LEVEL: {}", game.level)))?
        .queue(MoveTo(WIDTH + 1, 14))?
        .queue(Print(format!("LINES: {}", game.lines)))?
        .queue(MoveTo(0, 0))?
        .flush()?;

    Ok(())
}

fn draw(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            stdout
                .queue(MoveTo(x, y))?
                .queue(PrintStyledContent(StyledContent::new(ContentStyle::new(),
                    if x == 0 && y == 0 {
                        "╔"
                    } else if x == 0 && y == HEIGHT - 1 {
                        "╚"
                    } else if x == WIDTH - 1 && y == 0 {
                        "╗"
                    } else if x == WIDTH -1 &&  y == HEIGHT - 1 {
                        "╝"
                    } else if x == 0 || x == WIDTH - 1 {
                        "║"
                    } else if y == 0 || y == HEIGHT - 1 {
                        "═"
                    } else if x % 2 == 0 {
                        "."
                    } else {
                        " "
                    }
                )))?;
        }
    }
    stdout
        .queue(MoveTo(8, 0))?
        .queue(PrintStyledContent("TETRIS".bold()))?
        .queue(MoveTo(WIDTH + 1, 2))?
        .queue(Print("NEXT:"))?
        .queue(MoveTo(WIDTH + 1, 7))?
        .queue(Print("HOLDING:"))?
        .queue(MoveTo(0, 0))?
        .flush()?;

    render(game)?;

    Ok(())
}

fn main() -> Result<()> {
    let mut stdout = stdout();

    let debug_window = DebugWindow::new();

    let args = env::args().collect::<Vec<String>>();
    let level = if args.len() == 2 { args[1].parse::<u32>().unwrap() } else { 1 };

    enable_raw_mode()?;

    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

    let game = &mut Game::start(level);

    draw(game)?;

    let tick_frequency = game.level * 2;

    let tick_duration = Duration::from_secs(1) / tick_frequency;
    let lock_delay_duration = Duration::from_millis(500);

    let mut tick_start = Instant::now();
    let mut lock_delay_start: Option<Instant> = None;

    macro_rules! quit {
        () => {{
            debug_window.close();
            disable_raw_mode()?;
            execute!(stdout, Show, Clear(ClearType::All))?;
            println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);
            break
        }};
    }

    Ok(loop {
        if game.end { quit!() }

        if game.placing {
            match lock_delay_start {
                Some(remaining_duration) => {
                    if lock_delay_duration.checked_sub(remaining_duration.elapsed()).is_none() || game.num_lock_resets == 15 {
                        game.place();
                        game.placing = false;
                        game.num_lock_resets = 0;
                    }
                },
                None => lock_delay_start = Some(Instant::now()),
            }
        } else {
            lock_delay_start = None;
        }

        match tick_duration.checked_sub(tick_start.elapsed()) {
            Some(remaining_duration) => {
                if poll(remaining_duration)? {
                    if let Event::Key(event) = read()? {
                        match event.code {
                            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => game.rotate(),
                            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => game.shift(Direction::Left),
                            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => game.soft_drop(),
                            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => game.shift(Direction::Right),
                            KeyCode::Char('c') | KeyCode::Char('C') => game.hold(),
                            KeyCode::Char(' ') => game.hard_drop(),
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => quit!(),
                            _ => continue,
                        };
                    }
                }
            },
            None => {
                game.tick();
                tick_start = Instant::now();
            },
        }
        render(game)?;
        sleep(Duration::from_millis(1));
    })
}
