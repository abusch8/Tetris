use std::{io::{stdout, Write}, time::{Instant, Duration}, env};
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

type Dimension = (usize, usize);
type Origin = (f32, f32);

const GAME_DIMENSIONS: Dimension = (10, 20);

#[derive(Clone, Copy, FromPrimitive)]
enum Direction { Left, Right }

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
                shape: vec![(3, 1), (4, 1), (5, 1), (6, 1)],
                origin: (4.5, 1.5),
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
    placed: Vec<Vec<Option<Color>>>,
    score: u32,
    level: u32,
    lines: u32,
}

impl Game {
    fn start(level: u32) -> Self {
        Game {
            falling: Tetromino::new(gen()),
            holding: None,
            can_hold: true,
            next: vec![Tetromino::new(gen()); 3],
            placed: vec![vec![None; GAME_DIMENSIONS.0]; GAME_DIMENSIONS.1],
            score: 0,
            level,
            lines: 0,
        }
    }

    fn touching(&self) -> bool {
        self.falling.shape.iter().any(|position| {
            position.1 + 1 == GAME_DIMENSIONS.1 || self.placed.iter().enumerate().any(|(i, row)| {
                row.iter().enumerate().any(|(j, block)| block.is_some() && j == position.0 && i == position.1 + 1)
            })
        })
    }

    fn place(&mut self) {
        for position in self.falling.shape.iter() {
            self.placed[position.1][position.0] = Some(self.falling.color);
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
        for (i, row) in self.placed.clone().iter().enumerate() {
            if row.iter().all(|block| block.is_some()) {
                self.placed.remove(i);
                self.placed.insert(0, vec![None; GAME_DIMENSIONS.0]);
                self.lines += 1;
            }
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
                if self.falling.shape.iter().all(|position| position.0 > 0) {
                    for position in self.falling.shape.iter_mut() {
                        position.0 -= 1;
                    }
                    self.falling.origin.0 -= 1.0;
                }
            },
            Direction::Right => {
                if self.falling.shape.iter().all(|position| position.0 < GAME_DIMENSIONS.0 - 1) {
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
            rotated.push((xp.round() as usize, yp.round() as usize));
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

const WIDTH: u16 = GAME_DIMENSIONS.0 as u16 * 2 + 2;
const HEIGHT: u16 = GAME_DIMENSIONS.1 as u16 + 2;

fn render_board(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    for x in 1..WIDTH - 1 {
        for y in 1..HEIGHT - 1 {
            stdout
                .queue(MoveTo(x, y))?
                .queue(PrintStyledContent((|| {
                    for position in game.falling.shape.iter() {
                        if ((position.0 as u16 + 1) * 2 == x || (position.0 as u16 + 1) * 2 - 1 == x) && position.1 as u16 + 1 == y {
                            return " ".on(game.falling.color)
                        }
                    }
                    for (i, row) in game.placed.iter().enumerate() {
                        for (j, color) in row.iter().enumerate() {
                            if color.is_some() && ((j as u16 + 1) * 2 == x || (j as u16 + 1) * 2 - 1 == x) && i as u16 + 1 == y {
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
    for position in game.next[game.next.len() - 1].shape.iter() {
        stdout
            .queue(MoveTo((position.0 as u16 - 3) * 2 + WIDTH + 2, position.1 as u16 + 4))?
            .queue(PrintStyledContent(" ".on(game.next[game.next.len() - 1].color)))?
            .queue(MoveTo((position.0 as u16 - 3) * 2 + WIDTH + 1, position.1 as u16 + 4))?
            .queue(PrintStyledContent(" ".on(game.next[game.next.len() - 1].color)))?;
    }
    if game.holding.is_some() {
        stdout
            .queue(MoveTo(WIDTH + 1, 9))?
            .queue(Print("        "))?
            .queue(MoveTo(WIDTH + 1, 10))?
            .queue(Print("        "))?;
        for position in game.holding.as_ref().unwrap().shape.iter() {
            stdout
                .queue(MoveTo((position.0 as u16 - 3) * 2 + WIDTH + 2, position.1 as u16 + 9))?
                .queue(PrintStyledContent(" ".on(game.holding.as_ref().unwrap().color)))?
                .queue(MoveTo((position.0 as u16 - 3) * 2 + WIDTH + 1, position.1 as u16 + 9))?
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

fn render_menu(game: &Game) -> Result<()> {
    let mut stdout = stdout();

    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            stdout
                .queue(MoveTo(x, y))?
                .queue(PrintStyledContent((|| {
                    StyledContent::new(ContentStyle::new(),
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
                    )
                })()))?;
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

    render_menu(game)?;

    let loop_frequency = game.level;
    let sleep_duration = Duration::from_secs(1) / loop_frequency;

    Ok(loop {
        let loop_start = Instant::now();

        render_board(game)?;

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
            game.tick();
        }
    })
}
