use std::{io::{stdout, Write}, time::{Instant, Duration}};
use crossterm::{
    Result, QueueableCommand, execute,
    style::{PrintStyledContent, StyledContent, Color, Stylize, ContentStyle, Print},
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType, enable_raw_mode, disable_raw_mode, SetTitle},
    event::{read, Event, poll, KeyCode},
};
use rand::Rng;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

type Position = (u16, u16);
type Origin = (f32, f32);

const GAME_DIMENSIONS: Position = (10, 20);

#[derive(Clone, Copy, FromPrimitive)]
enum Direction { Left, Right }

#[derive(Clone, Copy, FromPrimitive, PartialEq)]
enum TetrominoVariant { I, J, L, O, S, T, Z }

struct PlacedTetromino {
    position: Position,
    color: Color,
}

#[derive(Clone)]
struct Tetromino {
    shape: Vec<Position>,
    origin: Origin,
    color: Color,
    variant: TetrominoVariant,
}

impl Tetromino {
    fn new(variant: TetrominoVariant) -> Self {
        match variant {
            TetrominoVariant::I => Tetromino {
                shape: vec![(3, 1), (4, 1), (5, 1), (6, 1)],
                origin: (4.5, 1.0),
                color: Color::Cyan,
                variant,
            },
            TetrominoVariant::J => Tetromino {
                shape: vec![(4, 0), (5, 0), (6, 0), (6, 1)],
                origin: (5.0, 0.0),
                color: Color::Blue,
                variant,
            },
            TetrominoVariant::L => Tetromino {
                shape: vec![(4, 1), (5, 1), (6, 1), (6, 0)],
                origin: (5.0, 1.0),
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
                shape: vec![(4, 0), (5, 0), (5, 1), (6, 0)],
                origin: (5.0, 0.0),
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

fn gen() -> TetrominoVariant {
    TetrominoVariant::from_i32(rand::thread_rng().gen_range(0..7)).unwrap()
}

struct Game {
    falling: Tetromino,
    holding: Option<Tetromino>,
    can_hold: bool,
    next: Vec<Tetromino>,
    placed: Vec<PlacedTetromino>,
    score: u32,
    level: u32,
    lines: u32,
}

impl Game {
    fn start() -> Self {
        Game {
            falling: Tetromino::new(gen()),
            holding: None,
            can_hold: true,
            next: vec![Tetromino::new(gen()), Tetromino::new(gen()), Tetromino::new(gen())],
            placed: Vec::new(),
            score: 0,
            level: 5,
            lines: 0,
        }
    }

    fn touching(&self) -> bool {
        self.falling.shape.iter().any(|falling| {
            falling.1 + 1 == GAME_DIMENSIONS.1 || self.placed.iter().any(|block| {
                block.position.0 == falling.0 && block.position.1 == falling.1 + 1
            })
        })
    }

    fn place(&mut self) {
        for position in self.falling.shape.iter() {
            self.placed.push(PlacedTetromino { position: *position, color: self.falling.color });
        }
        self.falling = self.next.pop().unwrap();
        self.next.push(Tetromino::new(gen()));
        self.can_hold = true;
    }

    fn tick(&mut self) {
        if self.touching() {
            self.place();
            return
        }
        for position in self.falling.shape.iter_mut() {
            position.1 += 1;
        }
        self.falling.origin.1 += 1.0;
    }

    fn shift(&mut self, direction: Direction) {
        if self.touching() {
            self.place();
            return
        }
        match direction {
            Direction::Left => {
                if self.falling.shape[0].0 > 0 {
                    for position in self.falling.shape.iter_mut() {
                        position.0 -= 1;
                    }
                    self.falling.origin.0 -= 1.0;
                }
            },
            Direction::Right => {
                if self.falling.shape[self.falling.shape.len() - 1].0 < GAME_DIMENSIONS.0 - 1 {
                    for position in self.falling.shape.iter_mut() {
                        position.0 += 1;
                    }
                    self.falling.origin.0 += 1.0;
                }
            },
        }
    }

    fn drop(&mut self) {

    }

    fn rotate(&mut self) {
        let mut rotated = Vec::new();
        let angle = f32::from(90.0).to_radians();
        for position in self.falling.shape.iter() {
            let x = position.0 as f32 - self.falling.origin.0;
            let y = position.1 as f32 - self.falling.origin.1;
            let mut xp = x * angle.cos() - y * angle.sin();
            let mut yp = x * angle.sin() + y * angle.cos();
            xp += self.falling.origin.0;
            yp += self.falling.origin.1;
            rotated.push((xp.round() as u16, yp.round() as u16));
        }
        self.falling.shape = rotated;
    }

    fn hold(&mut self) {
        if self.can_hold {
            let swap = self.holding.clone().unwrap_or(Tetromino::new(gen()));
            self.holding = Some(Tetromino::new(self.falling.variant));
            self.falling = swap;
            self.can_hold = false;
        }
    }
}

fn draw(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    let width = GAME_DIMENSIONS.0 * 2 + 2;
    let height = GAME_DIMENSIONS.1 + 2;

    for x in 0..width {
        for y in 0..height {
            stdout
                .queue(MoveTo(x, y))?
                .queue(PrintStyledContent((|| {
                    for position in game.falling.shape.iter() {
                        if ((position.0 + 1) * 2 == x || (position.0 + 1) * 2 - 1 == x) && position.1 + 1 == y {
                            return " ".on(game.falling.color)
                        }
                    }
                    for block in game.placed.iter() {
                        if ((block.position.0 + 1) * 2 == x || (block.position.0 + 1) * 2 - 1 == x) && block.position.1 + 1 == y {
                            return " ".on(block.color)
                        }
                    }
                    StyledContent::new(ContentStyle::new(),
                    if x == 0 && y == 0 {
                        "╔"
                    } else if x == 0 && y == height - 1 {
                        "╚"
                    } else if x == width - 1 && y == 0 {
                        "╗"
                    } else if x == width -1 &&  y == height - 1 {
                        "╝"
                    } else if x == 0 || x == width - 1 {
                        "║"
                    } else if y == 0 || y == height - 1 {
                        "═"
                    } else if x % 2 == 0 {
                        "."
                    } else {
                        " "
                    })
                })()))?;
        }
    }
    stdout
        .queue(MoveTo(width + 1, 4))?
        .queue(Print("        "))?
        .queue(MoveTo(width + 1, 5))?
        .queue(Print("        "))?;
    for position in game.next[game.next.len() - 1].shape.iter() {
        stdout
            .queue(MoveTo((position.0 - 3) * 2 + width + 2, position.1 + 4))?
            .queue(PrintStyledContent(" ".on(game.next[game.next.len() - 1].color)))?
            .queue(MoveTo((position.0 - 3) * 2 + width + 1, position.1 + 4))?
            .queue(PrintStyledContent(" ".on(game.next[game.next.len() - 1].color)))?;
    }
    if game.holding.is_some() {
        stdout
            .queue(MoveTo(width + 1, 9))?
            .queue(Print("        "))?
            .queue(MoveTo(width + 1, 10))?
            .queue(Print("        "))?;
        for position in game.holding.as_ref().unwrap().shape.iter() {
            stdout
                .queue(MoveTo((position.0 - 3) * 2 + width + 2, position.1 + 9))?
                .queue(PrintStyledContent(" ".on(game.holding.as_ref().unwrap().color)))?
                .queue(MoveTo((position.0 - 3) * 2 + width + 1, position.1 + 9))?
                .queue(PrintStyledContent(" ".on(game.holding.as_ref().unwrap().color)))?;
        }
    }
    stdout
        .queue(MoveTo(8, 0))?
        .queue(PrintStyledContent("TETRIS".bold()))?
        .queue(MoveTo(width + 1, 2))?
        .queue(Print("NEXT:"))?
        .queue(MoveTo(width + 1, 7))?
        .queue(Print("HOLDING:"))?
        .queue(MoveTo(width + 1, 12))?
        .queue(Print(format!("SCORE: {}", game.score)))?
        .queue(MoveTo(width + 1, 13))?
        .queue(Print(format!("LEVEL: {}", game.level)))?
        .queue(MoveTo(width + 1, 14))?
        .queue(Print(format!("LINES: {}", game.lines)))?
        .queue(MoveTo(0, 0))?
        .flush()?;

    Ok(())
}

fn main() -> Result<()> {
    let mut stdout = stdout();

    enable_raw_mode()?;

    execute!(stdout, Hide, Clear(ClearType::All), SetTitle("TETRIS"))?;

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
                            KeyCode::Char(' ') => game.drop(),
                            KeyCode::Char('q') | KeyCode::Esc => {
                                disable_raw_mode()?;
                                execute!(stdout, Show, Clear(ClearType::All))?;
                                println!("SCORE: {}\nLEVEL: {}\nLINES: {}", game.score, game.level, game.lines);
                                break
                            },
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
