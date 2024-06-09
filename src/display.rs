use std::io::Stdout;
use::std::io::{stdout, Write};
use crossterm::{
    execute, QueueableCommand, Result,
    cursor::MoveTo,
    style::{ContentStyle, Print, PrintStyledContent, StyledContent, Stylize},
    terminal::{self, Clear, ClearType},
};

use crate::{game::Game, tetromino::Tetromino};

// use crate::debug_println;

pub const BOARD_DIMENSION: Dimension = (10, 20);

pub type Dimension = (i32, i32);

const BOARD_WIDTH: u16 = BOARD_DIMENSION.0 as u16 * 2 + 2;
const BOARD_HEIGHT: u16 = BOARD_DIMENSION.1 as u16 + 2;

pub struct Display {
    pub stdout: Stdout,
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub board_width_start: u16,
    pub board_width_end: u16,
}

impl Display {
    pub fn new() -> Result<Self> {
        let (terminal_width, terminal_height) = terminal::size().unwrap();

        let board_width_start = terminal_width / 2 - BOARD_WIDTH / 2;
        let board_width_end = board_width_start + BOARD_WIDTH;

        let stdout = stdout();

        Ok(Display { stdout, terminal_width, terminal_height, board_width_start, board_width_end })
    }

    pub fn draw(&mut self) -> Result<()> {
        execute!(self.stdout, Clear(ClearType::All))?;

        let (terminal_width, _terminal_height) = terminal::size().unwrap();

        let board_width_start = terminal_width / 2 - BOARD_WIDTH / 2;
        let board_width_end = board_width_start + BOARD_WIDTH;

        for x in board_width_start..board_width_end {
            for y in 0..BOARD_HEIGHT {
                self.stdout
                    .queue(MoveTo(x, y))?
                    .queue(Print(
                        if x == board_width_start && y == 0 {
                            "╔"
                        } else if x == board_width_start && y == BOARD_HEIGHT - 1 {
                            "╚"
                        } else if x == board_width_end - 1 && y == 0 {
                            "╗"
                        } else if x == board_width_end - 1 && y == BOARD_HEIGHT - 1 {
                            "╝"
                        } else if x == board_width_start || x == board_width_end - 1 {
                            "║"
                        } else if y == 0 || y == BOARD_HEIGHT - 1 {
                            "═"
                        } else if x % 2 != terminal_width / 2 % 2 {
                            "."
                        } else {
                            " "
                        }
                    ))?;
            }
        }

        self.stdout
            .queue(MoveTo(board_width_start + 8, 0))?
            .queue(PrintStyledContent("TETRIS".bold()))?
            .queue(MoveTo(board_width_end + 1, 2))?
            .queue(Print("NEXT:"))?
            .queue(MoveTo(board_width_start - 9, 2))?
            .queue(Print("HOLD:"))?
            .queue(MoveTo(0, 0))?;

        Ok(self.stdout.flush()?)
    }

    pub fn render(&mut self, game: &Game) -> Result<()> {
        self.render_board(game)?
            .render_hold(game)?
            .render_next(game)?
            .render_stats(game)?;

        Ok(self.stdout.flush()?)
    }

    fn render_board(&mut self, game: &Game) -> Result<&mut Self> {

        fn position_in_view(position: &Dimension, view: &Dimension, offset_x: i32) -> bool {
            BOARD_DIMENSION.1 - position.1 == view.1 &&
            (
                (position.0 + 1) * 2 + offset_x == view.0 ||
                (position.0 + 1) * 2 + offset_x - 1 == view.0
            )
        }

        fn tetromino_in_view(tetromino: &Tetromino, view: &Dimension, offset_x: i32) -> bool {
            tetromino.shape.iter().any(|position| position_in_view(position, view, offset_x))
        }

        for x in self.board_width_start + 1..self.board_width_end - 1 {
            for y in 1..BOARD_HEIGHT - 1 {
                self.stdout
                    .queue(MoveTo(x, y))?
                    .queue(PrintStyledContent((|| {

                        let view = &(x as i32, y as i32);
                        let offset_x = self.board_width_start as i32;

                        if tetromino_in_view(&game.falling, view, offset_x) {
                            return if game.locking {
                                "▓".with(game.falling.color)
                            } else {
                                 " ".on(game.falling.color)
                            }
                        }

                        if let Some(ghost) = &game.ghost {
                            if tetromino_in_view(ghost, view, offset_x) {
                                return "░".with(game.falling.color)
                            }
                        }

                        for (i, row) in game.stack.iter().enumerate() {
                            for (j, color) in row.iter().enumerate() {
                                if let Some(color) = color {
                                    if position_in_view(&(j as i32, i as i32), view, offset_x) {
                                        return " ".on(*color)
                                    }
                                }
                            }
                        }

                        StyledContent::new(ContentStyle::new(),
                            if x % 2 != self.terminal_width / 2 % 2 {
                                "."
                            } else {
                                " "
                            }
                        )
                    })()))?;
            }
        }

        Ok(self)
    }

    fn render_hold(&mut self, game: &Game) -> Result<&mut Self> {
        if let Some(holding) = &game.holding {
            self.stdout
                .queue(MoveTo(self.board_width_start - 9, 4))?
                .queue(Print("        "))?
                .queue(MoveTo(self.board_width_start - 9, 5))?
                .queue(Print("        "))?;
            for position in holding.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
                self.stdout
                    .queue(MoveTo((position.0 - 3) * 2 + self.board_width_start - 9, BOARD_HEIGHT - position.1 + 1))?
                    .queue(PrintStyledContent(" ".on(holding.color)))?
                    .queue(MoveTo((position.0 - 3) * 2 + self.board_width_start - 8, BOARD_HEIGHT - position.1 + 1))?
                    .queue(PrintStyledContent(" ".on(holding.color)))?;
            }
        }

        Ok(self)
    }

    fn render_next(&mut self, game: &Game) -> Result<&mut Self> {
        for (i, tetromino) in game.next.iter().enumerate() {
            self.stdout
                .queue(MoveTo(self.board_width_end + 1, 4 + (i as u16 * 3)))?
                .queue(Print("        "))?
                .queue(MoveTo(self.board_width_end + 1, 5 + (i as u16 * 3)))?
                .queue(Print("        "))?;
            for position in tetromino.shape.iter().map(|(x, y)| (*x as u16, *y as u16)) {
                self.stdout
                    .queue(MoveTo((position.0 - 3) * 2 + self.board_width_end + 2, BOARD_HEIGHT - position.1 + 1 + (i as u16 * 3)))?
                    .queue(PrintStyledContent(" ".on(tetromino.color)))?
                    .queue(MoveTo((position.0 - 3) * 2 + self.board_width_end + 1, BOARD_HEIGHT - position.1 + 1 + (i as u16 * 3)))?
                    .queue(PrintStyledContent(" ".on(tetromino.color)))?;
            }
        }

        Ok(self)
    }

    fn render_stats(&mut self, game: &Game) -> Result<&mut Self> {
        self.stdout
            .queue(MoveTo(self.board_width_end + 1, 17))?
            .queue(Print(format!("SCORE: {}", game.score)))?
            .queue(MoveTo(self.board_width_end + 1, 18))?
            .queue(Print(format!("LEVEL: {}", game.level)))?
            .queue(MoveTo(self.board_width_end + 1, 19))?
            .queue(Print(format!("LINES: {}", game.lines)))?
            .queue(MoveTo(0, 0))?;

        Ok(self)
    }

    pub fn render_debug_info(&mut self, debug_frame: u64) -> Result<&mut Self> {
        self.stdout
            .queue(MoveTo(0, 0))?
            .queue(Print(format!("{} fps", debug_frame)))?;

        Ok(self)
    }
}
