use std::{io::{stdout, Write}, time::{Instant, Duration}, env::args, thread::sleep, mem::replace};
use crossterm::{
    Result, QueueableCommand, execute,
    style::{PrintStyledContent, StyledContent, Color, Stylize, ContentStyle, Print},
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode},
    event::{Event, KeyCode, read, poll},
};
use rand::{thread_rng, seq::SliceRandom};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::debug::*;

mod debug;

type Dimension = (i32, i32);
type Shape = Vec<Dimension>;

const BOARD_DIMENSION: Dimension = (10, 20);

#[derive(Clone, Copy, FromPrimitive)]
enum ShiftDirection { Left, Right, Down }

#[derive(PartialEq, Debug)]
enum RotationDirection { Clockwise, CounterClockwise }

#[derive(Clone, Copy, FromPrimitive, PartialEq, Debug)]
enum CardinalDirection { North, East, South, West }

#[derive(Clone, Copy, EnumIter, FromPrimitive, PartialEq, Debug)]
enum TetrominoVariant { I, J, L, O, S, T, Z }

#[derive(Clone)]
struct Tetromino {
    shape: Shape,
    center: Dimension,
    direction: CardinalDirection,
    color: Color,
    variant: TetrominoVariant,
}

impl Tetromino {
    fn new(variant: TetrominoVariant) -> Self {
        match variant {
            TetrominoVariant::I => Tetromino {
                shape: vec![(3, 18), (4, 18), (5, 18), (6, 18)],
                center: (4, 18),
                direction: CardinalDirection::North,
                color: Color::Cyan,
                variant,
            },
            TetrominoVariant::J => Tetromino {
                shape: vec![(4, 19), (4, 18), (5, 18), (6, 18)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: Color::Blue,
                variant,
            },
            TetrominoVariant::L => Tetromino {
                shape: vec![(4, 18), (5, 18), (6, 18), (6, 19)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: Color::White,
                variant,
            },
            TetrominoVariant::O => Tetromino {
                shape: vec![(4, 18), (4, 19), (5, 18), (5, 19)],
                center: (4, 18),
                direction: CardinalDirection::North,
                color: Color::Yellow,
                variant,
            },
            TetrominoVariant::S => Tetromino {
                shape: vec![(4, 18), (5, 18), (5, 19), (6, 19)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: Color::Green,
                variant,
            },
            TetrominoVariant::T => Tetromino {
                shape: vec![(4, 18), (5, 18), (5, 19), (6, 18)],
                center: (5, 18),
                direction: CardinalDirection::North,
                color: Color::Magenta,
                variant,
            },
            TetrominoVariant::Z => Tetromino {
                shape: vec![(4, 19), (5, 19), (5, 18), (6, 18)],
                center: (5, 18),
                direction: CardinalDirection::North,
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
    can_hold: bool,
    locking: bool,
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
            can_hold: true,
            locking: false,
            end: false,
        };
        game.update_ghost();
        game
    }

    fn clear(&mut self) -> u32 {
        let old_stack = replace(&mut self.stack, Vec::new());
        let mut num_cleared = 0;
        for row in old_stack.into_iter() {
            if row.iter().all(|block| block.is_some()) {
                num_cleared += 1;
            } else {
                self.stack.push(row);
            }
        }
        for _ in 0..num_cleared {
            self.stack.push(vec![None; BOARD_DIMENSION.0 as usize]);
        }
        num_cleared
    }

    fn tick(&mut self) {
        if self.locking { return }
        let num_cleared = self.clear();
        if num_cleared > 0 {
            self.lines += num_cleared;
            self.level = self.start_level + self.lines / 10;
            self.update_ghost();
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
        self.shift(ShiftDirection::Down);
    }

    fn get_next(&mut self) -> Tetromino {
        let next = self.next.clone();
        if self.bag.is_empty() { self.bag = rand_bag_gen() }
        self.next = self.bag.pop().unwrap();
        next
    }

    fn hitting_bottom(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.1 == 0 ||
            position.1 < BOARD_DIMENSION.1 &&
            self.stack[(position.1 - 1) as usize][position.0 as usize].is_some()
        })
    }

    fn hitting_left(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.0 == 0 ||
            position.1 < BOARD_DIMENSION.1 &&
            self.stack[position.1 as usize][(position.0 - 1) as usize].is_some()
        })
    }

    fn hitting_right(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.0 == BOARD_DIMENSION.0 - 1 ||
            position.1 < BOARD_DIMENSION.1 &&
            self.stack[position.1 as usize][(position.0 + 1) as usize].is_some()
        })
    }

    fn update_ghost(&mut self) {
        let mut ghost = self.falling.clone();
        while !self.hitting_bottom(&ghost) {
            for position in ghost.shape.iter_mut() {
                position.1 -= 1;
            }
        }
        self.ghost = if self.overlapping(&ghost.shape) { None } else { Some(ghost) };
    }

    fn shift(&mut self, direction: ShiftDirection) {
        match direction {
            ShiftDirection::Left => {
                if !self.hitting_left(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 -= 1;
                    }
                    self.falling.center.0 -= 1;
                    self.locking = false;
                }
            },
            ShiftDirection::Right => {
                if !self.hitting_right(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 += 1;
                    }
                    self.falling.center.0 += 1;
                    self.locking = false;
                }
            },
            ShiftDirection::Down => {
                if !self.hitting_bottom(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.1 -= 1;
                    }
                    self.falling.center.1 -= 1;
                } else {
                    self.locking = true;
                }
            },
        }
        self.update_ghost();
    }

    fn overlapping(&self, shape: &Shape) -> bool {
        shape.iter().any(|position| {
            position.0 < 0 ||
            position.1 < 0 ||
            position.0 > BOARD_DIMENSION.0 - 1 ||
            position.1 > BOARD_DIMENSION.1 - 1 ||
            self.stack[position.1 as usize][position.0 as usize].is_some()
        })
    }

    fn rotate(&mut self, direction: RotationDirection) {
        let mut rotated = Vec::new();
        let (angle, new_direction) = match direction {
            RotationDirection::Clockwise => (
                f32::from(-90.0).to_radians(),
                CardinalDirection::from_i32((self.falling.direction as i32 + 1) % 4).unwrap(),
            ),
            RotationDirection::CounterClockwise => (
                f32::from(90.0).to_radians(),
                CardinalDirection::from_i32(((self.falling.direction as i32 - 1) % 4 + 4) % 4).unwrap(),
            ),
        };
        for position in self.falling.shape.iter() {
            let x = (position.0 - self.falling.center.0) as f32;
            let y = (position.1 - self.falling.center.1) as f32;
            rotated.push((
                ((x * angle.cos() - y * angle.sin()) + self.falling.center.0 as f32).round() as i32,
                ((x * angle.sin() + y * angle.cos()) + self.falling.center.1 as f32).round() as i32,
            ));
        }
        let offset_data = match self.falling.variant {
            TetrominoVariant::J | TetrominoVariant::L | TetrominoVariant::S | TetrominoVariant::T | TetrominoVariant::Z => vec![
                vec![( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)], // North
                vec![( 0,  0), ( 1,  0), ( 1, -1), ( 0,  2), ( 1,  2)], // East
                vec![( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  0)], // South
                vec![( 0,  0), (-1,  0), (-1, -1), ( 0,  2), (-1,  2)], // West
            ],
            TetrominoVariant::I => vec![
                vec![( 0,  0), (-1,  0), ( 2,  0), (-1,  0), ( 2,  0)],
                vec![(-1,  0), ( 0,  0), ( 0,  0), ( 0,  1), ( 0, -2)],
                vec![(-1,  1), ( 1,  1), (-2,  1), ( 1,  0), (-2,  0)],
                vec![( 0,  1), ( 0,  1), ( 0,  1), ( 0, -1), ( 0,  2)],
            ],
            TetrominoVariant::O => vec![
                vec![( 0,  0)],
                vec![( 0, -1)],
                vec![(-1, -1)],
                vec![(-1,  0)],
            ],
        };
        for i in 0..offset_data[0].len() {
            let offset_x = offset_data[new_direction as usize][i].0 - offset_data[self.falling.direction as usize][i].0;
            let offset_y = offset_data[new_direction as usize][i].1 - offset_data[self.falling.direction as usize][i].1;
            let mut kicked = rotated.clone();
            for position in kicked.iter_mut() {
                position.0 -= offset_x;
                position.1 -= offset_y;
            }
            if !self.overlapping(&kicked) {
                self.falling.shape = kicked;
                self.falling.center.0 -= offset_x;
                self.falling.center.1 -= offset_y;
                self.falling.direction = new_direction;
                self.locking = false;
                self.update_ghost();
                return
            }
        }
    }

    fn lock(&mut self) {
        for position in self.falling.shape.iter() {
            if position.1 > BOARD_DIMENSION.1 - 1 {
                self.end = true;
                return
            }
            self.stack[position.1 as usize][position.0 as usize] = Some(self.falling.color);
        }
        let mut falling = self.get_next();
        for i in 17..20 {
            if self.stack[i].iter().any(|block| block.is_some()) {
                for position in falling.shape.iter_mut() {
                    position.1 += 1;
                }
                falling.center.1 += 1;
            }
        }
        self.falling = falling;
        self.can_hold = true;
        self.update_ghost();
    }

    fn soft_drop(&mut self) {
        self.shift(ShiftDirection::Down);
        if !self.hitting_bottom(&self.falling) {
            self.score += 1;
        }
    }

    fn hard_drop(&mut self) {
        while !self.hitting_bottom(&self.falling) {
            for position in self.falling.shape.iter_mut() {
                position.1 -= 1;
                self.score += 2;
            }
        }
        self.lock();
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

    fn position_in_view(position: &Dimension, view: &Dimension) -> bool {
        BOARD_DIMENSION.1 - position.1 == view.1 && ((position.0 + 1) * 2 == view.0 || (position.0 + 1) * 2 - 1 == view.0)
    }

    for x in 1..WIDTH as i32 - 1 {
        for y in 1..HEIGHT as i32 - 1 {
            stdout
                .queue(MoveTo(x as u16, y as u16))?
                .queue(PrintStyledContent((|| {
                    for position in game.falling.shape.iter() {
                        if position_in_view(position, &(x, y)) {
                            return if game.locking { "▓".with(game.falling.color) } else { " ".on(game.falling.color) }
                        }
                    }
                    if let Some(ghost) = &game.ghost {
                        for position in ghost.shape.iter() {
                            if position_in_view(position, &(x, y)) {
                                return "░".with(game.falling.color)
                            }
                        }
                    }
                    for (i, row) in game.stack.iter().enumerate() {
                        for (j, color) in row.iter().enumerate() {
                            if let Some(color) = color {
                                if position_in_view(&(j as i32, i as i32), &(x, y)) {
                                    return " ".on(*color)
                                }
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
    for position in game.next.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
        stdout
            .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 2, HEIGHT - position.1 + 1))?
            .queue(PrintStyledContent(" ".on(game.next.color)))?
            .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 1, HEIGHT - position.1 + 1))?
            .queue(PrintStyledContent(" ".on(game.next.color)))?;
    }
    if let Some(holding) = &game.holding {
        stdout
            .queue(MoveTo(WIDTH + 1, 9))?
            .queue(Print("        "))?
            .queue(MoveTo(WIDTH + 1, 10))?
            .queue(Print("        "))?;
        for position in holding.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
            stdout
                .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 2, HEIGHT - position.1 + 6))?
                .queue(PrintStyledContent(" ".on(game.holding.as_ref().unwrap().color)))?
                .queue(MoveTo((position.0 - 3) * 2 + WIDTH + 1, HEIGHT - position.1 + 6))?
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

    // let debug_window = DebugWindow::new();

    let args = args().collect::<Vec<String>>();
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
            // debug_window.close();
            disable_raw_mode()?;
            execute!(stdout, Show, Clear(ClearType::All))?;
            println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);
            break
        }};
    }

    Ok(loop {
        if game.end { quit!() }

        if game.locking {
            match lock_delay_start {
                Some(remaining_duration) => {
                    if lock_delay_duration.checked_sub(remaining_duration.elapsed()).is_none() {
                        game.lock();
                        game.locking = false;
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
                            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => game.rotate(RotationDirection::Clockwise),
                            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => game.shift(ShiftDirection::Left),
                            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => game.soft_drop(),
                            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => game.shift(ShiftDirection::Right),
                            KeyCode::Char('z') | KeyCode::Char('Z') => game.rotate(RotationDirection::CounterClockwise),
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
