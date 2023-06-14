use std::{io::{stdout, Write}, time::{Instant, Duration}};
use crossterm::{
    Result, QueueableCommand, execute,
    style::{PrintStyledContent, StyledContent, Color, Stylize, ContentStyle},
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType, enable_raw_mode, disable_raw_mode},
    event::{read, Event, poll, KeyCode},
};
use rand::Rng;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

type Pos = (u16, u16);

const GAME_DIMENSIONS: Pos = (10, 20);

enum Direction { Left, Right }

#[derive(Clone, FromPrimitive)]
enum TetrominoVariant { I, J, L, O, S, T, Z }

#[derive(Clone)]
struct Tetromino {
    shape: Vec<Pos>,
    color: Color,
    variant: TetrominoVariant,
}

impl Tetromino {
    fn new(variant: TetrominoVariant) -> Self {
        match variant {
            TetrominoVariant::I => Tetromino {
                shape: vec![(4, 1), (5, 1), (6, 1), (7, 1)],
                color: Color::Cyan,
                variant,
            },
            TetrominoVariant::J => Tetromino {
                shape: vec![(5, 1), (6, 1), (7, 1), (7, 2)],
                color: Color::Blue,
                variant,
            },
            TetrominoVariant::L => Tetromino {
                shape: vec![(5, 1), (6, 1), (7, 1), (7, 0)],
                color: Color::White,
                variant,
            },
            TetrominoVariant::O => Tetromino {
                shape: vec![(5, 1), (5, 2), (6, 1), (6, 2)],
                color: Color::Yellow,
                variant,
            },
            TetrominoVariant::S => Tetromino {
                shape: vec![(4, 1), (5, 1), (5, 0), (6, 0)],
                color: Color::Green,
                variant,
            },
            TetrominoVariant::T => Tetromino {
                shape: vec![(5, 0), (6, 0), (6, 1), (7, 0)],
                color: Color::Magenta,
                variant,
            },
            TetrominoVariant::Z => Tetromino {
                shape: vec![(4, 0), (5, 0), (5, 1), (6, 1)],
                color: Color::Red,
                variant,
            },
        }
    }
}

fn gen() -> TetrominoVariant {
    TetrominoVariant::from_i32(rand::thread_rng().gen_range(1..7)).unwrap()
}

struct Game {
    falling: Tetromino,
    holding: Option<Tetromino>,
    next: Vec<Tetromino>,
    placed: Vec<Tetromino>,
    score: u32,
    level: u32,
}

impl Game {
    fn start() -> Self {
        Game {
            falling: Tetromino::new(gen()),
            holding: None,
            next: vec![Tetromino::new(gen()), Tetromino::new(gen()), Tetromino::new(gen())],
            placed: Vec::new(),
            score: 0,
            level: 5,
        }
    }

    fn touching(&self) -> bool {
        self.falling.shape.iter().any(|falling| {
            falling.1 + 1 == GAME_DIMENSIONS.1 || self.placed.iter().any(|tetromino| {
                tetromino.shape.iter().any(|placed| {
                    placed.0 == falling.0 && placed.1 == falling.1 + 1
                })
            })
        })
    }

    fn place(&mut self) {
        self.placed.push(self.falling.clone());
        self.falling = self.next.pop().unwrap();
        self.next.push(Tetromino::new(gen()));
    }

    fn tick(&mut self) {
        if self.touching() {
            self.place();
        } else {
            for block in self.falling.shape.iter_mut() {
                block.1 += 1;
            }
        }
    }

    fn shift(&mut self, direction: Direction) {
        if self.touching() { self.place() }
        match direction {
            Direction::Left => {
                if self.falling.shape[0].0 > 0 {
                    for block in self.falling.shape.iter_mut() {
                        block.0 -= 1;
                    }
                }
            },
            Direction::Right => {
                if self.falling.shape[self.falling.shape.len() - 1].0 + 1 < GAME_DIMENSIONS.0 {
                    for block in self.falling.shape.iter_mut() {
                        block.0 += 1;
                    }
                }
            },
        }
    }

    fn rotate(&mut self) {

    }

    fn hold(&mut self) {
        self.holding = Some(Tetromino::new(self.falling.variant.clone()));
        self.falling = self.holding.clone().unwrap_or(Tetromino::new(gen()));
    }
}

fn draw(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    let width = GAME_DIMENSIONS.0 * 2 + 2;
    let height = GAME_DIMENSIONS.1 + 2;

    for x in 0..width {
        for y in 0..height {
            stdout
                .queue(MoveTo(x as u16, y as u16))?
                .queue(PrintStyledContent((|| -> StyledContent<&str> {
                    for block in game.falling.shape.iter() {
                        if ((block.0 + 1) * 2 == x || (block.0 + 1) * 2 - 1 == x) && block.1 + 1 == y {
                            return " ".on(game.falling.color)
                        }
                    }
                    for tetromino in game.placed.iter() {
                        for block in tetromino.shape.iter() {
                            if ((block.0 + 1) * 2 == x || (block.0 + 1) * 2 - 1 == x) && block.1 + 1 == y {
                                return " ".on(tetromino.color)
                            }
                        }
                    }
                    if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                        " ".on(Color::Blue)
                    } else {
                        StyledContent::new(ContentStyle::new(), " ")
                    }
                })()))?;
        }
    }
    stdout.flush()?;

    Ok(())
}

fn main() -> Result<()> {
    let mut stdout = stdout();

    enable_raw_mode()?;

    execute!(stdout, Hide, Clear(ClearType::All))?;

    macro_rules! quit {
        ($msg:expr) => {{
            disable_raw_mode()?;
            execute!(stdout, Show, Clear(ClearType::All))?;
            println!("{}", $msg);
            break
        }};
    }

    let game = &mut Game::start();

    let loop_frequency = game.level;
    let sleep_duration = Duration::from_secs(1) / loop_frequency;

    Ok(loop {
        let loop_start = Instant::now();

        draw(game)?;

        let work_duration = loop_start.elapsed();

        if let Some(remaining_duration) = sleep_duration.checked_sub(work_duration) {
            if poll(remaining_duration)? {
                match read()? {
                    Event::Key(event) => {
                        match event.code {
                            KeyCode::Char('w') | KeyCode::Up => game.rotate(),
                            KeyCode::Char('a') | KeyCode::Left => game.shift(Direction::Left),
                            KeyCode::Char('d') | KeyCode::Right => game.shift(Direction::Right),
                            KeyCode::Char('c') => game.hold(),
                            KeyCode::Char('q') | KeyCode::Esc => quit!(""),
                            _ => continue,
                        };
                    },
                    _ => continue,
                }
            }
        }
        game.tick();
    })
}
