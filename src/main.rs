use std::{io::{stdout, Write}, time::{Instant, Duration}, env, thread};
use crossterm::{
    Result, QueueableCommand, execute,
    style::{PrintStyledContent, StyledContent, Color, Stylize, ContentStyle, Print},
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType, SetTitle, enable_raw_mode, disable_raw_mode},
    event::{Event, KeyCode, read, poll},
};
use rand::Rng;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

type Dimension = (i32, i32);
type Origin = (f32, f32);

const BOARD_DIMENSION: Dimension = (10, 20);

#[derive(Clone, Copy, FromPrimitive)]
enum Direction { Left, Right, Down }

#[derive(Clone, Copy, FromPrimitive, PartialEq)]
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
                shape: vec![(3, -1), (4, -1), (5, -1), (6, -1)],
                origin: (4.5, -1.5),
                color: Color::Cyan,
                variant,
            },
            TetrominoVariant::J => Tetromino {
                shape: vec![(4, -2), (5, -2), (6, -2), (6, -1)],
                origin: (5.0, -2.0),
                color: Color::Blue,
                variant,
            },
            TetrominoVariant::L => Tetromino {
                shape: vec![(4, -1), (5, -1), (6, -1), (6, -2)],
                origin: (5.0, -1.0),
                color: Color::White,
                variant,
            },
            TetrominoVariant::O => Tetromino {
                shape: vec![(4, -1), (4, -2), (5, -1), (5, -2)],
                origin: (4.5, -1.5),
                color: Color::Yellow,
                variant,
            },
            TetrominoVariant::S => Tetromino {
                shape: vec![(4, -1), (5, -1), (5, -2), (6, -2)],
                origin: (5.0, -1.0),
                color: Color::Green,
                variant,
            },
            TetrominoVariant::T => Tetromino {
                shape: vec![(4, -1), (5, -1), (5, -2), (6, -1)],
                origin: (5.0, -1.0),
                color: Color::Magenta,
                variant,
            },
            TetrominoVariant::Z => Tetromino {
                shape: vec![(4, -2), (5, -2), (5, -1), (6, -1)],
                origin: (5.0, -2.0),
                color: Color::Red,
                variant,
            },
        }
    }
}

fn gen() -> TetrominoVariant {
    TetrominoVariant::from_i32(rand::thread_rng().gen_range(0..7)).unwrap()
}

struct Game {
    falling: Tetromino,
    holding: Option<Tetromino>,
    ghost: Option<Tetromino>,
    bag: Vec<Tetromino>,
    placed: Vec<Vec<Option<Color>>>,
    score: u32,
    level: u32,
    lines: u32,
    can_hold: bool,
    placing: bool,
    end: bool,
}

impl Game {
    fn start(level: u32) -> Self {
        let mut game = Game {
            falling: Tetromino::new(gen()),
            holding: None,
            ghost: None,
            bag: vec![Tetromino::new(gen()); 8],
            placed: vec![vec![None; BOARD_DIMENSION.0 as usize]; BOARD_DIMENSION.1 as usize],
            score: 0,
            level,
            lines: 0,
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
        for (i, row) in self.placed.clone().iter().enumerate() {
            if row.iter().all(|block| block.is_some()) {
                self.placed.remove(i);
                self.placed.insert(0, vec![None; BOARD_DIMENSION.0 as usize]);
                num_cleared += 1;
            }
        }
        if num_cleared > 0 {
            self.update_ghost();
        }
        self.lines += num_cleared;
        self.score += match num_cleared {
            1 => self.level * 100,
            2 => self.level * 300,
            3 => self.level * 500,
            4 => self.level * 800,
            _ => 0,
        };
        self.shift(Direction::Down);
    }

    fn hitting_bottom(&self, tetromino: &Tetromino) -> bool {
        tetromino.shape.iter().any(|position| {
            position.1 == BOARD_DIMENSION.1 - 1 || self.placed.iter().enumerate().any(|(i, row)| {
                row.iter().enumerate().any(|(j, block)| {
                    block.is_some() && i == (position.1 + 1) as usize && j == position.0 as usize
                })
            })
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

    fn shift(&mut self, direction: Direction) {
        match direction {
            Direction::Left => {
                if self.falling.shape.iter().all(|position| position.0 > 0
                && (position.1.is_negative() || self.placed[position.1 as usize][(position.0 - 1) as usize].is_none())) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 -= 1;
                    }
                    self.falling.origin.0 -= 1.0;
                    self.placing = false;
                }
            },
            Direction::Right => {
                if self.falling.shape.iter().all(|position| position.0 < BOARD_DIMENSION.0 - 1
                && (position.1.is_negative() || self.placed[position.1 as usize][(position.0 + 1) as usize].is_none())) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 += 1;
                    }
                    self.falling.origin.0 += 1.0;
                    self.placing = false;
                }
            },
            Direction::Down => {
                if !self.hitting_bottom(&self.falling) {
                    for position in self.falling.shape.iter_mut() {
                        position.1 += 1;
                    }
                    self.falling.origin.1 += 1.0;
                    self.placing = false;
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
        self.update_ghost();
    }

    fn place(&mut self) {
        for position in self.falling.shape.iter() {
            if position.1.is_negative() {
                self.end = true;
                return
            }
            self.placed[position.1 as usize][position.0 as usize] = Some(self.falling.color);
        }
        self.falling = self.bag.pop().unwrap();
        self.bag.push(Tetromino::new(gen()));
        self.can_hold = true;
        self.update_ghost();
    }

    fn drop(&mut self) {
        while !self.hitting_bottom(&self.falling) {
            for position in self.falling.shape.iter_mut() {
                position.1 += 1;
            }
        }
        self.place();
    }

    fn hold(&mut self) {
        if self.can_hold {
            let swap = self.holding.clone().unwrap_or(Tetromino::new(gen()));
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
                    for (i, row) in game.placed.iter().enumerate() {
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
    for position in game.bag.last().unwrap().shape.iter() {
        stdout
            .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 2, (position.1 + 2) as u16 + 4))?
            .queue(PrintStyledContent(" ".on(game.bag.last().unwrap().color)))?
            .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 1, (position.1 + 2) as u16 + 4))?
            .queue(PrintStyledContent(" ".on(game.bag.last().unwrap().color)))?;
    }
    if game.holding.is_some() {
        stdout
            .queue(MoveTo(WIDTH + 1, 9))?
            .queue(Print("        "))?
            .queue(MoveTo(WIDTH + 1, 10))?
            .queue(Print("        "))?;
        for position in game.holding.as_ref().unwrap().shape.iter() {
            stdout
                .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 2, (position.1 + 2) as u16 + 9))?
                .queue(PrintStyledContent(" ".on(game.holding.as_ref().unwrap().color)))?
                .queue(MoveTo((position.0 - 3) as u16 * 2 + WIDTH + 1, (position.1 + 2) as u16 + 9))?
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

fn main() -> Result<()> {
    let mut stdout = stdout();

    let args = env::args().collect::<Vec<String>>();
    let level = if args.len() == 2 { args[1].parse::<u32>().unwrap() } else { 1 };

    enable_raw_mode()?;

    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

    let game = &mut Game::start(level);

    draw(game)?;

    let tick_frequency = game.level * 2;

    let sleep_duration_main = Duration::from_secs(1) / tick_frequency;
    let sleep_duration_cancel_place = Duration::from_millis(500);

    let mut loop_start_main = Instant::now();
    let mut loop_start_cancel_place: Option<Instant> = None;

    macro_rules! quit {
        () => {{
            disable_raw_mode()?;
            execute!(stdout, Show, Clear(ClearType::All))?;
            println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);
            break
        }};
    }

    Ok(loop {
        if game.end { quit!() }

        if game.placing {
            match loop_start_cancel_place {
                Some(remaining_duration) => {
                    if sleep_duration_cancel_place.checked_sub(remaining_duration.elapsed()).is_none() {
                        game.place();
                        game.placing = false;
                    }
                },
                None => loop_start_cancel_place = Some(Instant::now()),
            }
        } else {
            loop_start_cancel_place = None;
        }

        match sleep_duration_main.checked_sub(loop_start_main.elapsed()) {
            Some(remaining_duration) => {
                if poll(remaining_duration)? {
                    if let Event::Key(event) = read()? {
                        match event.code {
                            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Up => game.rotate(),
                            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Left => game.shift(Direction::Left),
                            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Down => game.shift(Direction::Down),
                            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Right => game.shift(Direction::Right),
                            KeyCode::Char('c') | KeyCode::Char('C') => game.hold(),
                            KeyCode::Char(' ') => game.drop(),
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => quit!(),
                            _ => continue,
                        };
                    }
                }
            },
            None => {
                game.tick();
                loop_start_main = Instant::now();
            },
        }
        render(game)?;
        thread::sleep(Duration::from_millis(1));
    })
}
