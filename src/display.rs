use std::io::{Result, Stdout};
use::std::io::{stdout, Write};
use crossterm::{
    execute, QueueableCommand,
    cursor::MoveTo,
    style::{Color, ContentStyle, Print, PrintStyledContent, StyledContent, Stylize},
    terminal::{self, Clear, ClearType},
};

use crate::{config, game::Game, tetromino::{Tetromino, TetrominoVariant}};

pub type Dimension = (i32, i32);

pub const BOARD_DIMENSION: Dimension = (10, 20);
pub const BOARD_MP_OFFSET: u16 = 30;

pub const CLEAR: &str = "        ";

pub struct Display {
    pub stdout: Stdout,
    pub terminal_size: (u16, u16),
    pub board_x: Vec<(u16, u16)>,
    pub board_y: (u16, u16),
    pub prev_next: Option<TetrominoVariant>,
    pub prev_hold: Option<TetrominoVariant>,
}

fn calc_board_x() -> Vec<(u16, u16)> {
    let terminal_size = terminal::size().unwrap();

    let board_x_0 = terminal_size.0 / 2 - BOARD_DIMENSION.0 as u16 * 2 / 2;
    let board_x_1 = terminal_size.0 / 2 - BOARD_DIMENSION.0 as u16 + BOARD_DIMENSION.0 as u16 * 2 + 2;

    if *config::ENABLE_MULTIPLAYER {
        vec![
            (board_x_0 - BOARD_MP_OFFSET, board_x_1 - BOARD_MP_OFFSET),
            (board_x_0 + BOARD_MP_OFFSET, board_x_1 + BOARD_MP_OFFSET),
        ]
    } else {
        vec![
            (board_x_0, board_x_1),
        ]
    }
}

impl Display {
    pub fn new() -> Result<Self> {
        let stdout = stdout();

        let terminal_size = terminal::size().unwrap();

        let board_x = calc_board_x();
        let board_y = (
            0,
            BOARD_DIMENSION.1 as u16 + 2,
        );

        Ok(Display {
            stdout,
            terminal_size,
            board_x,
            board_y,
            prev_next: None,
            prev_hold: None,
        })
    }

    pub fn draw(&mut self) -> Result<()> {
        execute!(self.stdout, Clear(ClearType::All))?;

        self.terminal_size = terminal::size().unwrap();

        self.board_x = calc_board_x();

        self.prev_hold = None;
        self.prev_next = None;

        for board_x in self.board_x.iter() {
            for x in board_x.0..board_x.1 {
                for y in self.board_y.0..self.board_y.1 {
                    self.stdout
                        .queue(MoveTo(x, y))?
                        .queue(Print(
                            if x == board_x.0 && y == 0 {
                                "╔"
                            } else if x == board_x.0 && y == self.board_y.1 - 1 {
                                "╚"
                            } else if x == board_x.1 - 1 && y == self.board_y.0 {
                                "╗"
                            } else if x == board_x.1 - 1 && y == self.board_y.1 - 1 {
                                "╝"
                            } else if x == board_x.0 || x == board_x.1 - 1 {
                                "║"
                            } else if y == self.board_y.0 || y == self.board_y.1 - 1 {
                                "═"
                            } else if x % 2 != self.terminal_size.0 / 2 % 2 {
                                "."
                            } else {
                                " "
                            }
                        ))?;
                }
            }
            self.stdout
                .queue(MoveTo(board_x.0 + (board_x.1 - board_x.0) / 2 - 3, 0))?
                .queue(PrintStyledContent("TETRIS".bold()))?
                .queue(MoveTo(board_x.1 + 1, 2))?
                .queue(Print("NEXT:"))?
                .queue(MoveTo(board_x.0 - 9, 2))?
                .queue(Print("HOLD:"))?
                .queue(MoveTo(0, 0))?;
        }

        Ok(self.stdout.flush()?)
    }

    pub fn render(&mut self, game: &Game) -> Result<()> {
        self.render_board(game)?
            .render_hold(game)?
            .render_next(game)?
            .render_stats(game)?;

        Ok(self.stdout.flush()?)
    }

    fn tetromino_at_position(&self, tetromino: &Tetromino, pos: &Dimension, i: usize) -> bool {
        tetromino.geometry.shape.iter().any(|(x, y)| {
            self.board_y.1 as i32 - y - 2 == pos.1 && (
                self.board_x[i].0 as i32 + (x + 1) * 2 == pos.0 ||
                self.board_x[i].0 as i32 + (x + 1) * 2 == pos.0 + 1
            )
        })
    }

    fn render_board(&mut self, game: &Game) -> Result<&mut Self> {
        for (i, board_x) in self.board_x.iter().enumerate() {
            for x in board_x.0 + 1..board_x.1 - 1 {
                for y in self.board_y.0 + 1..self.board_y.1 - 1 {
                    let pos = &(x as i32, y as i32);

                    let mut content = StyledContent::new(ContentStyle::new(),
                        if x % 2 != self.terminal_size.0 / 2 % 2 {
                            "."
                        } else {
                            " "
                        }
                    );

                    if let Some(ghost) = &game.player[i].ghost {
                        if self.tetromino_at_position(ghost, pos, i) {
                            content = "░".with(game.player[i].falling.color);
                        }
                    }

                    if self.tetromino_at_position(&game.player[i].falling, pos, i) {
                        content = if game.player[i].locking {
                            "▓".with(game.player[i].falling.color)
                        } else {
                            " ".on(game.player[i].falling.color)
                        };
                    }

                    let j = (self.board_y.1 - 2 - y) as usize;
                    let k = ((x - board_x.0 - 1) / 2) as usize;

                    if let Some(color) = game.player[i].stack[j][k] {
                        content = if game.player[i].clearing.get(&j).is_some() {
                            "▓".with(Color::White)
                        } else {
                            " ".on(color)
                        }
                    }

                    self.stdout
                        .queue(MoveTo(x, y))?
                        .queue(PrintStyledContent(content))?;
                }
            }
        }
        Ok(self)
    }

    fn render_hold(&mut self, game: &Game) -> Result<&mut Self> {
        for (i, board_x) in self.board_x.iter().enumerate() {
            if let Some(holding) = &game.player[i].holding {
                if self.prev_hold == Some(holding.variant) {
                    return Ok(self)
                }
                self.prev_hold = Some(holding.variant);

                self.stdout
                    .queue(MoveTo(board_x.0 - 10, 4))?
                    .queue(Print(CLEAR))?
                    .queue(MoveTo(board_x.0 - 10, 5))?
                    .queue(Print(CLEAR))?;
                for position in holding.geometry.shape.iter().map(|&(x, y)| (x as u16, y as u16)) {
                    self.stdout
                        .queue(MoveTo((position.0 - 3) * 2 + board_x.0 - 10, self.board_y.1 - position.1 + 1))?
                        .queue(PrintStyledContent(" ".on(holding.color)))?
                        .queue(MoveTo((position.0 - 3) * 2 + board_x.0 - 9, self.board_y.1 - position.1 + 1))?
                        .queue(PrintStyledContent(" ".on(holding.color)))?;
                }
            }
        }
        Ok(self)
    }

    fn render_next(&mut self, game: &Game) -> Result<&mut Self> {
        for (i, board_x) in self.board_x.iter().enumerate() {
            if let Some(next) = &game.player[i].next.get(0) {
                if self.prev_next == Some(next.variant) {
                    return Ok(self)
                }
                self.prev_next = Some(next.variant);
            }
            for (i, tetromino) in game.player[i].next.iter().enumerate() {
                self.stdout
                    .queue(MoveTo(board_x.1 + 1, 4 + (i as u16 * 3)))?
                    .queue(Print(CLEAR))?
                    .queue(MoveTo(board_x.1 + 1, 5 + (i as u16 * 3)))?
                    .queue(Print(CLEAR))?;
                for position in tetromino.geometry.shape.iter().map(|&(x, y)| (x as u16, y as u16)) {
                    self.stdout
                        .queue(MoveTo((position.0 - 3) * 2 + board_x.1 + 2, self.board_y.1 - position.1 + 1 + (i as u16 * 3)))?
                        .queue(PrintStyledContent(" ".on(tetromino.color)))?
                        .queue(MoveTo((position.0 - 3) * 2 + board_x.1 + 1, self.board_y.1 - position.1 + 1 + (i as u16 * 3)))?
                        .queue(PrintStyledContent(" ".on(tetromino.color)))?;
                }
            }
        }
        Ok(self)
    }

    fn render_stats(&mut self, game: &Game) -> Result<&mut Self> {
        for (i, board_x) in self.board_x.iter().enumerate() {
            self.stdout
                .queue(MoveTo(board_x.1 + 1, 17))?
                .queue(Print(format!("SCORE: {}", game.player[i].score)))?
                .queue(MoveTo(board_x.1 + 1, 18))?
                .queue(Print(format!("LEVEL: {}", game.player[i].level)))?
                .queue(MoveTo(board_x.1 + 1, 19))?
                .queue(Print(format!("LINES: {}", game.player[i].lines)))?
                .queue(MoveTo(0, 0))?;
        }
        Ok(self)
    }

    pub fn render_debug_info(&mut self, debug_frame: u64) -> Result<&mut Self> {
        self.stdout
            .queue(MoveTo(0, 0))?
            .queue(Print(CLEAR))?
            .queue(MoveTo(0, 0))?
            .queue(Print(format!("{} fps", debug_frame)))?;

        Ok(self)
    }
}

